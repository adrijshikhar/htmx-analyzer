use htmx_service::HtmxLanguageService;
use htmx_vfs::DocumentStore;
use lsp_types::{
    CodeActionParams, CodeActionResponse, CompletionParams, CompletionResponse,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, Hover,
    HoverParams,
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
    let list = service.complete(doc, pos);
    Some(CompletionResponse::List(list))
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

pub fn handle_did_open(doc_store: &mut DocumentStore, params: DidOpenTextDocumentParams) {
    let item = params.text_document;
    info!("Opened document: {:?}", item.uri);
    doc_store.open(item.uri, item.version, &item.text);
}

pub fn handle_did_change(doc_store: &mut DocumentStore, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri;
    let version = params.text_document.version;

    debug!("Changed document: {:?} (version {})", uri, version);
    doc_store.change(&uri, version, params.content_changes);
}

pub fn handle_did_close(doc_store: &mut DocumentStore, params: DidCloseTextDocumentParams) {
    let uri = params.text_document.uri;
    info!("Closed document: {:?}", uri);
    doc_store.close(&uri);
}
