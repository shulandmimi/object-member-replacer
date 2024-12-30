use std::collections::HashSet;

use super::constant;

#[derive(Debug, Default)]
pub struct TokenAllocator {
    pos: usize,
    used_allocator: HashSet<String>,
}

impl TokenAllocator {
    pub fn new() -> Self {
        Default::default()
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

            if !self.used_allocator.contains(&s) {
                self.used_allocator.insert(s.clone());
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
        assert_eq!(v[199], "cR");
    }

    #[test]
    fn ident_alloc_with_used() {
        let mut token = TokenAllocator::new();

        token.used_allocator.insert("b".to_string());

        let v = (0..200).map(|_| token.alloc()).collect::<Vec<_>>();

        assert_eq!(v[0], "a");
        assert_eq!(v[199], "cS");
    }
}
