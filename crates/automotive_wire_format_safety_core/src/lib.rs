#![forbid(unsafe_code)]

use automotive_wire_format_safety_types::{NumberList, SortedList};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortError {
    TooLong,
}

/// Sorts a list of numbers using insertion sort. Returns an error if the input list is too long.
///
/// # Errors
/// - `SortError::TooLong` if the input list exceeds `automotive_wire_format_safety_types::MAX_LEN`.
pub fn sort(input: NumberList) -> Result<SortedList, SortError> {
    let mut data = input.values;
    if data.len() > automotive_wire_format_safety_types::MAX_LEN {
        return Err(SortError::TooLong);
    }

    insertion_sort(&mut data);
    Ok(SortedList { values: data })
}

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

#[cfg(test)]
mod tests {
    use super::insertion_sort;
    #[cfg(not(miri))]
    use proptest::prelude::*;

    #[test]
    fn insertion_sort_orders_values() {
        let mut values = [5, 2, 9, 1, 5, 6];
        insertion_sort(&mut values);
        assert_eq!(values, [1, 2, 5, 5, 6, 9]);
    }

    #[test]
    fn insertion_sort_handles_empty_and_singleton() {
        let mut empty: [i64; 0] = [];
        insertion_sort(&mut empty);
        assert!(empty.is_empty());

        let mut singleton = [42];
        insertion_sort(&mut singleton);
        assert_eq!(singleton, [42]);
    }

    #[test]
    fn insertion_sort_preserves_duplicates() {
        let mut values = [3, 1, 3, 2, 2, 1];
        insertion_sort(&mut values);
        assert_eq!(values, [1, 1, 2, 2, 3, 3]);
    }

    #[cfg(miri)]
    #[test]
    fn miri_insertion_sort_idempotent_after_sort() {
        let mut values = [8, -2, 4, 4, 0, 3, -2];
        insertion_sort(&mut values);
        let once_sorted = values;
        insertion_sort(&mut values);
        assert_eq!(values, once_sorted);
    }

    #[cfg(not(miri))]
    proptest! {
        #[test]
        fn prop_insertion_sort_matches_std_sort(mut values in proptest::collection::vec(any::<i64>(), 0..128)) {
            let mut expected = values.clone();
            insertion_sort(&mut values);
            expected.sort_unstable();
            prop_assert_eq!(values, expected);
        }

        #[test]
        fn prop_insertion_sort_output_is_non_decreasing(mut values in proptest::collection::vec(any::<i64>(), 0..128)) {
            insertion_sort(&mut values);
            let is_sorted = values.windows(2).all(|window| window[0] <= window[1]);
            prop_assert!(is_sorted);
        }
    }
}
