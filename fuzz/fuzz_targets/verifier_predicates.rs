#![no_main]

use libfuzzer_sys::fuzz_target;
use std::collections::BTreeMap;

fn is_non_decreasing(values: &[i64]) -> bool {
    values.windows(2).all(|window| window[0] <= window[1])
}

fn same_multiset(input: &[i64], output: &[i64]) -> bool {
    let mut in_counts: BTreeMap<i64, u64> = BTreeMap::new();
    let mut out_counts: BTreeMap<i64, u64> = BTreeMap::new();

    for value in input {
        *in_counts.entry(*value).or_insert(0) += 1;
    }
    for value in output {
        *out_counts.entry(*value).or_insert(0) += 1;
    }

    in_counts == out_counts
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
    let values = decode_i64_values(data);

    let mut sorted = values.clone();
    sorted.sort();

    assert!(is_non_decreasing(&sorted));
    assert!(same_multiset(&values, &sorted));

    if let Some(first) = sorted.first().copied() {
        let mut changed = sorted.clone();
        changed.push(first.wrapping_add(1));
        assert!(!same_multiset(&values, &changed));
    }
});
