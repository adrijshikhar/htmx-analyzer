use crate::handler;
use htmx_service::HtmxLanguageService;
use htmx_vfs::DocumentStore;
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
        PublishDiagnostics,
    },
    request::{CodeActionRequest, Completion, HoverRequest, Request as _},
    CodeActionOptions, CodeActionProviderCapability, CompletionOptions, HoverProviderCapability,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
};
use std::collections::HashSet;
use std::error::Error;
use tracing::{error, info, trace, warn};

pub struct Server {
    connection: Connection,
    service: HtmxLanguageService,
    doc_store: DocumentStore,
    cancelled_requests: HashSet<RequestId>,
}

impl Server {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            service: HtmxLanguageService::new(),
            doc_store: DocumentStore::new(),
            cancelled_requests: HashSet::new(),
        }
    }

    pub fn run(mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(vec![
                    "-".to_string(),
                    "\"".to_string(),
                    " ".to_string(),
                    ":".to_string(),
                    "[".to_string(),
                ]),
                all_commit_characters: None,
                work_done_progress_options: Default::default(),
                completion_item: None,
            }),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
                code_action_kinds: Some(vec![lsp_types::CodeActionKind::QUICKFIX]),
                work_done_progress_options: Default::default(),
                resolve_provider: Some(false),
            })),
            ..Default::default()
        };

        let server_capabilities = serde_json::to_value(&capabilities)?;
        let _init_params = self.connection.initialize(server_capabilities)?;

        info!("htmx-analyzer initialized successfully. Listening on stdio.");

        let receiver = self.connection.receiver.clone();
        while let Ok(msg) = receiver.recv() {
            match msg {
                Message::Request(req) => {
                    if self.connection.handle_shutdown(&req)? {
                        info!("Received shutdown request, exiting loop.");
                        return Ok(());
                    }
                    self.handle_request(req);
                }
                Message::Notification(not) => {
                    self.handle_notification(not);
                }
                Message::Response(_resp) => {
                    trace!("Received unhandled response");
                }
            }
        }

        Ok(())
    }

    fn handle_request(&mut self, req: Request) {
        if self.cancelled_requests.remove(&req.id) {
            info!("Skipping cancelled request: {:?}", req.id);
            return;
        }

        let id = req.id.clone();
        let method = req.method.as_str();

        match method {
            Completion::METHOD => {
                let params = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(err) => {
                        error!("Failed to parse completion params: {:?}", err);
                        return;
                    }
                };
                let resp = handler::handle_completion(&self.service, &self.doc_store, params);
                let result = serde_json::to_value(resp).unwrap_or(serde_json::Value::Null);
                self.send_response(Response::new_ok(id, result));
            }
            HoverRequest::METHOD => {
                let params = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(err) => {
                        error!("Failed to parse hover params: {:?}", err);
                        return;
                    }
                };
                let resp = handler::handle_hover(&self.service, &self.doc_store, params);
                let result = serde_json::to_value(resp).unwrap_or(serde_json::Value::Null);
                self.send_response(Response::new_ok(id, result));
            }
            CodeActionRequest::METHOD => {
                let params = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(err) => {
                        error!("Failed to parse code action params: {:?}", err);
                        return;
                    }
                };
                let resp = handler::handle_code_action(&self.service, &self.doc_store, params);
                let result = serde_json::to_value(resp).unwrap_or(serde_json::Value::Null);
                self.send_response(Response::new_ok(id, result));
            }
            _ => {
                warn!("Received unhandled request method: {}", method);
                let err = lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::MethodNotFound as i32,
                    message: format!("Method not found: {}", method),
                    data: None,
                };
                self.send_response(Response::new_err(id, err.code, err.message));
            }
        }
    }

    fn handle_notification(&mut self, not: Notification) {
        let method = not.method.as_str();

        match method {
            DidOpenTextDocument::METHOD => {
                let params = match serde_json::from_value(not.params) {
                    Ok(p) => p,
                    Err(err) => {
                        error!("Failed to parse didOpen params: {:?}", err);
                        return;
                    }
                };
                let publish_params =
                    handler::handle_did_open(&self.service, &mut self.doc_store, params);
                self.publish_diagnostics(publish_params);
            }
            DidChangeTextDocument::METHOD => {
                let params = match serde_json::from_value(not.params) {
                    Ok(p) => p,
                    Err(err) => {
                        error!("Failed to parse didChange params: {:?}", err);
                        return;
                    }
                };
                if let Some(publish_params) =
                    handler::handle_did_change(&self.service, &mut self.doc_store, params)
                {
                    self.publish_diagnostics(publish_params);
                }
            }
            DidCloseTextDocument::METHOD => {
                let params = match serde_json::from_value(not.params) {
                    Ok(p) => p,
                    Err(err) => {
                        error!("Failed to parse didClose params: {:?}", err);
                        return;
                    }
                };
                let publish_params = handler::handle_did_close(&mut self.doc_store, params);
                self.publish_diagnostics(publish_params);
            }
            "$/cancelRequest" => {
                #[derive(serde::Deserialize)]
                struct CancelParams {
                    id: RequestId,
                }
                if let Ok(p) = serde_json::from_value::<CancelParams>(not.params) {
                    info!("Received cancel request for id: {:?}", p.id);
                    self.cancelled_requests.insert(p.id);
                }
            }
            _ => {
                trace!("Ignoring unhandled notification: {}", method);
            }
        }
    }

    fn send_response(&self, resp: Response) {
        if let Err(err) = self.connection.sender.send(Message::Response(resp)) {
            error!("Failed to send response: {:?}", err);
        }
    }

    fn publish_diagnostics(&self, params: lsp_types::PublishDiagnosticsParams) {
        let not = Notification::new(PublishDiagnostics::METHOD.to_string(), params);
        if let Err(err) = self.connection.sender.send(Message::Notification(not)) {
            error!("Failed to send publishDiagnostics notification: {:?}", err);
        }
    }
}
