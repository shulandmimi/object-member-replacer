use std::{
    cell::{RefCell, RefMut},
    mem,
    rc::Rc,
};

use rustc_hash::FxHashMap;

use crate::optimize::gzip::{
    git_bits::{get_bit, get_bits},
    util::{parse_distance_code, parse_length_code},
};

#[derive(Debug)]
enum TokenDetail {
    Repeat {
        length: usize,
        lengthBits: usize,
        distance: usize,
        distanceBits: usize,
    },
    Literal {
        bits: usize,
    },
}

#[derive(Debug)]
struct Position {
    start: usize,
    end: usize,
    reference: Vec<(usize, usize)>,
}

#[derive(Debug)]
struct InflateToken {
    value: usize,
    bits: usize,
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

enum Formate {
    Gzip,
    Zlib,
}

struct TreeSet {
    litCodeLengths: Vec<usize>,
    litCodeTree: FxHashMap<usize, usize>,
    distCodeLengths: Vec<usize>,
    distCodeTree: FxHashMap<usize, usize>,
}

const CODE_CODE_LENGTH_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

fn buildHuffmanTree(code_lengths: &[usize]) -> FxHashMap<usize, usize> {
    let max_bits = code_lengths.iter().fold(0, |prev, cur| prev.max(*cur));

    let mut code = 0;
    let mut bl_count = vec![0; max_bits + 1];
    let mut next_code = vec![0; max_bits + 1];

    for len in code_lengths {
        if *len > 1 {
            bl_count[len - 1] += 1;
        }
    }

    // code = (0 + 0) << 1
    for i_bit in 1..max_bits {
        code = code + bl_count[i_bit - 1] << 1;
        let v: usize = bl_count[i_bit - 1] << 1;
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

fn decodeHuffman(
    array: &[u8],
    head_box: &mut [usize],
    code_lengths: &[usize],
    tree: &FxHashMap<usize, usize>,
) -> usize {
    let mut head = head_box[0];

    let mut alpha = 0;
    let mut length = 0;
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

fn parseDynamicHuffmanTree(array: &[u8], head_box: &mut [usize]) -> TreeSet {
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
    let code_code_tree = buildHuffmanTree(&code_code_lengths);

    // parse literal and length + distance code lengths
    let mut lit_dist_code_lengths = vec![0; lit_codes + dist_codes];

    let mut prev_length = 0;

    let mut i_code = 0;
    while i_code < lit_codes + dist_codes {
        let value = decodeHuffman(array, head_box, &code_code_lengths, &code_code_tree);

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
    let lit_code_tree = buildHuffmanTree(&lit_dist_code_lengths);
    let dist_code_tree = buildHuffmanTree(&dist_code_lengths);

    return TreeSet {
        litCodeLengths: lit_dist_code_lengths,
        litCodeTree: lit_code_tree,
        distCodeLengths: dist_code_lengths,
        distCodeTree: dist_code_tree,
    };
}

fn process_block_using_tree(
    array: &[u8],
    head_box: &mut [usize],
    raw: &mut Vec<usize>,
    TreeSet {
        litCodeLengths,
        litCodeTree,
        distCodeLengths,
        distCodeTree,
    }: TreeSet,
) -> (Vec<InflateToken>, Vec<Position>) {
    let mut tokens: Vec<InflateToken> = vec![];
    let mut ranges = vec![];

    loop {
        let head_before_alpha = head_box[0];
        let value = decodeHuffman(array, head_box, &litCodeLengths, &litCodeTree);

        if value == 256 {
            // end-of-block
            break;
        } else if value > 256 {
            // `match`
            let length = parse_length_code(array, head_box, value);
            let head_before_dist = head_box[0];
            let length_bits = head_before_dist - head_before_alpha;

            let dist_code = decodeHuffman(array, head_box, &distCodeLengths, &distCodeTree);
            let distance = parse_distance_code(array, head_box, dist_code);
            let distance_bits = head_box[0] - head_before_dist;

            let start = raw.len() - distance;
            let end = (start + length).min(raw.len());
            // let reference = [start, ]
            let sliced: Vec<usize> = (&raw[start..(start + length).min(raw.len())])
                .iter()
                .cloned()
                .collect();

            let concat = if sliced.len() < length {
                // concat
                let v = vec![0; length]
                    .into_iter()
                    .enumerate()
                    .map(|(_, i)| sliced[i % sliced.len()])
                    .collect::<Vec<_>>();
                v
            } else {
                sliced
            };

            let reference = if end < (start + length) {
                let end2 = length % (end - start);
                vec![(start, end), (start, start + end2)]
            } else {
                vec![(start, end)]
            };

            ranges.push(Position {
                start: raw.len(),
                end: raw.len() + concat.len(),
                reference,
            });
            raw.extend(concat.clone());

            concat.into_iter().for_each(|value| {
                tokens.push(InflateToken {
                    value,
                    bits: ((length_bits + distance_bits) as usize) / length,
                    details: TokenDetail::Repeat {
                        length,
                        lengthBits: length_bits as usize,
                        distance,
                        distanceBits: distance_bits as usize,
                    },
                })
            });
        } else {
            // raw.push(value);
            raw.push(value);

            let bits = head_box[0] - head_before_alpha;

            tokens.push(InflateToken {
                value,
                bits: bits as usize,
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
        let tree_set = parseDynamicHuffmanTree(array, head_box);
        let (result, ranges) = process_block_using_tree(array, head_box, raw, tree_set);

        return ParsedBlock {
            tokens: result,
            is_final_block,
            ranges,
        };
    }
}

fn inflate(array: Vec<u8>) {
    let mut head_box: Vec<usize> = vec![0; 1024];

    // const headBox: [number] = [0];
    // const tokens: InflateToken[] = [];
    // const raw: number[] = [];

    let mut result: Vec<InflateToken> = vec![];
    let mut raw: Vec<usize> = vec![];
    let mut range_list = vec![];
    let mut formater: Option<Formate> = None;

    if check_gzip(&array, &mut head_box) {
        formater = Some(Formate::Gzip);
        // } else if check_gzip(&array, &mut head_box) {
        //     formater = Some(Formate::Zlib);
        // } else {
        //     println!("Not a gzip or zlib file");
        //     return;
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

    for token in result {
        content.push(token.value as u8 as char);
    }

    for range in range_list {
        println!(
            "range: {:#?}\nsource: {:#?}",
            range,
            content[range.start..range.end].to_string()
        );
    }
    // println!("ranges: {:#?}", range_list);

    println!(
        "content: {}\nraw: {}",
        content,
        String::from_utf8_lossy(&raw.iter().map(|v| *v as u8).collect::<Vec<u8>>()).to_string()
    );
}

enum SegmentTree {
    Leaf(SegmentLeaf),
    Node(SegmentNode),
}

impl SegmentTree {
    fn as_leaf(self) -> SegmentLeaf {
        match self {
            SegmentTree::Leaf(segment_leaf) => segment_leaf,
            SegmentTree::Node(_) => panic!("Cannot convert Node to Leaf"),
        }
    }
}

struct SegmentLeaf {
    start: usize,
    end: usize,
}

trait SegmentTreeTrait {
    // fn insert(&self, range: (usize, usize));
    // fn distance(&self, other: &Self) -> usize;
    fn start(&self) -> usize;
    fn end(&self) -> usize;

    fn distance(&self, other: &dyn SegmentTreeTrait) -> usize {
        // if overflay, return 0
        if self.start() >= other.end() || self.end() <= other.start() {
            let start = self.start().max(other.start());
            let end = self.end().min(other.end());

            end - start
        } else {
            0
        }
    }

    fn is_left(&self, other: &dyn SegmentTreeTrait) -> bool {
        self.end() <= other.start()
    }
}

impl SegmentTreeTrait for SegmentLeaf {
    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}

impl SegmentTreeTrait for SegmentNode {
    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}

impl SegmentTreeTrait for SegmentTree {
    fn start(&self) -> usize {
        match self {
            SegmentTree::Leaf(segment_leaf) => segment_leaf.start(),
            SegmentTree::Node(segment_node) => segment_node.start(),
        }
    }

    fn end(&self) -> usize {
        match self {
            SegmentTree::Leaf(segment_leaf) => segment_leaf.end(),
            SegmentTree::Node(segment_node) => segment_node.end(),
        }
    }
}

// impl <T: SegmentTreeTrait> T {
//     fn distance(&self, other: &T) -> usize {
//         // if overflay, return 0
//         if self.start() >= other.end() || self.end() <= other.start() {
//             return 0;
//         } else {
//             let start = self.start().max(other.start());
//             let end = self.end().min(other.end());

//             end - start
//         }
//     }
// }

impl SegmentTree {
    fn insert(&mut self, range: SegmentLeaf) {
        if self.distance(&range) == 0 {
            return;
        }
        let is_left = self.is_left(&range);

        // if let SegmentTree::Leaf(leaf) = self {
        //     *self = SegmentTree::Node(SegmentNode {
        //         start: range.start,
        //         end: range.end,
        //         left: Some(Box::new(RefCell::new(SegmentNode {
        //             start: range.start,
        //             end: (range.start + range.end) / 2,
        //             left: None,
        //             right: None,
        //         }))),
        //         right: None,
        //     });
        // }
        if let SegmentTree::Leaf(leaf) = &self {
            let is_left = leaf.is_left(&range);
            let mut node = SegmentNode {
                start: range.start.min(leaf.start),
                end: range.end.max(leaf.end),
                left: None,
                right: None,
            };

            let tree_leaf = Some(Box::new(RefCell::new(SegmentTree::Leaf(range))));
            if is_left {
                node.right = tree_leaf;
            } else {
                node.left = tree_leaf;
            }

            drop(leaf);

            let leaf = mem::replace(self, SegmentTree::Node(node)).as_leaf();

            let tree_leaf = Some(Box::new(RefCell::new(SegmentTree::Leaf(leaf))));
            // if is_left {

            // } else {
            //     if let SegmentTree::Node(node) = self {
            //         node.right = tree_leaf;
            //     }
            // }
            return;
            // node.left;
        }
        if matches!(self, SegmentTree::Leaf(_)) {

            // if leaf.is_left(&range) ;
        }

        // match self {
        //     // (5, 8) insert (2, 4) -> (2, 4) (5, 8)
        //     // (2, 4) insert (5, 8) -> (2, 4) (5, 8)
        //     SegmentTree::Leaf(segment_leaf) => {}
        //     SegmentTree::Node(segment_node) => {
        //         // let mut distance = 0;
        //         if let Some(left) = segment_node.left.as_ref() {
        //             // left.borrow()
        //         }

        //         if let Some(right) = segment_node.right.as_ref() {}
        //     }
        // }
    }
}

struct SegmentNode {
    start: usize,
    end: usize,
    // value: usize,
    left: Option<Box<RefCell<SegmentTree>>>,
    right: Option<Box<RefCell<SegmentTree>>>,
}

impl SegmentNode {
    fn insert(&self, range: (usize, usize)) {}

    fn find(&self, (start, end): (usize, usize)) {
        // out current range
        //          ----
        //     ^end^    ^start^
        let out_range = start >= self.end || end <= self.start;

        if out_range {
            return;
        }

        if let Some(left) = &self.left {
            left.borrow().find((start, end));
        }

        if let Some(right) = &self.right {
            right.borrow().find((start, end));
        }
    }
}

/// (1, 3), (8, 10)
///     (1, 10)
/// (1, 3)    (8, 10)
///
/// insert 4,5
///

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::optimize::gzip::main::inflate;

    #[test]
    fn t1() {
        let result = fs::read("./hello.gz").unwrap();

        // println!("result: {:#?}", result);
        inflate(result);
    }
}
