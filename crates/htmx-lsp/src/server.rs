use crate::handler;
use htmx_service::HtmxLanguageService;
use htmx_vfs::DocumentStore;
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
        PublishDiagnostics,
    },
    request::{CodeActionRequest, Completion, HoverRequest, Request as _},
    CodeActionOptions, CodeActionProviderCapability, CompletionOptions, HoverProviderCapability,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
};
use std::error::Error;
use tracing::{debug, error, info, trace, warn};

/// LSP exit kind returned when the server terminates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
    Clean,
    NoShutdown,
}

pub struct Server {
    connection: Connection,
    service: HtmxLanguageService,
    doc_store: DocumentStore,
}

impl Server {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            service: HtmxLanguageService::new(),
            doc_store: DocumentStore::new(),
        }
    }

    pub fn run(mut self) -> Result<ExitKind, Box<dyn Error + Send + Sync>> {
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
        let mut is_shutdown = false;

        while let Ok(msg) = receiver.recv() {
            match msg {
                Message::Request(req) => {
                    if req.method == "shutdown" {
                        info!("Received shutdown request; transitioning to shutdown state.");
                        self.send_response(Response::new_ok(req.id, serde_json::Value::Null));
                        is_shutdown = true;
                        continue;
                    }
                    if is_shutdown {
                        self.send_response(Response::new_err(
                            req.id,
                            lsp_server::ErrorCode::InvalidRequest as i32,
                            "Server is shutting down".to_string(),
                        ));
                        continue;
                    }

                    let id = req.id.clone();
                    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        self.handle_request(req);
                    }));
                    if res.is_err() {
                        error!("Panic while handling request {}", id);
                        self.send_response(Response::new_err(
                            id,
                            lsp_server::ErrorCode::InternalError as i32,
                            "Internal server error".to_string(),
                        ));
                    }
                }
                Message::Notification(not) => {
                    if not.method == "exit" {
                        info!("Received exit notification; exiting process.");
                        if is_shutdown {
                            return Ok(ExitKind::Clean);
                        } else {
                            return Ok(ExitKind::NoShutdown);
                        }
                    }
                    if is_shutdown {
                        continue;
                    }

                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        self.handle_notification(not);
                    }));
                }
                Message::Response(_resp) => {
                    trace!("Received unhandled response");
                }
            }
        }

        Ok(if is_shutdown {
            ExitKind::Clean
        } else {
            ExitKind::NoShutdown
        })
    }

    fn handle_request(&mut self, req: Request) {
        let id = req.id.clone();
        let method = req.method.as_str();

        #[cfg(test)]
        if method == "htmx-analyzer/__testPanic" {
            panic!("intentional test panic");
        }

        match method {
            Completion::METHOD => {
                let params = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(err) => {
                        error!("Failed to parse completion params: {:?}", err);
                        self.send_response(Response::new_err(
                            id,
                            lsp_server::ErrorCode::InvalidParams as i32,
                            format!("Invalid completion params: {}", err),
                        ));
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
                        self.send_response(Response::new_err(
                            id,
                            lsp_server::ErrorCode::InvalidParams as i32,
                            format!("Invalid hover params: {}", err),
                        ));
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
                        self.send_response(Response::new_err(
                            id,
                            lsp_server::ErrorCode::InvalidParams as i32,
                            format!("Invalid code action params: {}", err),
                        ));
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
                debug!("Received cancel request notification");
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

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_server::{Message, Notification, Request, RequestId};
    use lsp_types::{Position, TextDocumentItem};
    use std::str::FromStr;
    use std::time::Duration;

    #[test]
    fn request_panic_returns_internal_error_and_server_continues() {
        let (server_conn, client_conn) = Connection::memory();
        let server = Server::new(server_conn);
        let handle = std::thread::spawn(move || server.run().expect("server run failed"));

        // Initialize handshake
        let init_req = Request::new(
            RequestId::from(1),
            "initialize".to_string(),
            serde_json::json!({ "capabilities": {} }),
        );
        client_conn.sender.send(Message::Request(init_req)).unwrap();
        let _init_resp = client_conn
            .receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("init resp");
        client_conn
            .sender
            .send(Message::Notification(Notification::new(
                "initialized".to_string(),
                serde_json::json!({}),
            )))
            .unwrap();

        // Open a document
        let doc_item = TextDocumentItem {
            uri: lsp_types::Uri::from_str("file:///test.html").unwrap(),
            language_id: "html".to_string(),
            version: 1,
            text: "<button hx-get=\"/api\">Click</button>".to_string(),
        };
        client_conn
            .sender
            .send(Message::Notification(Notification::new(
                "textDocument/didOpen".to_string(),
                serde_json::to_value(lsp_types::DidOpenTextDocumentParams {
                    text_document: doc_item,
                })
                .unwrap(),
            )))
            .unwrap();

        // Send panic request
        let panic_req = Request::new(
            RequestId::from(2),
            "htmx-analyzer/__testPanic".to_string(),
            serde_json::Value::Null,
        );
        client_conn
            .sender
            .send(Message::Request(panic_req))
            .unwrap();

        // Expect -32603 response
        let resp = loop {
            match client_conn
                .receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("recv panic resp")
            {
                Message::Response(r) if r.id == RequestId::from(2) => break r,
                _ => continue,
            }
        };
        assert_eq!(
            resp.response_result.unwrap_err().code,
            lsp_server::ErrorCode::InternalError as i32
        );

        // A following hover on an open document → Ok
        let hover_req = Request::new(
            RequestId::from(3),
            "textDocument/hover".to_string(),
            serde_json::to_value(lsp_types::HoverParams {
                text_document_position_params: lsp_types::TextDocumentPositionParams {
                    text_document: lsp_types::TextDocumentIdentifier {
                        uri: lsp_types::Uri::from_str("file:///test.html").unwrap(),
                    },
                    position: Position {
                        line: 0,
                        character: 10,
                    },
                },
                work_done_progress_params: Default::default(),
            })
            .unwrap(),
        );
        client_conn
            .sender
            .send(Message::Request(hover_req))
            .unwrap();

        let hover_resp = loop {
            match client_conn
                .receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("recv hover resp")
            {
                Message::Response(r) if r.id == RequestId::from(3) => break r,
                _ => continue,
            }
        };
        assert!(hover_resp.response_result.is_ok());

        // Clean exit
        client_conn
            .sender
            .send(Message::Request(Request::new(
                RequestId::from(4),
                "shutdown".to_string(),
                serde_json::Value::Null,
            )))
            .unwrap();
        client_conn
            .sender
            .send(Message::Notification(Notification::new(
                "exit".to_string(),
                serde_json::Value::Null,
            )))
            .unwrap();

        let exit_kind = handle.join().expect("join failed");
        assert_eq!(exit_kind, ExitKind::Clean);
    }
}
