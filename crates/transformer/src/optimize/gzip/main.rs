use std::{cell::RefCell, mem};

use rustc_hash::FxHashMap;

use crate::optimize::gzip::{
    git_bits::{get_bit, get_bits},
    util::{parse_distance_code, parse_length_code},
};

#[allow(dead_code)]
#[derive(Debug)]
enum TokenDetail {
    Repeat {
        length: usize,
        length_bits: usize,
        distance: usize,
        distance_bits: usize,
    },
    Literal {
        bits: usize,
    },
}

#[derive(Debug)]
pub struct Position {
    pub start: usize,
    pub end: usize,
    pub bits: f64,
    pub reference: Vec<(usize, usize)>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct InflateToken {
    value: usize,
    bits: f64,
    details: TokenDetail,
}

fn check_gzip(array: &[u8], head_box: &mut [usize]) -> bool {
    let id1 = &array[0];
    let id2 = &array[1];

    if *id1 == 0x1f && *id2 == 0x8b {
        println!("Assuming I'm processing gzip");

        let cm = &array[2];
        let flg = &array[3];

        if *cm == 8 && *flg == 0 {
            head_box[0] = 80; // 10 bytes
                              // head_box
            return true;
        } else {
            println!("Encountered unsupported gzip feature");
            return false;
        }
    } else {
        return false;
    }
}

fn check_zlib(array: &[u8], head_box: &mut [usize]) -> bool {
    let cm = get_bits(array, head_box, 4);
    let c_info = get_bits(array, head_box, 4);
    get_bits(array, head_box, 5);
    let f_dict = get_bits(array, head_box, 1);
    get_bits(array, head_box, 2);

    let check = (array[0] as usize) * 256 + (array[1] as usize);

    if cm == 8 && c_info == 7 && f_dict == 0 && check % 31 == 0 {
        //   println!( 'Assuming I\'m processing zlib deflate' );

        return true;
    } else {
        head_box[0] = 0;

        return false;
    }
}

enum Formate {
    Gzip,
    #[allow(dead_code)]
    Zlib,
    DeflateRaw,
}

struct TreeSet {
    lit_code_lengths: Vec<usize>,
    lit_code_tree: FxHashMap<usize, usize>,
    dist_code_lengths: Vec<usize>,
    dist_code_tree: FxHashMap<usize, usize>,
}

const CODE_CODE_LENGTH_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

fn build_huffman_tree(code_lengths: &[usize]) -> FxHashMap<usize, usize> {
    let max_bits = code_lengths.iter().fold(0, |prev, cur| prev.max(*cur));

    let mut code = 0;
    let mut bl_count = vec![0; max_bits + 1];
    let mut next_code = vec![0; max_bits + 1];

    for len in code_lengths {
        if *len >= 1 {
            bl_count[len - 1] += 1;
        }
    }

    // code = (0 + 0) << 1
    for i_bit in 1..max_bits {
        code = code + bl_count[i_bit - 1] << 1;
        next_code[i_bit] = code;
    }

    let mut tree = FxHashMap::<usize, usize>::default();

    let max_code = code_lengths.len();
    for i_code in 0..max_code {
        let len = code_lengths[i_code];

        if len != 0 {
            tree.insert(next_code[len - 1], i_code);
            next_code[len - 1] += 1;
        }
    }

    return tree;
}

fn decode_huffman(
    array: &[u8],
    head_box: &mut [usize],
    code_lengths: &[usize],
    tree: &FxHashMap<usize, usize>,
) -> usize {
    let mut head = head_box[0];

    let mut alpha = 0;
    let mut length = 0;
    #[allow(unused_assignments)]
    let mut value = 0;

    loop {
        length += 1;
        alpha |= get_bit(array, head);
        head += 1;

        if tree.contains_key(&alpha) {
            value = tree.get(&alpha).unwrap().clone();

            if code_lengths[value] == length {
                break;
            }
        }

        alpha <<= 1;
    }

    head_box[0] = head;

    return value;
}

fn parse_dynamic_huffman_tree(array: &[u8], head_box: &mut [usize]) -> TreeSet {
    let lit_codes = 257 + get_bits(array, head_box, 5);
    let dist_codes = 1 + get_bits(array, head_box, 5);
    let code_codes = 4 + get_bits(array, head_box, 4);

    // parse code code lengths
    let mut code_code_lengths = vec![0; CODE_CODE_LENGTH_ORDER.len()];

    for i_code in 0..code_codes {
        let value = CODE_CODE_LENGTH_ORDER[i_code];
        code_code_lengths[value] = get_bits(array, head_box, 3);
    }

    // build code code tree
    let code_code_tree = build_huffman_tree(&code_code_lengths);

    // parse literal and length + distance code lengths
    let mut lit_dist_code_lengths = vec![0; lit_codes + dist_codes];

    let mut prev_length = 0;

    let mut i_code = 0;
    while i_code < lit_codes + dist_codes {
        let value = decode_huffman(array, head_box, &code_code_lengths, &code_code_tree);

        if value == 16 {
            // `repeat`, 3 - 6
            let count = 3 + get_bits(array, head_box, 2);

            for _ in 0..count {
                lit_dist_code_lengths[i_code] = prev_length;
                i_code += 1;
            }
        } else if value == 17 {
            // `zeros`, 3 - 10
            let count = 3 + get_bits(array, head_box, 3);
            i_code += count;
        } else if value == 18 {
            // `zeros`, 11 - 138
            let count = 11 + get_bits(array, head_box, 7);
            i_code += count;
        } else {
            lit_dist_code_lengths[i_code] = value;
            i_code += 1;
            prev_length = value;
        }
    }

    let dist_code_lengths = lit_dist_code_lengths.split_off(lit_codes);
    let lit_code_tree = build_huffman_tree(&lit_dist_code_lengths);
    let dist_code_tree = build_huffman_tree(&dist_code_lengths);

    return TreeSet {
        lit_code_lengths: lit_dist_code_lengths,
        lit_code_tree,
        dist_code_lengths,
        dist_code_tree,
    };
}

fn process_block_using_tree(
    array: &[u8],
    head_box: &mut [usize],
    raw: &mut Vec<usize>,
    TreeSet {
        lit_code_lengths,
        lit_code_tree,
        dist_code_lengths,
        dist_code_tree,
    }: TreeSet,
) -> (Vec<InflateToken>, Vec<Position>) {
    let mut tokens: Vec<InflateToken> = vec![];
    let mut ranges = vec![];

    loop {
        let head_before_alpha = head_box[0];
        let value = decode_huffman(array, head_box, &lit_code_lengths, &lit_code_tree);

        if value == 256 {
            // end-of-block
            break;
        } else if value > 256 {
            // `match`
            let length = parse_length_code(array, head_box, value);
            let head_before_dist = head_box[0];
            let length_bits = head_before_dist - head_before_alpha;

            let dist_code = decode_huffman(array, head_box, &dist_code_lengths, &dist_code_tree);
            let distance = parse_distance_code(array, head_box, dist_code);
            let distance_bits = head_box[0] - head_before_dist;

            let start = raw.len() - distance;
            // let end = start + length;
            // let reference = [start, ]
            let sliced: Vec<usize> = (&raw[start..(start + length).min(raw.len())])
                .iter()
                .cloned()
                .collect();

            let mut reference = vec![];
            let concat = if sliced.len() < length {
                // concat
                let v = vec![0; length]
                    .into_iter()
                    .enumerate()
                    .map(|(_, i)| sliced[i % sliced.len()])
                    .collect::<Vec<_>>();
                reference.push((start, start + sliced.len()));
                v
            } else {
                sliced
            };

            let bits = ((length_bits + distance_bits) as f64) / (length as f64);

            ranges.push(Position {
                start: raw.len(),
                end: raw.len() + concat.len(),
                bits,
                reference,
            });

            raw.extend(concat.clone());

            concat.into_iter().for_each(|value| {
                tokens.push(InflateToken {
                    value,
                    bits,
                    details: TokenDetail::Repeat {
                        length,
                        length_bits: length_bits as usize,
                        distance,
                        distance_bits: distance_bits as usize,
                    },
                })
            });
        } else {
            raw.push(value);

            let bits = head_box[0] - head_before_alpha;

            tokens.push(InflateToken {
                value,
                bits: bits as f64,
                details: TokenDetail::Literal {
                    bits: bits as usize,
                },
            });
        }
    }

    return (tokens, ranges);
}

struct ParsedBlock {
    tokens: Vec<InflateToken>,
    is_final_block: bool,
    ranges: Vec<Position>,
}

fn process_block(array: &[u8], head_box: &mut [usize], raw: &mut Vec<usize>) -> ParsedBlock {
    let is_final_block = get_bits(array, head_box, 1) == 1;
    let ty = get_bits(array, head_box, 2);

    if ty == 0 {
        panic!("Unsupported block");
    } else if ty == 1 {
        panic!("Unsupported block");
    } else {
        let tree_set = parse_dynamic_huffman_tree(array, head_box);
        let (result, ranges) = process_block_using_tree(array, head_box, raw, tree_set);

        return ParsedBlock {
            tokens: result,
            is_final_block,
            ranges,
        };
    }
}

pub fn inflate(array: Vec<u8>) -> Vec<Position> {
    let mut head_box: Vec<usize> = vec![0; 1024];

    let mut result: Vec<InflateToken> = vec![];
    let mut raw: Vec<usize> = vec![];
    let mut range_list = vec![];

    #[allow(unused_assignments)]
    let mut formate: Option<Formate> = None;

    if formate.is_none() {
        let is_gzip = check_gzip(&array, &mut head_box);
        if is_gzip {
            formate = Some(Formate::Gzip);
        }
    }

    if formate.is_none() {
        let is_zlib = check_zlib(&array, &mut head_box);
        if is_zlib {
            formate = Some(Formate::Zlib);
        }
    }

    if formate.is_none() {
        formate = Some(Formate::DeflateRaw);
    }

    if formate.is_none() {
        panic!("Unsupported format");
    }

    loop {
        let ParsedBlock {
            tokens,
            is_final_block,
            ranges,
        } = process_block(&array, &mut head_box, &mut raw);
        result.extend(tokens);
        range_list.extend(ranges);
        // tokens.push(...result.tokens);

        if is_final_block {
            break;
        }
    }

    let mut content = String::from("");

    for token in &result {
        content.push(token.value as u8 as char);
    }

    range_list
}

#[derive(Debug)]
struct SegmentRangeTree<T> {
    root: Option<SegmentNode<T>>,
}

impl<T> SegmentRangeTree<T> {
    pub fn new() -> Self {
        SegmentRangeTree { root: None }
    }

