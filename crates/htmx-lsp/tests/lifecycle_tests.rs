mod support;
use htmx_analyzer::ExitKind;
use support::TestClient;

#[test]
fn shutdown_then_request_gets_invalid_request_then_exit_clean() {
    let mut client = TestClient::start(serde_json::json!({}));
    let resp = client.request("shutdown", serde_json::Value::Null);
    assert_eq!(resp.response_result.unwrap(), serde_json::Value::Null);

    let hover_resp = client.request(
        "textDocument/hover",
        serde_json::json!({
            "textDocument": { "uri": "file:///test.html" },
            "position": { "line": 0, "character": 0 }
        }),
    );
    assert_eq!(
        hover_resp.response_result.unwrap_err().code,
        lsp_server::ErrorCode::InvalidRequest as i32
    );

    client.notify("exit", serde_json::Value::Null);
    let exit_kind = client.join();
    assert_eq!(exit_kind, ExitKind::Clean);
}

#[test]
fn exit_without_shutdown_returns_no_shutdown() {
    let client = TestClient::start(serde_json::json!({}));
    client.notify("exit", serde_json::Value::Null);
    let exit_kind = client.join();
    assert_eq!(exit_kind, ExitKind::NoShutdown);
}
