
pub fn get_bit(array: &[u8], bit: usize) -> usize {
    let offset = bit >> 3;
    if offset >= array.len() {
        return 0;
    }
    (array[offset] as usize) >> (bit & 7) & 1
}

pub fn get_bits(array: &[u8], head_box: &mut [usize], len: usize) -> usize {
    let mut head = head_box[0];
    let mut value = 0;

    for i in 0..len {
        value += get_bit(array, head) << i;
        head += 1;
    }

    head_box[0] = head;

    return value;
}
