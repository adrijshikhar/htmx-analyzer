use crate::document::Document;
use lsp_types::{TextDocumentContentChangeEvent, Uri};
use std::collections::HashMap;

/// In-memory document storage and synchronization manager.
#[derive(Debug, Default)]
pub struct DocumentStore {
    documents: HashMap<Uri, Document>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    /// Open and store a new document.
    pub fn open(&mut self, uri: Uri, version: i32, text: &str) {
        let doc = Document::new(uri.clone(), version, text);
        self.documents.insert(uri, doc);
    }

    /// Apply incremental or full updates to an open document.
    pub fn change(
        &mut self,
        uri: &Uri,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) {
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.apply_changes(changes, version);
        }
    }

    /// Remove a closed document from memory.
    pub fn close(&mut self, uri: &Uri) -> Option<Document> {
        self.documents.remove(uri)
    }

    /// Retrieve an immutable reference to an open document.
    pub fn get(&self, uri: &Uri) -> Option<&Document> {
        self.documents.get(uri)
    }

    /// Retrieve a mutable reference to an open document.
    pub fn get_mut(&mut self, uri: &Uri) -> Option<&mut Document> {
        self.documents.get_mut(uri)
    }

    /// Number of open documents in memory.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}
