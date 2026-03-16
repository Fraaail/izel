use std::str::Chars;

pub const EOF_CHAR: char = '\0';

/// A cursor over a source string, providing character-by-character access.
pub struct Cursor<'a> {
    chars: Chars<'a>,
    len_remaining: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars(),
            len_remaining: source.len(),
        }
    }

    /// Returns the first character without consuming it.
    pub fn first(&self) -> char {
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    /// Returns the second character without consuming it.
    pub fn second(&self) -> char {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Checks if the cursor is at the end of the file.
    pub fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    /// Consumes the next character.
    pub fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.len_remaining -= c.len_utf8();
        Some(c)
    }

    /// Consumes characters while the predicate is true.
    pub fn eat_while<F>(&mut self, mut mut_pred: F)
    where
        F: FnMut(char) -> bool,
    {
        while mut_pred(self.first()) && !self.is_eof() {
            self.bump();
        }
    }

    /// Returns the number of bytes consumed since the cursor was created.
    pub fn pos_within(&self, source: &str) -> usize {
        source.len() - self.chars.as_str().len()
    }
}