    // pub fn build() {

    // }

    pub fn insert(&mut self, range: SegmentLeaf<T>) {
        if let Some(root) = &mut self.root {
            root.insert(range);
        } else {
            self.root = Some(SegmentNode::Leaf(SegmentLeaf {
                start: range.start,
                end: range.end,
                data: None,
            }));
        }
        // self.root.insert(range);
    }

    pub fn contain(&self, range: &SegmentRange) -> bool {
        self.root
            .as_ref()
            .map(|v| v.contain(range))
            .unwrap_or(false)
    }
}

#[derive(Debug)]
pub enum SegmentNode<T> {
    Leaf(SegmentLeaf<T>),
    Node(SegmentTreeNode<T>),
}

impl<T> SegmentNode<T> {
    fn as_leaf(self) -> SegmentLeaf<T> {
        match self {
            SegmentNode::Leaf(segment_leaf) => segment_leaf,
            SegmentNode::Node(_) => panic!("Cannot convert Node to Leaf"),
        }
    }
}

pub struct SegmentRange {
    pub start: isize,
    pub end: isize,
}

impl From<(isize, isize)> for SegmentRange {
    fn from(value: (isize, isize)) -> Self {
        SegmentRange {
            start: value.0,
            end: value.1,
        }
    }
}

impl SegmentTreeTrait for SegmentRange {
    fn start(&self) -> isize {
        self.start
    }

