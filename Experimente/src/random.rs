pub(crate) fn next_random(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

pub(crate) fn shuffle<T>(items: &mut [T], state: &mut u64) {
    for index in (1..items.len()).rev() {
        let swap = (next_random(state) % (index + 1) as u64) as usize;
        items.swap(index, swap);
    }
}
