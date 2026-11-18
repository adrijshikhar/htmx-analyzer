mod support;
use htmx_analyzer::ExitKind;
use lsp_server::{Message, Notification, Request, RequestId};
use lsp_types::{
    DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, Position, TextDocumentContentChangeEvent, TextDocumentIdentifier,
    TextDocumentItem, Uri, VersionedTextDocumentIdentifier,
};
use std::str::FromStr;
use std::time::Duration;
use support::TestClient;

#[test]
fn burst_of_changes_publishes_once_with_final_version() {
    let uri = Uri::from_str("file:///burst.html").unwrap();
    let mut queued = Vec::new();

    // didOpen v1
    queued.push(Message::Notification(Notification::new(
        "textDocument/didOpen".to_string(),
        serde_json::to_value(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "html".to_string(),
                version: 1,
                text: "<div>0</div>".to_string(),
            },
        })
        .unwrap(),
    )));

    // 50 incremental didChange (v2..=51)
    for v in 2..=51 {
        queued.push(Message::Notification(Notification::new(
            "textDocument/didChange".to_string(),
            serde_json::to_value(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: v,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: format!("<div>{}</div>", v),
                }],
            })
            .unwrap(),
        )));
    }

    // hover request
    queued.push(Message::Request(Request::new(
        RequestId::from(100),
        "textDocument/hover".to_string(),
        serde_json::to_value(lsp_types::HoverParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 0,
                    character: 1,
                },
            },
            work_done_progress_params: Default::default(),
        })
        .unwrap(),
    )));

    let client = TestClient::start_with_queued(serde_json::json!({}), queued);

    // First message received must be the hover response!
    let msg1 = client.recv(Duration::from_secs(5)).expect("recv msg1");
    match msg1 {
        Message::Response(r) => {
            assert_eq!(r.id, RequestId::from(100));
        }
        other => panic!("Expected hover response, got {:?}", other),
    }

    // Second message received must be publishDiagnostics with version Some(51)
    let msg2 = client.recv(Duration::from_secs(5)).expect("recv msg2");
    match msg2 {
        Message::Notification(not) => {
            assert_eq!(not.method, "textDocument/publishDiagnostics");
            let params: lsp_types::PublishDiagnosticsParams =
                serde_json::from_value(not.params).unwrap();
            assert_eq!(params.uri, uri);
            assert_eq!(params.version, Some(51));
        }
        other => panic!("Expected publishDiagnostics notification, got {:?}", other),
    }

    // No further publishes should arrive
    let extra = client.recv(Duration::from_millis(200));
    assert!(extra.is_none(), "Unexpected extra message: {:?}", extra);

    client.notify("exit", serde_json::Value::Null);
    let _ = client.join();
}

#[test]
fn initialization_options_set_severity() {
    let client = TestClient::start(serde_json::json!({
        "diagnostics": {
            "severity": {
                "HTMX004": "warning"
            }
        }
    }));

    let uri = Uri::from_str("file:///init_sev.html").unwrap();
    client.notify(
        "textDocument/didOpen",
        serde_json::to_value(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "html".to_string(),
                version: 1,
                text: "<b hx-target=\"#ghost\"></b>".to_string(),
            },
        })
        .unwrap(),
    );

    let msg = client.recv(Duration::from_secs(5)).expect("recv publish");
    match msg {
        Message::Notification(not) => {
            assert_eq!(not.method, "textDocument/publishDiagnostics");
            let params: lsp_types::PublishDiagnosticsParams =
                serde_json::from_value(not.params).unwrap();
            assert_eq!(params.diagnostics.len(), 1);
            assert_eq!(
                params.diagnostics[0].severity,
                Some(DiagnosticSeverity::WARNING)
            );
        }
        other => panic!("Expected publishDiagnostics, got {:?}", other),
    }

    client.notify("exit", serde_json::Value::Null);
    let _ = client.join();
}

