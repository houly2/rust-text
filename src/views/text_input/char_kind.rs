#[derive(Debug, PartialEq)]
pub enum CharKind {
    WhiteSpace,
    Word,
    Punctuation,
}

impl CharKind {
    pub fn kind(c: char) -> CharKind {
        if c.is_whitespace() {
            return CharKind::WhiteSpace;
        } else if c.is_alphanumeric() || c == '_' {
            return CharKind::Word;
        }

        CharKind::Punctuation
    }
}
