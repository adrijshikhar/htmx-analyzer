use htmx_service::HtmxLanguageService;
use htmx_vfs::DocumentStore;
use lsp_types::{
    CodeActionParams, CodeActionResponse, CompletionParams, CompletionResponse,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, Hover,
    HoverParams, PublishDiagnosticsParams,
};
use tracing::{debug, info};

pub fn handle_completion(
    service: &HtmxLanguageService,
    doc_store: &DocumentStore,
    params: CompletionParams,
) -> Option<CompletionResponse> {
    let uri = params.text_document_position.text_document.uri;
    let pos = params.text_document_position.position;

    debug!("Handling completion request for {:?} at {:?}", uri, pos);

    let doc = doc_store.get(&uri)?;
    let items = service.complete(doc, pos);
    Some(CompletionResponse::Array(items))
}

pub fn handle_hover(
    service: &HtmxLanguageService,
    doc_store: &DocumentStore,
    params: HoverParams,
) -> Option<Hover> {
    let uri = params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;

    debug!("Handling hover request for {:?} at {:?}", uri, pos);

    let doc = doc_store.get(&uri)?;
    service.hover(doc, pos)
}

pub fn handle_code_action(
    service: &HtmxLanguageService,
    doc_store: &DocumentStore,
    params: CodeActionParams,
) -> Option<CodeActionResponse> {
    let uri = params.text_document.uri;
    let range = params.range;
    let diags = params.context.diagnostics;

    debug!("Handling code action for {:?} at {:?}", uri, range);

    let doc = doc_store.get(&uri)?;
    let actions = service.code_actions(doc, range, &diags);
    Some(actions)
}

pub fn handle_did_open(
    service: &HtmxLanguageService,
    doc_store: &mut DocumentStore,
    params: DidOpenTextDocumentParams,
) -> PublishDiagnosticsParams {
    let item = params.text_document;
    info!("Opened document: {:?}", item.uri);

    doc_store.open(item.uri.clone(), item.version, &item.text);

    let diagnostics = if let Some(doc) = doc_store.get(&item.uri) {
        service.validate(doc)
    } else {
        Vec::new()
    };

    PublishDiagnosticsParams {
        uri: item.uri,
        diagnostics,
        version: Some(item.version),
    }
}

pub fn handle_did_change(
    service: &HtmxLanguageService,
    doc_store: &mut DocumentStore,
    params: DidChangeTextDocumentParams,
) -> Option<PublishDiagnosticsParams> {
    let uri = params.text_document.uri;
    let version = params.text_document.version;

    debug!("Changed document: {:?} (version {})", uri, version);

    doc_store.change(&uri, version, params.content_changes);

    let doc = doc_store.get(&uri)?;
    let diagnostics = service.validate(doc);

    Some(PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: Some(version),
    })
}

pub fn handle_did_close(
    doc_store: &mut DocumentStore,
    params: DidCloseTextDocumentParams,
) -> PublishDiagnosticsParams {
    let uri = params.text_document.uri;
    info!("Closed document: {:?}", uri);

    doc_store.close(&uri);

    // Clear diagnostics on close
    PublishDiagnosticsParams {
        uri,
        diagnostics: Vec::new(),
        version: None,
    }
}
