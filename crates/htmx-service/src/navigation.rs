use htmx_parser::{find_id_definitions, find_id_reference_at, find_id_references, lex};
use htmx_vfs::Document;
use lsp_types::{GotoDefinitionResponse, Location, LocationLink, Position, Range, Uri};
use smallvec::SmallVec;
use std::collections::HashMap;

/// Location of an ID definition within a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdLocation {
    pub uri: Uri,
    pub range: Range,     // Exact span of ID text
    pub tag_range: Range, // Span of enclosing element
}

/// In-memory symbol index of all declared element IDs across the workspace.
#[derive(Default, Debug, Clone)]
pub struct WorkspaceIdIndex {
    /// Maps an ID to its definition locations.
    /// SmallVec keeps the single location inline with zero heap allocation.
    defs: HashMap<String, SmallVec<[IdLocation; 1]>>,
    file_to_ids: HashMap<Uri, Vec<String>>,
}

impl WorkspaceIdIndex {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Update definitions for a single file from its raw text.
    pub fn update_file(&mut self, uri: &Uri, text: &str) {
        self.remove_file(uri);

        let tags = lex(text);
        let id_defs = find_id_definitions(&tags);
        if id_defs.is_empty() {
            return;
        }

        let doc = Document::new(uri.clone(), 0, text);
        let mut file_ids = Vec::with_capacity(id_defs.len());

        for def in id_defs {
            let Some(start_pos) = doc.offset_to_position(def.value_range.start) else {
                continue;
            };
            let Some(end_pos) = doc.offset_to_position(def.value_range.end) else {
                continue;
            };
            let range = Range {
                start: start_pos,
                end: end_pos,
            };

            let Some(tag_start) = doc.offset_to_position(def.tag_range.start) else {
                continue;
            };
            let Some(tag_end) = doc.offset_to_position(def.tag_range.end) else {
                continue;
            };
            let tag_range = Range {
                start: tag_start,
                end: tag_end,
            };

            let loc = IdLocation {
                uri: uri.clone(),
                range,
                tag_range,
            };

            file_ids.push(def.id.to_string());
            self.defs.entry(def.id.to_string()).or_default().push(loc);
        }

        self.file_to_ids.insert(uri.clone(), file_ids);
    }

    /// Remove all definitions indexed from a file.
    pub fn remove_file(&mut self, uri: &Uri) {
        if let Some(ids) = self.file_to_ids.remove(uri) {
            for id in ids {
                if let Some(locations) = self.defs.get_mut(&id) {
                    locations.retain(|loc| loc.uri != *uri);
                    if locations.is_empty() {
                        self.defs.remove(&id);
                    }
                }
            }
        }
    }

    /// Retrieve all definition locations for an ID.
    #[must_use]
    pub fn get_definitions(&self, id: &str) -> &[IdLocation] {
        self.defs.get(id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Check if the index contains definitions for an ID.
    #[must_use]
    pub fn contains_id(&self, id: &str) -> bool {
        self.defs.get(id).is_some_and(|v| !v.is_empty())
    }

    /// Merge another workspace index into this one.
    pub fn merge(&mut self, other: WorkspaceIdIndex) {
        for (uri, ids) in other.file_to_ids {
            self.file_to_ids.insert(uri, ids);
        }
        for (id, locations) in other.defs {
            let entry = self.defs.entry(id).or_default();
            for loc in locations {
                if !entry
                    .iter()
                    .any(|existing| existing.uri == loc.uri && existing.range == loc.range)
                {
                    entry.push(loc);
                }
            }
        }
    }
}

/// Navigate from an ID reference to its definition `<tag id="...">`.
#[must_use]
pub fn goto_definition(
    _uri: &Uri,
    position: Position,
    doc: &Document,
    index: &WorkspaceIdIndex,
    link_support: bool,
) -> Option<GotoDefinitionResponse> {
    let offset = doc.position_to_offset(position)?;
    let text = doc.text();
    let tags = lex(&text);
    let id_ref = find_id_reference_at(&tags, offset)?;

    let defs = index.get_definitions(id_ref.id);
    if defs.is_empty() {
        return None;
    }

    if link_support {
        let origin_range = Range {
            start: doc.offset_to_position(id_ref.origin_span.start)?,
            end: doc.offset_to_position(id_ref.origin_span.end)?,
        };
        let links: Vec<LocationLink> = defs
            .iter()
            .map(|d| LocationLink {
                origin_selection_range: Some(origin_range),
                target_uri: d.uri.clone(),
                target_range: d.tag_range,
                target_selection_range: d.range,
            })
            .collect();
        Some(GotoDefinitionResponse::Link(links))
    } else if defs.len() == 1 {
        Some(GotoDefinitionResponse::Scalar(Location {
            uri: defs[0].uri.clone(),
            range: defs[0].range,
        }))
    } else {
        let locs: Vec<Location> = defs
            .iter()
            .map(|d| Location {
                uri: d.uri.clone(),
                range: d.range,
            })
            .collect();
        Some(GotoDefinitionResponse::Array(locs))
    }
}

/// Find all references to the ID under the cursor.
#[must_use]
pub fn find_references(
    _uri: &Uri,
    position: Position,
    doc: &Document,
    open_docs: &[&Document],
    index: &WorkspaceIdIndex,
    include_declaration: bool,
) -> Vec<Location> {
    let mut results = Vec::new();
    let Some(offset) = doc.position_to_offset(position) else {
        return results;
    };
    let text = doc.text();
    let tags = lex(&text);
    let Some(target_ref) = find_id_reference_at(&tags, offset) else {
        return results;
    };
    let target_id = target_ref.id;

    for open_doc in open_docs {
        let doc_text = open_doc.text();
        let doc_tags = lex(&doc_text);
        let refs = find_id_references(&doc_tags);
        for r in refs {
            if r.id != target_id {
                continue;
            }
            if !include_declaration && r.is_declaration {
                continue;
            }
            if let (Some(start), Some(end)) = (
                open_doc.offset_to_position(r.origin_span.start),
                open_doc.offset_to_position(r.origin_span.end),
            ) {
                results.push(Location {
                    uri: open_doc.uri.clone(),
                    range: Range { start, end },
                });
            }
        }
    }

    // If declaration was requested and any definition exists in closed workspace files
    if include_declaration {
        for def in index.get_definitions(target_id) {
            let already_included = results
                .iter()
                .any(|loc| loc.uri == def.uri && loc.range == def.range);
            if !already_included {
                results.push(Location {
                    uri: def.uri.clone(),
                    range: def.range,
                });
            }
        }
    }

    results
}
