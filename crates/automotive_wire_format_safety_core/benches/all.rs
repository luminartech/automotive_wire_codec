use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

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

fn make_random(n: usize) -> Vec<i64> {
    // Deterministic LCG so benchmarks are reproducible across runs.
    let mut v = Vec::with_capacity(n);
    let mut x: u64 = 0xDEAD_BEEF_CAFE_BABE;
    for _ in 0..n {
        x = x
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        v.push((x >> 32).cast_signed());
    }
    v
}

fn bench_sort_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("insertion_sort/random");
    for size in [8usize, 64, 256, 1_024] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &n| {
            let input = make_random(n);
            b.iter(|| {
                let mut data = black_box(input.clone());
                insertion_sort(&mut data);
                data
            });
        });
    }
    group.finish();
}

fn bench_sort_worst_case(c: &mut Criterion) {
    let mut group = c.benchmark_group("insertion_sort/worst_case");
    for size in [8usize, 64, 256] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &n| {
            // Reverse-sorted is worst case for insertion sort (O(n²) swaps).
            let input: Vec<i64> = (0..i64::try_from(n).unwrap()).rev().collect();
            b.iter(|| {
                let mut data = black_box(input.clone());
                insertion_sort(&mut data);
                data
            });
        });
    }
    group.finish();
}

fn bench_sort_best_case(c: &mut Criterion) {
    let mut group = c.benchmark_group("insertion_sort/best_case");
    for size in [8usize, 64, 256, 1_024] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &n| {
            // Already-sorted is best case (O(n) comparisons, zero swaps).
            let input: Vec<i64> = (0..i64::try_from(n).unwrap()).collect();
            b.iter(|| {
                let mut data = black_box(input.clone());
                insertion_sort(&mut data);
                data
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_sort_by_size,
    bench_sort_worst_case,
    bench_sort_best_case
);
criterion_main!(benches);
