use lsp_types::{Position, Range, TextDocumentContentChangeEvent, Uri};
use ropey::Rope;

/// A managed in-memory text document backed by a `ropey::Rope`.
#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Uri,
    pub version: i32,
    pub rope: Rope,
}

impl Document {
    pub fn new(uri: Uri, version: i32, text: &str) -> Self {
        Self {
            uri,
            version,
            rope: Rope::from_str(text),
        }
    }

    /// Return the full text of the document as an owned String.
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Convert an LSP Position (0-indexed line and UTF-16 character offset) to a UTF-8 byte offset.
    pub fn position_to_offset(&self, position: Position) -> Option<usize> {
        let line_idx = position.line as usize;
        if line_idx >= self.rope.len_lines() {
            return None;
        }

        let line = self.rope.line(line_idx);
        let char_cu = (position.character as usize).min(line.len_utf16_cu());
        let char_offset_in_line = line.utf16_cu_to_char(char_cu);
        let line_char_start = self.rope.line_to_char(line_idx);
        let total_char_offset = line_char_start + char_offset_in_line;

        if total_char_offset > self.rope.len_chars() {
            return None;
        }

        Some(self.rope.char_to_byte(total_char_offset))
    }

    /// Convert a UTF-8 byte offset to an LSP Position (UTF-16 character code units).
    pub fn offset_to_position(&self, offset: usize) -> Option<Position> {
        let offset = offset.min(self.rope.len_bytes());
        let char_idx = self.rope.byte_to_char(offset);
        let line_idx = self.rope.char_to_line(char_idx);
        let line_char_start = self.rope.line_to_char(line_idx);
        let char_in_line = char_idx - line_char_start;

        let line = self.rope.line(line_idx);
        let utf16_cu = line.char_to_utf16_cu(char_in_line);

        Some(Position {
            line: line_idx as u32,
            character: utf16_cu as u32,
        })
    }

    /// Apply incremental or full content change events.
    pub fn apply_changes(
        &mut self,
        changes: Vec<TextDocumentContentChangeEvent>,
        new_version: i32,
    ) {
        self.version = new_version;

        for change in changes {
            if let Some(range) = change.range {
                self.apply_range_change(range, &change.text);
            } else {
                // Full document replacement
                self.rope = Rope::from_str(&change.text);
            }
        }
    }

    fn apply_range_change(&mut self, range: Range, new_text: &str) {
        let start_offset = self.position_to_offset(range.start);
        let end_offset = self.position_to_offset(range.end);

        if let (Some(start_byte), Some(end_byte)) = (start_offset, end_offset) {
            let start_char = self.rope.byte_to_char(start_byte);
            let end_char = self.rope.byte_to_char(end_byte);

            if start_char <= end_char && end_char <= self.rope.len_chars() {
                self.rope.remove(start_char..end_char);
                self.rope.insert(start_char, new_text);
            }
        }
    }
}
