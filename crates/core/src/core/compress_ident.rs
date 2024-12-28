use std::collections::HashMap;

use itertools::Itertools;

pub struct IdentCost {
    pub first_cost: usize,
    pub more_cost: isize,
    pub original_cost: isize,
    pub compress_cost: isize,
}

impl IdentCost {
    pub fn new(pos: usize, ch_len: usize, used_counts: usize) -> Self {
        let pos = pos as isize;
        let ch_len = ch_len as isize;
        let used_counts = used_counts as isize;

        // 预测该 ch 压缩后的长度
        let cost = (pos / 52).max(1);

        // 固定代价
        // var , var 的的代价不进行计算

        // .foo => var a = "foo"; [a]
        // 使用一次的最小代价 `="",[]`
        let v1 = 1 + 2 + 1 + 2 + (cost * 2) - 1;
        // 后续使用的代价
        let v2 = (cost + 2 - 1) - ch_len;
        let v3 = v2 * (used_counts - 1);

        Self {
            first_cost: v1 as usize,
            more_cost: v2,
            original_cost: (ch_len * used_counts),
            compress_cost: v1 + v3,
        }
    }

    pub fn should_compress(&self) -> bool {
        self.compress_cost < 0
    }
}

pub fn filter_cannot_compress_ident(map: HashMap<String, usize>) -> HashMap<String, usize> {
    let mut v = map
        .into_iter()
        .filter(|(_, c)| *c > 1)
        .sorted_by_key(|(v, _)| v.len())
        .collect::<Vec<_>>();

    let len = v.len();

    let mut iter_once = false;
    let position = v
        .iter()
        .rev()
        .enumerate()
        .rposition(|(index, (ident, count))| {
            iter_once = true;

            if ident.len() <= 2 {
                return false;
            }

            let cost = IdentCost::new(index, ident.len(), *count);

            if !cost.should_compress() {
                return false;
            }

            true
        });

    if iter_once && position.is_none() {
        return HashMap::new();
    }

    if let Some(position) = position {
        v.truncate(len - position + 1);
    }

    v.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    mod cost {
        use super::IdentCost;

        #[test]
        fn f1() {
            let v = IdentCost::new(0, 3, 1);

            assert_eq!(v.first_cost, 7);
            assert_eq!(v.more_cost, -1);
            assert_eq!(v.original_cost, 3);
            assert_eq!(v.compress_cost, 7);
            assert!(!v.should_compress());

            let v = IdentCost::new(0, 3, 8);

            assert_eq!(v.first_cost, 7);
            assert_eq!(v.more_cost, -1);
            assert_eq!(v.original_cost, 24);
            assert_eq!(v.compress_cost, 0);
            assert!(!v.should_compress());

            let v = IdentCost::new(0, 3, 20);
            assert_eq!(v.first_cost, 7);
            assert_eq!(v.more_cost, -1);
            assert_eq!(v.original_cost, 60);
            assert_eq!(v.compress_cost, -12);
            assert!(v.should_compress());

            let v = IdentCost::new(0, 3, 100);
            assert_eq!(v.first_cost, 7);
            assert_eq!(v.more_cost, -1);
            assert_eq!(v.original_cost, 300);
            assert_eq!(v.compress_cost, -92);
            assert!(v.should_compress());
        }
    }

    mod filter_compress {
        use super::*;

        #[test]
        fn cannot_compress() {
            let map = HashMap::from_iter([
                ("aaa".to_string(), 1),
                ("bbb".to_string(), 1),
                ("ccc".to_string(), 1),
                ("ddd".to_string(), 1),
                ("eee".to_string(), 1),
            ]);

            let v = filter_cannot_compress_ident(map);

            assert_eq!(v, HashMap::new());
        }

        #[test]
        fn cannot_compress_long_but_used_once() {
            let map = HashMap::from_iter([("a".repeat(20), 1)]);

            let v = filter_cannot_compress_ident(map);

            assert_eq!(v, HashMap::new());
        }

        #[test]
        fn compress_long_ident_but_low_used() {
            let s = "a".repeat(40);
            let map = HashMap::from_iter([(s.clone(), 2)]);

            let v = filter_cannot_compress_ident(map);

            assert_eq!(v, HashMap::from_iter([(s, 2)]));
        }
    }
}