    fn end(&self) -> isize {
        self.end
    }
}

#[derive(Debug)]
pub struct SegmentLeaf<T> {
    pub start: isize,
    pub end: isize,
    #[allow(dead_code)]
    pub data: Option<T>,
}

impl<T> From<(isize, isize)> for SegmentLeaf<T> {
    fn from(value: (isize, isize)) -> Self {
        SegmentLeaf {
            start: value.0,
            end: value.1,
            data: None,
        }
    }
}

trait SegmentTreeTrait {
    // fn insert(&self, range: (usize, usize));
    // fn distance(&self, other: &Self) -> usize;
    fn start(&self) -> isize;
    fn end(&self) -> isize;

    fn distance(&self, other: &dyn SegmentTreeTrait) -> isize {
        // if overlay, return 0
        if self.start() >= other.end() || self.end() <= other.start() {
            let start = self.start().max(other.start());
            let end = self.end().min(other.end());

            end - start
        } else {
            0
        }
    }

    // fn is_left(&self, other: &dyn SegmentTreeTrait) -> bool {
    //     self.end() <= other.start()
    // }

    fn is_left_start(&self, other: &dyn SegmentTreeTrait) -> bool {
        self.start() <= other.start()
    }

    fn is_overlay(&self, other: &dyn SegmentTreeTrait) -> bool {
        !(self.start() > other.end() || self.end() < other.start())
    }
}

impl<T> SegmentTreeTrait for SegmentLeaf<T> {
    fn start(&self) -> isize {
        self.start
    }

