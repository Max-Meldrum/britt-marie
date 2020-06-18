use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};
#[cfg(feature = "nightly")]
use packed_simd::u32x16;

const VALUES: [u32; 16] = [
    131123, 321321, 10, 13213, 13902, 2348, 32131, 8, 9999, 100101, 1130213, 120, 10103, 140, 150,
    8816,
];

fn sum(c: &mut Criterion) {
    let mut group = c.benchmark_group("sum");
    group.bench_function("normal sum", normal_sum);
    #[cfg(feature = "nightly")]
    group.bench_function("simd sum", simd_sum);
}

fn normal_sum(b: &mut Bencher) {
    b.iter(|| {
        // Put in a black box as the compiler will probably
        // figure out that it can calculate the sum at compile time...
        //let sum: u32 = black_box(VALUES).iter().sum();
        let sum: u32 = black_box(VALUES).iter().sum();
        assert_eq!(sum, 1773698);
    });
}

#[cfg(feature = "nightly")]
fn simd_sum(b: &mut Bencher) {
    b.iter(|| {
        let arr = u32x16::from_slice_unaligned(&black_box(VALUES));
        let sum: u32 = arr.wrapping_sum();
        assert_eq!(sum, 1773698);
    });
}

criterion_group!(benches, sum);
criterion_main!(benches);
