use crate::handler;
use htmx_service::{Config, HtmxLanguageService};
use htmx_vfs::DocumentStore;
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    notification::{
        DidChangeConfiguration, DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
        LogMessage, Notification as _, PublishDiagnostics,
    },
    request::{CodeActionRequest, Completion, HoverRequest, Request as _},
    CodeActionOptions, CodeActionProviderCapability, CompletionOptions, HoverProviderCapability,
    LogMessageParams, MessageType, PublishDiagnosticsParams, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
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
    #[cfg(test)]
    pub panic_on_next_did_change: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

fn mark_dirty(dirty: &mut Vec<Uri>, uri: Uri) {
    if !dirty.contains(&uri) {
        dirty.push(uri);
    }
}

impl Server {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            service: HtmxLanguageService::new(),
            doc_store: DocumentStore::new(),
            #[cfg(test)]
            panic_on_next_did_change: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
                false,
            )),
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
        let init_params_val = self.connection.initialize(server_capabilities)?;
        let init_options = init_params_val
            .get("initializationOptions")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let config = Config::from_json(&init_options);
        self.service.set_config(config);

        info!("htmx-analyzer initialized successfully. Listening on stdio.");

        let (pump_tx, pump_rx) = crossbeam_channel::unbounded();
        while let Ok(msg) = self.connection.receiver.try_recv() {
            let _ = pump_tx.send(msg);
        }
        let conn_rx = self.connection.receiver.clone();
        std::thread::spawn(move || {
            while let Ok(msg) = conn_rx.recv() {
                if pump_tx.send(msg).is_err() {
                    break;
                }
            }
        });

        let mut is_shutdown = false;
        let mut dirty: Vec<Uri> = Vec::new();
        let mut oversized_notified: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        while let Ok(first) = pump_rx.recv() {
            let mut batch = std::collections::VecDeque::new();
            batch.push_back(first);
            while let Ok(msg) = pump_rx.try_recv() {
                batch.push_back(msg);
            }

            while let Some(msg) = batch.pop_front() {
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

                        let method = not.method.clone();
                        match method.as_str() {
                            DidOpenTextDocument::METHOD => {
                                let params: lsp_types::DidOpenTextDocumentParams =
                                    match serde_json::from_value(not.params) {
                                        Ok(p) => p,
                                        Err(err) => {
                                            error!("Failed to parse didOpen params: {:?}", err);
                                            continue;
                                        }
                                    };
                                let uri = params.text_document.uri.clone();
                                let res =
                                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                        handler::handle_did_open(&mut self.doc_store, params);
                                    }));
                                if res.is_err() {
                                    error!("Panic in didOpen for {:?}", uri);
                                    continue;
                                }

                                if let Some(doc) = self.doc_store.get(&uri) {
                                    let len = doc.text().len();
                                    if len > htmx_parser::MAX_FILE_SIZE {
                                        if oversized_notified.insert(uri.to_string()) {
                                            let msg = format!(
                                                "htmx-analyzer: {} is {} bytes, above the 5 MiB analysis limit; diagnostics, hover and completion are off for this file.",
                                                uri.as_str(), len
                                            );
                                            self.send_log_message(MessageType::WARNING, msg);
                                        }
                                    } else {
                                        oversized_notified.remove(uri.as_str());
                                    }
                                }

                                mark_dirty(&mut dirty, uri);
                            }
                            DidChangeTextDocument::METHOD => {
                                let params: lsp_types::DidChangeTextDocumentParams =
                                    match serde_json::from_value(not.params) {
                                        Ok(p) => p,
                                        Err(err) => {
                                            error!("Failed to parse didChange params: {:?}", err);
                                            continue;
                                        }
                                    };
                                let uri = params.text_document.uri.clone();

                                if self.doc_store.get(&uri).is_none() {
                                    continue;
                                }

                                let res =
                                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                        #[cfg(test)]
                                        if self
                                            .panic_on_next_did_change
                                            .swap(false, std::sync::atomic::Ordering::SeqCst)
                                        {
                                            panic!("intentional didChange panic");
                                        }
                                        handler::handle_did_change(&mut self.doc_store, params);
                                    }));

                                if res.is_err() {
                                    error!("Panic in didChange for {:?}", uri);
                                    self.doc_store.close(&uri);
                                    dirty.retain(|u| u != &uri);
                                    oversized_notified.remove(uri.as_str());
                                    self.publish_diagnostics(PublishDiagnosticsParams {
                                        uri,
                                        diagnostics: vec![],
                                        version: None,
                                    });
                                    continue;
                                }

                                if let Some(doc) = self.doc_store.get(&uri) {
                                    let len = doc.text().len();
                                    if len > htmx_parser::MAX_FILE_SIZE {
                                        if oversized_notified.insert(uri.to_string()) {
                                            let msg = format!(
                                                "htmx-analyzer: {} is {} bytes, above the 5 MiB analysis limit; diagnostics, hover and completion are off for this file.",
                                                uri.as_str(), len
                                            );
                                            self.send_log_message(MessageType::WARNING, msg);
                                        }
                                    } else {
                                        oversized_notified.remove(uri.as_str());
                                    }
                                }

                                mark_dirty(&mut dirty, uri);
                            }
                            DidCloseTextDocument::METHOD => {
                                let params: lsp_types::DidCloseTextDocumentParams =
                                    match serde_json::from_value(not.params) {
                                        Ok(p) => p,
                                        Err(err) => {
                                            error!("Failed to parse didClose params: {:?}", err);
                                            continue;
                                        }
                                    };
                                let uri = params.text_document.uri.clone();
                                let _ =
                                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                        handler::handle_did_close(&mut self.doc_store, params);
                                    }));
                                dirty.retain(|u| u != &uri);
                                oversized_notified.remove(uri.as_str());
                                self.publish_diagnostics(PublishDiagnosticsParams {
                                    uri,
                                    diagnostics: vec![],
                                    version: None,
                                });
                            }
                            DidChangeConfiguration::METHOD => {
                                let params: lsp_types::DidChangeConfigurationParams =
                                    match serde_json::from_value(not.params) {
                                        Ok(p) => p,
                                        Err(err) => {
                                            error!(
                                                "Failed to parse didChangeConfiguration params: {:?}",
                                                err
                                            );
                                            continue;
                                        }
                                    };
                                let new_config = Config::default()
                                    .merged(&init_options)
                                    .merged(&params.settings);
                                self.service.set_config(new_config);

                                for uri in self.doc_store.uris() {
                                    mark_dirty(&mut dirty, uri.clone());
                                }
                            }
                            "$/cancelRequest" => {
                                debug!("Received cancel request notification");
                            }
                            _ => {
                                trace!("Ignoring unhandled notification: {}", method);
                            }
                        }
                    }
                    Message::Response(_resp) => {
                        trace!("Received unhandled response");
                    }
                }

                while let Ok(more) = pump_rx.try_recv() {
                    batch.push_back(more);
                }
            }

            if !is_shutdown {
                for uri in &dirty {
                    if let Some(doc) = self.doc_store.get(uri) {
                        let len = doc.text().len();
                        let diagnostics = if len > htmx_parser::MAX_FILE_SIZE {
                            Vec::new()
                        } else {
                            let res =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    self.service.validate(doc)
                                }));
                            match res {
                                Ok(diags) => diags,
                                Err(_) => {
                                    error!("Panic while validating {:?}", uri);
                                    Vec::new()
                                }
                            }
                        };
                        self.publish_diagnostics(PublishDiagnosticsParams {
                            uri: uri.clone(),
                            diagnostics,
                            version: Some(doc.version),
                        });
                    }
                }
                dirty.clear();
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

    fn send_response(&self, resp: Response) {
        if let Err(err) = self.connection.sender.send(Message::Response(resp)) {
            error!("Failed to send response: {:?}", err);
        }
    }

    fn send_log_message(&self, typ: MessageType, message: String) {
        let params = LogMessageParams { typ, message };
        let not = Notification::new(LogMessage::METHOD.to_string(), params);
        if let Err(err) = self.connection.sender.send(Message::Notification(not)) {
            error!("Failed to send logMessage notification: {:?}", err);
        }
    }

    fn publish_diagnostics(&self, params: PublishDiagnosticsParams) {
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
    use lsp_types::{
        Position, TextDocumentContentChangeEvent, TextDocumentItem, VersionedTextDocumentIdentifier,
    };
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

    #[test]
    fn panic_in_did_change_drops_document() {
        let (server_conn, client_conn) = Connection::memory();
        let server = Server::new(server_conn);
        let panic_flag = server.panic_on_next_did_change.clone();
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
        let uri = lsp_types::Uri::from_str("file:///panic_doc.html").unwrap();
        let doc_item = TextDocumentItem {
            uri: uri.clone(),
            language_id: "html".to_string(),
            version: 1,
            text: "<button hx-ws=\"connect:/chat\">WS</button>".to_string(),
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

        // Drain the initial publish diagnostics
        let pub_diag = loop {
            match client_conn
                .receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("recv pub_diag")
            {
                Message::Notification(n) if n.method == "textDocument/publishDiagnostics" => {
                    break n
                }
                _ => continue,
            }
        };
        let params: PublishDiagnosticsParams = serde_json::from_value(pub_diag.params).unwrap();
        assert_eq!(params.diagnostics.len(), 1);

        // Arm panic for next didChange
        panic_flag.store(true, std::sync::atomic::Ordering::SeqCst);

        // Send didChange
        client_conn
            .sender
            .send(Message::Notification(Notification::new(
                "textDocument/didChange".to_string(),
                serde_json::to_value(lsp_types::DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier {
                        uri: uri.clone(),
                        version: 2,
                    },
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: None,
                        range_length: None,
                        text: "panic now".to_string(),
                    }],
                })
                .unwrap(),
            )))
            .unwrap();

        // Expect empty publish diagnostics for uri
        let pub_diag_empty = loop {
            match client_conn
                .receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("recv empty pub_diag")
            {
                Message::Notification(n) if n.method == "textDocument/publishDiagnostics" => {
                    break n
                }
                _ => continue,
            }
        };
        let params: PublishDiagnosticsParams =
            serde_json::from_value(pub_diag_empty.params).unwrap();
        assert!(params.diagnostics.is_empty());

        // A following hover on that document returns null
        let hover_req = Request::new(
            RequestId::from(2),
            "textDocument/hover".to_string(),
            serde_json::to_value(lsp_types::HoverParams {
                text_document_position_params: lsp_types::TextDocumentPositionParams {
                    text_document: lsp_types::TextDocumentIdentifier { uri },
                    position: Position {
                        line: 0,
                        character: 0,
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
                Message::Response(r) if r.id == RequestId::from(2) => break r,
                _ => continue,
            }
        };
        assert_eq!(hover_resp.response_result.unwrap(), serde_json::Value::Null);

        // Server still alive! Clean exit
        client_conn
            .sender
            .send(Message::Request(Request::new(
                RequestId::from(3),
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