    fn end(&self) -> isize {
        self.end
    }
}

impl<T> SegmentTreeTrait for SegmentTreeNode<T> {
    fn start(&self) -> isize {
        self.start
    }

    fn end(&self) -> isize {
        self.end
    }
}

impl<T> SegmentTreeTrait for SegmentNode<T> {
    fn start(&self) -> isize {
        match self {
            SegmentNode::Leaf(segment_leaf) => segment_leaf.start(),
            SegmentNode::Node(segment_node) => segment_node.start(),
        }
    }

    fn end(&self) -> isize {
        match self {
            SegmentNode::Leaf(segment_leaf) => segment_leaf.end(),
            SegmentNode::Node(segment_node) => segment_node.end(),
        }
    }
}

impl<T> SegmentNode<T> {
    fn update_range(&mut self) {
        match self {
            SegmentNode::Leaf(_) => {
                // segment_leaf.start = segment_leaf.start.min(segment_leaf.start());
                // segment_leaf.end = segment_leaf.end.max(segment_leaf.end());
            }
            SegmentNode::Node(segment_node) => {
                // Update the range of the leaf
                if let Some(left) = segment_node.left.as_ref() {
                    let left_borrowed = left.borrow();
                    segment_node.start = segment_node.start.min(left_borrowed.start());
                    segment_node.end = segment_node.end.max(left_borrowed.end());
                }

                if let Some(right) = segment_node.right.as_ref() {
                    let right_borrowed = right.borrow();
                    segment_node.start = segment_node.start.min(right_borrowed.start());
                    segment_node.end = segment_node.end.max(right_borrowed.end());
                }
            }
        }
    }
    pub fn insert(&mut self, range: SegmentLeaf<T>) {
        // if self.distance(&range) == 0 {
        //     return;
        // }

        if let SegmentNode::Leaf(leaf) = &self {
            let is_left = leaf.is_left_start(&range);

            let mut node = SegmentTreeNode {
                start: range.start.min(leaf.start),
                end: range.end.max(leaf.end),
                left: None,
                right: None,
            };

            let tree_leaf = Some(Box::new(RefCell::new(SegmentNode::Leaf(range))));

            if is_left {
                node.right = tree_leaf;
            } else {
                node.left = tree_leaf;
            }

            let leaf = mem::replace(self, SegmentNode::Node(node)).as_leaf();

            let tree_leaf = Some(Box::new(RefCell::new(SegmentNode::Leaf(leaf))));

            if let SegmentNode::Node(node) = self {
                if is_left {
                    node.left = tree_leaf;
                } else {
                    node.right = tree_leaf;
                }
            }

            self.update_range();

            return;
        }

        if let SegmentNode::Node(node) = self {
            //     a          b
            //     ^          ^
            //  1       2         3

            let mut index = 2;
            if let Some(left) = node.left.as_ref() {
                let left_borrowed = left.borrow_mut();

                if left_borrowed.is_left_start(&range) {
                    index = 2;
                } else {
                    index = 1
                }
            }

            if let Some(right) = node.right.as_ref() {
                let right_borrowed = right.borrow_mut();

                if right_borrowed.is_left_start(&range) {
                    index = 2;
                } else {
                    index = 3;
                }
            }

            if index == 2 {
                match (node.left.as_ref(), node.right.as_ref()) {
                    (None, _) => {
                        index = 1;
                    }
                    (Some(_), None) => {
                        index = 3;
                    }
                    (Some(left), Some(right)) => {
                        if left.borrow().is_overlay(&range) {
                            index = 1;
                        } else if right.borrow().is_overlay(&range) {
                            index = 3;
                        } else {
                            index = if left.borrow().distance(&range).abs()
                                < range.distance(&*right.borrow()).abs()
                            {
                                1
                            } else {
                                3
                            }
                        }
                    }
                }
            }

            match index {
                1 => {
                    // insert to left
                    if let Some(left) = node.left.as_mut() {
                        left.borrow_mut().insert(range);
                    } else {
                        node.left = Some(Box::new(RefCell::new(SegmentNode::Leaf(range))));
                    }
                }
                3 => {
                    // insert to right
                    if let Some(right) = node.right.as_mut() {
                        right.borrow_mut().insert(range);
                    } else {
                        node.right = Some(Box::new(RefCell::new(SegmentNode::Leaf(range))));
                    }
                }
                _ => {}
            }

            self.update_range();
        }
    }