#[test]
fn did_change_configuration_revalidates_open_documents() {
    let client = TestClient::start(serde_json::json!({}));
    let uri = Uri::from_str("file:///config_change.html").unwrap();

    // didOpen with missing target
    client.notify(
        "textDocument/didOpen",
        serde_json::to_value(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "html".to_string(),
                version: 1,
                text: "<b hx-target=\"#ghost\"></b>".to_string(),
            },
        })
        .unwrap(),
    );

    // Default severity is HINT (4)
    let msg1 = client.recv(Duration::from_secs(5)).expect("recv publish 1");
    match msg1 {
        Message::Notification(not) => {
            assert_eq!(not.method, "textDocument/publishDiagnostics");
            let params: lsp_types::PublishDiagnosticsParams =
                serde_json::from_value(not.params).unwrap();
            assert_eq!(params.diagnostics.len(), 1);
            assert_eq!(
                params.diagnostics[0].severity,
                Some(DiagnosticSeverity::HINT)
            );
        }
        other => panic!("Expected publishDiagnostics, got {:?}", other),
    }

    // Send didChangeConfiguration turning HTMX004 off
    client.notify(
        "workspace/didChangeConfiguration",
        serde_json::json!({
            "settings": {
                "diagnostics": {
                    "severity": {
                        "HTMX004": "off"
                    }
                }
            }
        }),
    );

    // Next publish has no HTMX004
    let msg2 = client.recv(Duration::from_secs(5)).expect("recv publish 2");
    match msg2 {
        Message::Notification(not) => {
            assert_eq!(not.method, "textDocument/publishDiagnostics");
            let params: lsp_types::PublishDiagnosticsParams =
                serde_json::from_value(not.params).unwrap();
            assert!(params.diagnostics.is_empty());
        }
        other => panic!("Expected publishDiagnostics, got {:?}", other),
    }

    client.notify("exit", serde_json::Value::Null);
    let _ = client.join();
}

#[test]
fn oversize_document_logs_once() {
    let client = TestClient::start(serde_json::json!({}));
    let uri = Uri::from_str("file:///large.html").unwrap();

    // 5 MiB + 1 byte
    let large_text = "a".repeat(5 * 1024 * 1024 + 1);

    client.notify(
        "textDocument/didOpen",
        serde_json::to_value(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "html".to_string(),
                version: 1,
                text: large_text,
            },
        })
        .unwrap(),
    );

    // Expect window/logMessage, type 2, message contains "5 MiB"
    let mut got_log = false;
    let mut got_empty_publish = false;

    for _ in 0..2 {
        let msg = client.recv(Duration::from_secs(5)).expect("recv msg");
        match msg {
            Message::Notification(not) if not.method == "window/logMessage" => {
                let params: lsp_types::LogMessageParams =
                    serde_json::from_value(not.params).unwrap();
                assert_eq!(params.typ, lsp_types::MessageType::WARNING);
                assert!(params.message.contains("5 MiB"));
                got_log = true;
            }
            Message::Notification(not) if not.method == "textDocument/publishDiagnostics" => {
                let params: lsp_types::PublishDiagnosticsParams =
                    serde_json::from_value(not.params).unwrap();
                assert!(params.diagnostics.is_empty());
                got_empty_publish = true;
            }
            other => panic!("Unexpected message: {:?}", other),
        }
    }
    assert!(got_log, "Did not get window/logMessage");
    assert!(got_empty_publish, "Did not get empty publish");

    // A following didChange sends no second logMessage
    client.notify(
        "textDocument/didChange",
        serde_json::to_value(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "b".repeat(5 * 1024 * 1024 + 2),
            }],
        })
        .unwrap(),
    );

    // We should get empty publish diagnostics, but NO logMessage
    let msg3 = client.recv(Duration::from_secs(5)).expect("recv msg 3");
    match msg3 {
        Message::Notification(not) => {
            assert_eq!(not.method, "textDocument/publishDiagnostics");
            let params: lsp_types::PublishDiagnosticsParams =
                serde_json::from_value(not.params).unwrap();
            assert!(params.diagnostics.is_empty());
        }
        other => panic!("Expected publishDiagnostics, got {:?}", other),
    }

    let extra = client.recv(Duration::from_millis(200));
    assert!(extra.is_none(), "Unexpected extra message: {:?}", extra);

    client.notify("exit", serde_json::Value::Null);
    let _ = client.join();
}

