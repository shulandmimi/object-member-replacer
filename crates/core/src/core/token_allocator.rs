use rustc_hash::FxHashSet;
use lazy_static::lazy_static;

use super::constant;

lazy_static! {
    static ref PRESERVE_KEYWORDS: FxHashSet<&'static str> = {
        FxHashSet::from_iter([
            "break",
            "case",
            "catch",
            "class",
            "const",
            "continue",
            "debugger",
            "default",
            "delete",
            "do",
            "else",
            "enum",
            "export",
            "extends",
            "false",
            "finally",
            "for",
            "function",
            "if",
            "import",
            "in",
            "instanceof",
            "new",
            "null",
            "return",
            "super",
            "switch",
            "this",
            "throw",
            "true",
            "try",
            "typeof",
            "var",
            "void",
            "while",
            "with",
            "as",
            "implements",
            "interface",
            "let",
            "package",
            "private",
            "protected",
            "public",
            "static",
            "yield",
            "any",
            "boolean",
            "constructor",
            "declare",
            "get",
            "module",
            "require",
            "number",
            "set",
            "string",
            "symbol",
            "type",
            "from",
            "of",
        ])
    };
}

#[derive(Debug, Default)]
pub struct TokenAllocator {
    pos: usize,
    used_ident: FxHashSet<String>,
}

impl TokenAllocator {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn extends(&mut self, set: FxHashSet<String>) {
        self.used_ident.extend(set);
    }

    pub fn allocable(&self, ident: &str) -> bool {
        !(PRESERVE_KEYWORDS.contains(ident) || self.used_ident.contains(ident))
    }

    fn ident(&self) -> String {
        let mut pos = self.pos / constant::COMPRESS_CHARACTER_WIDTH as usize;
        let mut r = String::new();

        let mut push_ch = |ch: u8| {
            r.push(if ch < 26 {
                (b'a' + ch) as char
            } else {
                (b'A' + (ch - 26)) as char
            });
        };

        while pos > 0 {
            let ch = (pos - 1) % constant::COMPRESS_CHARACTER_WIDTH as usize;

            push_ch(ch as u8);

            pos /= constant::COMPRESS_CHARACTER_WIDTH as usize;
        }

        push_ch((self.pos % constant::COMPRESS_CHARACTER_WIDTH as usize) as u8);

        r
    }

    pub fn alloc(&mut self) -> String {
        loop {
            let s = self.ident();
            self.pos += 1;

            if self.allocable(&s) {
                self.used_ident.insert(s.clone());
                return s;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ident_alloc() {
        let mut token = TokenAllocator::new();

        let v = (0..200).map(|_| token.alloc()).collect::<Vec<_>>();

        assert_eq!(v[0], "a");
        assert_eq!(v[199], "cS");
    }

    #[test]
    fn ident_alloc_with_used() {
        let mut token = TokenAllocator::new();

        token.used_ident.insert("b".to_string());

        let v = (0..200).map(|_| token.alloc()).collect::<Vec<_>>();

        assert_eq!(v[0], "a");
        assert_eq!(v[199], "cT");
    }
}