    pub fn contain(&self, range: &SegmentRange) -> bool {
        if !self.is_overlay(range) {
            return false;
        }

        match self {
            SegmentNode::Leaf(segment_leaf) => segment_leaf.is_overlay(range),
            SegmentNode::Node(segment_node) => {
                if let Some(left) = segment_node.left.as_ref() {
                    if left.borrow().contain(range) {
                        return true;
                    }
                }

                if let Some(right) = segment_node.right.as_ref() {
                    if right.borrow().contain(range) {
                        return true;
                    }
                }

                false
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct SegmentTreeNode<T> {
    pub start: isize,
    pub end: isize,
    pub left: Option<Box<RefCell<SegmentNode<T>>>>,
    pub right: Option<Box<RefCell<SegmentNode<T>>>>,
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Read};

    use crate::optimize::gzip::main::{
        inflate, SegmentLeaf, SegmentNode, SegmentRangeTree, SegmentTreeNode,
    };

    #[test]
    fn t4() {
        let ranges = [
            (15, 18),
            (21, 59),
            (59, 62),
            (63, 67),
            (69, 109),
            (111, 116),
            (117, 120),
            (128, 172),
            (173, 431),
            (431, 689),
            (689, 870),
        ];
        let mut tree = SegmentRangeTree::<()>::new();

        ranges.into_iter().for_each(|v| {
            tree.insert(v.into());
        });

        assert!(tree.contain(&(112, 115).into()));
        assert_eq!(tree.contain(&(0, 10).into()), false);
        assert_eq!(tree.contain(&(5, 20).into()), true);
    }

    #[test]
    fn t3() {
        let v = fs::read("./src/optimize/gzip/main.rs").unwrap();

        let mut v = flate2::read::GzEncoder::new(&v[..], Default::default());

        let mut ret = vec![];
        v.read_to_end(&mut ret).unwrap();

        inflate(ret);
    }

    #[test]
    fn t2() {
        let mut v = SegmentNode::<()>::Node(SegmentTreeNode::default());

        v.insert(SegmentLeaf {
            start: 1,
            end: 3,
            data: None,
        });

        v.insert(SegmentLeaf {
            start: 8,
            end: 10,
            data: None,
        });

        v.insert(SegmentLeaf {
            start: 11,
            end: 13,
            data: None,
        });

        assert!(v.contain(&(2, 3).into()));

        assert_eq!(v.contain(&(0, 1).into()), true);
        assert_eq!(v.contain(&(4, 5).into()), false);
        assert_eq!(v.contain(&(7, 9).into()), true);
        assert_eq!(v.contain(&(-10, 200).into()), true);
    }
}