#[test]
fn did_close_clears_diagnostics_immediately() {
    let client = TestClient::start(serde_json::json!({}));
    let uri = Uri::from_str("file:///close.html").unwrap();

    client.notify(
        "textDocument/didOpen",
        serde_json::to_value(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "html".to_string(),
                version: 1,
                text: "<button hx-ws=\"connect:/chat\">WS</button>".to_string(),
            },
        })
        .unwrap(),
    );

    let msg1 = client.recv(Duration::from_secs(5)).expect("recv publish");
    match msg1 {
        Message::Notification(not) => {
            assert_eq!(not.method, "textDocument/publishDiagnostics");
            let params: lsp_types::PublishDiagnosticsParams =
                serde_json::from_value(not.params).unwrap();
            assert_eq!(params.diagnostics.len(), 1);
        }
        other => panic!("Expected publishDiagnostics, got {:?}", other),
    }

    client.notify(
        "textDocument/didClose",
        serde_json::to_value(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        })
        .unwrap(),
    );

    let msg2 = client.recv(Duration::from_secs(5)).expect("recv clear");
    match msg2 {
        Message::Notification(not) => {
            assert_eq!(not.method, "textDocument/publishDiagnostics");
            let params: lsp_types::PublishDiagnosticsParams =
                serde_json::from_value(not.params).unwrap();
            assert!(params.diagnostics.is_empty());
            assert_eq!(params.version, None);
        }
        other => panic!("Expected publishDiagnostics, got {:?}", other),
    }

    client.notify("exit", serde_json::Value::Null);
    let _ = client.join();
}

#[test]
fn change_for_unknown_uri_is_ignored() {
    let mut client = TestClient::start(serde_json::json!({}));
    let uri = Uri::from_str("file:///unknown.html").unwrap();

    client.notify(
        "textDocument/didChange",
        serde_json::to_value(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "content".to_string(),
            }],
        })
        .unwrap(),
    );

    let resp = client.request(
        "textDocument/hover",
        serde_json::to_value(lsp_types::HoverParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line: 0,
                    character: 0,
                },
            },
            work_done_progress_params: Default::default(),
        })
        .unwrap(),
    );

    assert_eq!(resp.response_result.unwrap(), serde_json::Value::Null);

    // No publish received
    let extra = client.recv(Duration::from_millis(200));
    assert!(extra.is_none());

    client.notify("shutdown", serde_json::Value::Null);
    client.notify("exit", serde_json::Value::Null);
    let _ = client.join();
}

#[test]
fn empty_settings_keep_initialization_options() {
    let client = TestClient::start(serde_json::json!({
        "diagnostics": {
            "severity": {
                "HTMX004": "warning"
            }
        }
    }));

    client.notify(
        "workspace/didChangeConfiguration",
        serde_json::json!({ "settings": {} }),
    );

    let uri = Uri::from_str("file:///empty_settings.html").unwrap();
    client.notify(
        "textDocument/didOpen",
        serde_json::to_value(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "html".to_string(),
                version: 1,
                text: "<b hx-target=\"#ghost\"></b>".to_string(),
            },
        })
        .unwrap(),
    );

    let msg = client.recv(Duration::from_secs(5)).expect("recv publish");
    match msg {
        Message::Notification(not) => {
            let params: lsp_types::PublishDiagnosticsParams =
                serde_json::from_value(not.params).unwrap();
            assert_eq!(params.diagnostics.len(), 1);
            assert_eq!(
                params.diagnostics[0].severity,
                Some(DiagnosticSeverity::WARNING)
            );
        }
        other => panic!("Expected publishDiagnostics, got {:?}", other),
    }

    client.notify("exit", serde_json::Value::Null);
    let _ = client.join();
}

#[test]
fn notifications_after_shutdown_are_dropped() {
    let mut client = TestClient::start(serde_json::json!({}));
    let resp = client.request("shutdown", serde_json::Value::Null);
    assert!(resp.response_result.is_ok());

    let uri = Uri::from_str("file:///after_shutdown.html").unwrap();
    client.notify(
        "textDocument/didOpen",
        serde_json::to_value(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "html".to_string(),
                version: 1,
                text: "<button hx-ws=\"connect:/chat\">WS</button>".to_string(),
            },
        })
        .unwrap(),
    );

    let extra = client.recv(Duration::from_millis(200));
    assert!(
        extra.is_none(),
        "Unexpected message after shutdown: {:?}",
        extra
    );

    client.notify("exit", serde_json::Value::Null);
    let exit_kind = client.join();
    assert_eq!(exit_kind, ExitKind::Clean);
}
