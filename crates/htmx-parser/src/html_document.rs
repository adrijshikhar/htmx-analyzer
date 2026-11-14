use std::collections::HashSet;
use tree_sitter::{Language, Node, Parser, Tree};

/// Guard limit for document size to prevent DoS (5 MB).
pub const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;

/// Stack safety depth limit for iterative AST walks.
pub const MAX_DEPTH: usize = 256;

/// Parsed HTML document wrapper using Tree-Sitter with DoS protection.
pub struct HtmlDocument {
    pub tree: Option<Tree>,
    pub document_ids: HashSet<String>,
}

impl HtmlDocument {
    /// Parse HTML source with Tree-Sitter and extract document IDs.
    pub fn parse(source: &str) -> Self {
        if source.len() > MAX_FILE_SIZE {
            return Self {
                tree: None,
                document_ids: HashSet::new(),
            };
        }

        let mut parser = Parser::new();
        let language: Language = tree_sitter_html::LANGUAGE.into();
        if parser.set_language(&language).is_err() {
            return Self {
                tree: None,
                document_ids: HashSet::new(),
            };
        }

        let tree = parser.parse(source, None);
        let mut document_ids = HashSet::new();

        if let Some(ref t) = tree {
            Self::extract_ids(t.root_node(), source, &mut document_ids);
        }

        Self { tree, document_ids }
    }

    /// Iterative DFS traversal to extract all `id="..."` attributes without recursion.
    fn extract_ids(root: Node, source: &str, ids: &mut HashSet<String>) {
        let mut stack = vec![(root, 0usize)];

        while let Some((node, depth)) = stack.pop() {
            if depth > MAX_DEPTH {
                continue;
            }

            // Check if node is an attribute
            if node.kind() == "attribute" {
                let mut is_id_attr = false;
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "attribute_name" {
                        let name_text = &source[child.byte_range()];
                        if name_text == "id" {
                            is_id_attr = true;
                        }
                    } else if is_id_attr
                        && (child.kind() == "quoted_attribute_value"
                            || child.kind() == "attribute_value")
                    {
                        let raw_val = &source[child.byte_range()];
                        let clean_val = raw_val.trim_matches(|c| c == '"' || c == '\'');
                        if !clean_val.is_empty() {
                            ids.insert(clean_val.to_string());
                        }
                        break;
                    }
                }
            }

            // Push children using linear TreeCursor
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                stack.push((child, depth + 1));
            }
        }
    }
}
