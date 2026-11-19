mod support;
use lsp_types::{
    DidOpenTextDocumentParams, GotoDefinitionParams, Position, ReferenceContext, ReferenceParams,
    TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams, Uri,
};
use std::str::FromStr;
use support::TestClient;

#[test]
fn test_lsp_definition_and_references_e2e() {
    let mut client = TestClient::start(serde_json::json!({}));
    let uri = Uri::from_str("file:///test_navigation.html").unwrap();
    let text = r##"<div id="target-div">Content</div>
<button hx-target="#target-div">Click</button>"##;

    // 1. Open document
    client.notify(
        "textDocument/didOpen",
        serde_json::to_value(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "html".to_string(),
                version: 1,
                text: text.to_string(),
            },
        })
        .unwrap(),
    );

    // 2. Request Go to Definition on #target-div (line 1, col 20)
    let def_resp = client.request(
        "textDocument/definition",
        serde_json::to_value(GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(1, 20),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .unwrap(),
    );

    let def_val = def_resp.response_result.expect("definition should succeed");
    let result: Option<lsp_types::GotoDefinitionResponse> =
        serde_json::from_value(def_val).unwrap();
    assert!(result.is_some(), "Expected definition to be found");

    // 3. Request Find References on <div id="target-div"> (line 0, col 12)
    let ref_resp = client.request(
        "textDocument/references",
        serde_json::to_value(ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(0, 12),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        })
        .unwrap(),
    );

    let ref_val = ref_resp.response_result.expect("references should succeed");
    let refs: Option<Vec<lsp_types::Location>> = serde_json::from_value(ref_val).unwrap();
    assert!(refs.is_some(), "Expected references to be found");
    assert_eq!(refs.unwrap().len(), 2, "Expected declaration + reference");

    // 4. Clean shutdown
    let shutdown_resp = client.request("shutdown", serde_json::Value::Null);
    assert!(shutdown_resp.response_result.is_ok());
    client.notify("exit", serde_json::Value::Null);
    let exit_kind = client.join();
    assert_eq!(exit_kind, htmx_analyzer::ExitKind::Clean);
}
