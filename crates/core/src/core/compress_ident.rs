use std::cmp::Reverse;

use itertools::Itertools;
use rustc_hash::FxHashMap;

use super::constant;

#[allow(dead_code)]
struct ConstantCost {
    first_cost: usize,
    more_cost: isize,
}

struct HostingVariable;

#[allow(dead_code)]
struct HostingCustom(ConstantCost);

const VAR_HOSTING: HostingVariable = HostingVariable;

#[allow(dead_code)]
trait CostCalculator: Sized {
    // fn create(pos: usize, ident_len: usize, used_counts: usize) -> Self;
    fn first_cost() -> usize;
    fn more_cost() -> isize;
    fn should_compress(&self, pos: usize, ident_len: usize, used_counts: usize) -> bool;
}

impl CostCalculator for HostingVariable {
    // .foo => var a = "foo"; [a]
    // first usage cost `="",[]`
    fn first_cost() -> usize {
        6
    }

    // static cost
    // ._ => [_]
    fn more_cost() -> isize {
        2 - 1
    }

    fn should_compress(&self, pos: usize, ident_len: usize, used_counts: usize) -> bool {
        let pos = pos as isize;
        let ch_len = ident_len as isize;
        let used_counts = used_counts as isize;

        // predict the length after compressing ch.
        let cost = (pos / constant::COMPRESS_CHARACTER_WIDTH as isize).max(1);

        // Fixed cost
        // The cost of var, now var is not calculated
        let v1 = (Self::first_cost() as isize) + (cost * 2) - 1;

        // cost of subsequent use .a => [a], cost: 1, more_cost: -1 ch_len: xxx.len
        let v2 = (cost + Self::more_cost()) - ch_len;
        // all cost
        let v3 = v2 * (used_counts - 1);

        v1 + v3 < 0
    }
}

pub fn filter_cannot_compress_ident(map: FxHashMap<String, usize>) -> FxHashMap<String, usize> {
    let mut v = map
        .into_iter()
        .filter(|(i, c)| *c > 1 && i.len() > 2)
        .sorted_by_key(|(_, c)| Reverse(*c))
        .sorted_by_key(|(v, _)| Reverse(v.len()))
        .collect::<Vec<_>>();

    let mut end = v.len();

    // TODO: binary search
    for i in (0..end).rev() {
        let (ident, count) = &v[i];

        if VAR_HOSTING.should_compress(i, ident.len(), *count) {
            break;
        }

        end = i;
    }

    if end != v.len() {
        v.truncate(end);
    };

    v.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    mod cost {
        use crate::core::compress_ident::{CostCalculator, VAR_HOSTING};

        #[test]
        fn f1() {
            let v = VAR_HOSTING.should_compress(0, 3, 1);

            assert!(!v);

            let v = VAR_HOSTING.should_compress(0, 3, 8);

            assert!(!v);

            let v = VAR_HOSTING.should_compress(0, 3, 20);
            assert!(v);

            let v = VAR_HOSTING.should_compress(0, 3, 100);
            assert!(v);
        }
    }

    mod filter_compress {
        use super::*;

        #[test]
        fn cannot_compress() {
            let map = FxHashMap::from_iter([
                ("aaa".to_string(), 1),
                ("bbb".to_string(), 1),
                ("ccc".to_string(), 1),
                ("ddd".to_string(), 1),
                ("eee".to_string(), 1),
            ]);

            let v = filter_cannot_compress_ident(map);

            assert_eq!(v, FxHashMap::default());
        }

        #[test]
        fn cannot_compress_long_but_used_once() {
            let map = FxHashMap::from_iter([("a".repeat(20), 1)]);

            let v = filter_cannot_compress_ident(map);

            assert_eq!(v, FxHashMap::default());
        }

        #[test]
        fn compress_long_ident_but_low_used() {
            let s = "a".repeat(40);
            let map = FxHashMap::from_iter([(s.clone(), 2)]);

            let v = filter_cannot_compress_ident(map);

            assert_eq!(v, FxHashMap::from_iter([(s, 2)]));
        }

        #[test]
        fn t1() {
            let map = FxHashMap::from_iter([("a".to_string(), 10)]);

            let v = filter_cannot_compress_ident(map);

            assert_eq!(v, FxHashMap::default());

            let map = FxHashMap::from_iter([("aa".to_string(), 1000)]);

            let v = filter_cannot_compress_ident(map);

            assert_eq!(v, FxHashMap::default());
        }

        #[test]
        fn t2() {
            let map = FxHashMap::from_iter([
                ("aa1".to_string(), 1),
                ("aa2".to_string(), 2),
                ("aa3".to_string(), 3),
                ("aa4".to_string(), 4),
                ("aa5".to_string(), 5),
                ("aa6".to_string(), 6),
                ("aa7".to_string(), 7),
                ("aa8".to_string(), 8),
                ("aa9".to_string(), 9),
                ("aaa".to_string(), 10),
            ]);

            let v = filter_cannot_compress_ident(map);

            assert_eq!(
                v,
                FxHashMap::from_iter([("aaa".to_string(), 10), ("aa9".to_string(), 9)])
            );
        }

        #[test]
        fn t3() {
            let map = FxHashMap::from_iter([("localStorage".to_string(), 2)]);

            let v = filter_cannot_compress_ident(map.clone());

            assert_eq!(v, map);
        }
    }
}
