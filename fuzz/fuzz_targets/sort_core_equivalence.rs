#![no_main]

use libfuzzer_sys::fuzz_target;

fn insertion_sort(values: &mut [i64]) {
    let mut i = 1usize;
    while i < values.len() {
        let mut j = i;
        while j > 0 && values[j - 1] > values[j] {
            values.swap(j - 1, j);
            j -= 1;
        }
        i += 1;
    }
}

fn decode_i64_values(data: &[u8]) -> Vec<i64> {
    data.chunks_exact(8)
        .take(256)
        .map(|chunk| {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(chunk);
            i64::from_le_bytes(bytes)
        })
        .collect()
}

fuzz_target!(|data: &[u8]| {
    let mut candidate = decode_i64_values(data);
    let mut expected = candidate.clone();

    insertion_sort(&mut candidate);
    expected.sort();

    assert_eq!(candidate, expected);
});
