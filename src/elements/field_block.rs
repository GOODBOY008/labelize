use super::text_alignment::TextAlignment;

#[derive(Clone, Debug)]
pub struct FieldBlock {
    pub max_width: i32,
    pub max_lines: i32,
    pub line_spacing: i32,
    pub alignment: TextAlignment,
    pub hanging_indent: i32,
    /// Break words that exceed the line width at character boundaries.
    pub wrap_long_words: bool,
    /// Draw excess lines over the final line slot instead of discarding them.
    pub overwrite_last_line: bool,
}

impl Default for FieldBlock {
    fn default() -> Self {
        FieldBlock {
            max_width: 0,
            max_lines: 1,
            line_spacing: 0,
            alignment: TextAlignment::Left,
            hanging_indent: 0,
            wrap_long_words: false,
            overwrite_last_line: false,
        }
    }
}
