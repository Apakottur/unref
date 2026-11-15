//! Lines and columns of the positions in a module, as a reader sees them.

use ruff_source_file::LineIndex;
use ruff_text_size::TextSize;

/// Turn the positions in a syntax tree into the lines and columns that a reader sees, as ruff and ty
/// report them: lines start at 1, and columns count the characters of the line, starting at 1.
pub struct SourcePositions<'a> {
    text: &'a str,
    index: LineIndex,
}

impl<'a> SourcePositions<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            index: LineIndex::from_source_text(text),
        }
    }

    /// Get the line and the column of an offset in the text.
    pub fn line_column(&self, offset: TextSize) -> (u32, u32) {
        let location = self.index.line_column(offset, self.text);
        (to_u32(location.line.get()), to_u32(location.column.get()))
    }
}

/// Convert a line or column to the size that findings keep it in.
pub fn to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
