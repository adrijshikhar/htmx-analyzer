use crate::handler;
use htmx_service::{Config, HtmxLanguageService, WorkspaceIdIndex};
use htmx_vfs::DocumentStore;
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    notification::{
        DidChangeConfiguration, DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
        LogMessage, Notification as _, PublishDiagnostics,
    },
    request::{
        CodeActionRequest, Completion, GotoDefinition, HoverRequest, References, Request as _,
    },
    CodeActionOptions, CodeActionProviderCapability, CompletionOptions,
    HoverProviderCapability, LogMessageParams, MessageType, OneOf, PublishDiagnosticsParams,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};
use std::error::Error;
use std::str::FromStr;
use std::sync::Arc;
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
    workspace_index: Arc<parking_lot::RwLock<WorkspaceIdIndex>>,
    link_support: bool,
    #[cfg(test)]
    pub panic_on_next_did_change: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

fn mark_dirty(dirty: &mut Vec<Uri>, uri: Uri) {
    if !dirty.contains(&uri) {
        dirty.push(uri);
    }
}

fn is_template_file(path: &std::path::Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "html"
            | "htm"
            | "jinja"
            | "jinja2"
            | "j2"
            | "templ"
            | "blade"
            | "php"
            | "erb"
            | "hbs"
            | "liquid"
            | "twig"
    )
}

fn is_ignored_dir(name: &str) -> bool {
    matches!(
        name,
        ".git" | "node_modules" | "target" | "dist" | "build" | "vendor" | ".idea" | ".vscode"
    )
}

fn crawl_workspace(
    roots: &[std::path::PathBuf],
    index: &Arc<parking_lot::RwLock<WorkspaceIdIndex>>,
) {
    let mut staging = WorkspaceIdIndex::new();
    let mut count = 0usize;

    for root in roots {
        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    !is_ignored_dir(e.file_name().to_str().unwrap_or(""))
                } else {
                    true
                }
            })
            .filter_map(Result::ok)
        {
            if count >= 10_000 {
                warn!("Workspace crawler reached 10,000 files limit");
                break;
            }
            if !entry.file_type().is_file() || !is_template_file(entry.path()) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            if meta.len() > htmx_parser::MAX_FILE_SIZE as u64 {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            let uri_str = format!("file://{}", entry.path().to_string_lossy());
            let Ok(uri) = Uri::from_str(&uri_str) else {
                continue;
            };
            staging.update_file(&uri, &content);
            count += 1;
        }
    }

    info!("Workspace crawler indexed {} template files", count);
    index.write().merge(staging);
}

impl Server {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            service: HtmxLanguageService::new(),
            doc_store: DocumentStore::new(),
            workspace_index: Arc::new(parking_lot::RwLock::new(WorkspaceIdIndex::new())),
            link_support: false,
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
            definition_provider: Some(OneOf::Left(true)),
            references_provider: Some(OneOf::Left(true)),
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

        let link_support = init_params_val
            .get("capabilities")
            .and_then(|c| c.get("textDocument"))
            .and_then(|td| td.get("definition"))
            .and_then(|d| d.get("linkSupport"))
            .and_then(|ls| ls.as_bool())
            .unwrap_or(false);
        self.link_support = link_support;

        let mut root_paths = Vec::new();
        if let Some(folders) = init_params_val.get("workspaceFolders").and_then(|f| f.as_array()) {
            for folder in folders {
                if let Some(uri_str) = folder.get("uri").and_then(|u| u.as_str()) {
                    let path_str = uri_str.trim_start_matches("file://");
                    root_paths.push(std::path::PathBuf::from(path_str));
                }
            }
        } else if let Some(root_uri_str) = init_params_val.get("rootUri").and_then(|u| u.as_str()) {
            let path_str = root_uri_str.trim_start_matches("file://");
            root_paths.push(std::path::PathBuf::from(path_str));
        } else if let Some(root_path_str) = init_params_val.get("rootPath").and_then(|u| u.as_str()) {
            root_paths.push(std::path::PathBuf::from(root_path_str));
        }

        if !root_paths.is_empty() {
            let index_clone = self.workspace_index.clone();
            std::thread::spawn(move || {
                crawl_workspace(&root_paths, &index_clone);
            });
        }

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
                                let text = params.text_document.text.clone();
                                let res =
                                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                        handler::handle_did_open(&mut self.doc_store, params);
                                    }));
                                if res.is_err() {
                                    error!("Panic in didOpen for {:?}", uri);
                                    continue;
                                }

                                self.workspace_index.write().update_file(&uri, &text);

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
                                    self.workspace_index.write().remove_file(&uri);
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
                                    let text = doc.text();
                                    self.workspace_index.write().update_file(&uri, &text);
                                    let len = text.len();
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
                                let file_path =
                                    std::path::PathBuf::from(uri.as_str().trim_start_matches("file://"));
                                if file_path.exists() {
                                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                                        self.workspace_index.write().update_file(&uri, &content);
                                    } else {
                                        self.workspace_index.write().remove_file(&uri);
                                    }
                                } else {
                                    self.workspace_index.write().remove_file(&uri);
                                }
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
                            let index_guard = self.workspace_index.read();
                            let res =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    self.service.validate_with_index(doc, Some(&*index_guard))
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
            GotoDefinition::METHOD => {
                let params = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(err) => {
                        error!("Failed to parse goto definition params: {:?}", err);
                        self.send_response(Response::new_err(
                            id,
                            lsp_server::ErrorCode::InvalidParams as i32,
                            format!("Invalid goto definition params: {}", err),
                        ));
                        return;
                    }
                };
                let index_guard = self.workspace_index.read();
                let resp = handler::handle_definition(
                    &self.service,
                    &self.doc_store,
                    &index_guard,
                    self.link_support,
                    params,
                );
                let result = serde_json::to_value(resp).unwrap_or(serde_json::Value::Null);
                self.send_response(Response::new_ok(id, result));
            }
            References::METHOD => {
                let params = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(err) => {
                        error!("Failed to parse references params: {:?}", err);
                        self.send_response(Response::new_err(
                            id,
                            lsp_server::ErrorCode::InvalidParams as i32,
                            format!("Invalid references params: {}", err),
                        ));
                        return;
                    }
                };
                let index_guard = self.workspace_index.read();
                let resp = handler::handle_references(
                    &self.service,
                    &self.doc_store,
                    &index_guard,
                    params,
                );
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
