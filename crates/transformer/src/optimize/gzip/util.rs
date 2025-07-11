// import { getBits } from './getBits';

use crate::optimize::gzip::git_bits::get_bits;

const LENGTH_BASE: [usize; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];

const LENGTH_EXTRA_BITS: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];

pub fn parse_length_code(array: &[u8], head_box: &mut [usize], length_code: usize) -> usize {
    let base = LENGTH_BASE[length_code - 257];
    let extra = LENGTH_EXTRA_BITS[length_code - 257];

    return (base + get_bits(array, head_box, extra as usize)) as usize;
}

const DISTANCE_BASE: [usize; 30] = [
  1, 2, 3, 4, 5, 7, 9, 13, 17, 25,
  33, 49, 65, 97, 129, 193, 257, 385, 513, 769,
  1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];

const DISTANCE_EXTRA_BITS: [usize; 30] = [
  0, 0, 0, 0, 1, 1, 2, 2, 3, 3,
  4, 4, 5, 5, 6, 6, 7, 7, 8, 8,
  9, 9, 10, 10, 11, 11, 12, 12, 13, 13,
];

pub fn parse_distance_code(
  array: &[u8],
  head_box: &mut [usize],
  distance_code: usize,
) -> usize {
  let base = DISTANCE_BASE[ distance_code ];
  let extra = DISTANCE_EXTRA_BITS[ distance_code ];

  return base + get_bits( array, head_box, extra );
}
