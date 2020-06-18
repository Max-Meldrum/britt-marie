use criterion::{criterion_group, criterion_main, Bencher, BenchmarkId, Criterion};

#[cfg(target_arch = "x86")]
use core::arch::x86;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64 as x86;
#[cfg(feature = "nightly")]
use packed_simd::u8x16;

const TARGET_ARR: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
const TARGET_KEYS: [u8; 4] = [1, 6, 12, 16];
const EXPECTED_POSITIONS: [usize; 4] = [0, 5, 11, 15];

fn search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");

    for input in TARGET_KEYS.iter().zip(EXPECTED_POSITIONS.iter()) {
        let (key, pos) = input;
        group.bench_with_input(BenchmarkId::new("LINEAR", key), key, |b, &key| {
            linear_search(b, &key, *pos)
        });

        #[cfg(all(
            feature = "nightly",
            target_feature = "sse2",
            any(target_arch = "x86", target_arch = "x86_64"),
        ))]
        group.bench_with_input(BenchmarkId::new("LINEAR SIMD", key), key, |b, &key| {
            linear_search_simd(b, &key, *pos)
        });

        group.bench_with_input(BenchmarkId::new("BINARYSEARCH", key), key, |b, &key| {
            binary_search(b, &key, *pos)
        });
    }

    group.finish();
}

fn linear_search(b: &mut Bencher, key: &u8, pos: usize) {
    b.iter(|| assert_eq!(Some(pos), lin_search(&TARGET_ARR, *key)));
}

#[inline(always)]
fn lin_search(arr: &[u8], key: u8) -> Option<usize> {
    for i in 0..arr.len() {
        if arr[i] == key {
            return Some(i);
        }
    }
    return None;
}

fn binary_search(b: &mut Bencher, key: &u8, pos: usize) {
    b.iter(|| assert_eq!(Some(pos), binsearch(key, &TARGET_ARR)));
}

#[inline(always)]
fn binsearch<T: PartialOrd>(target: &T, collection: &[T]) -> Option<usize> {
    let mut lo: usize = 0;
    let mut hi: usize = collection.len();

    while lo < hi {
        let m: usize = (hi - lo) / 2 + lo;

        if *target == collection[m] {
            return Some(m);
        } else if *target < collection[m] {
            hi = m;
        } else {
            lo = m + 1;
        }
    }
    return None;
}

#[cfg(all(
    feature = "nightly",
    target_feature = "sse2",
    any(target_arch = "x86", target_arch = "x86_64"),
))]
fn linear_search_simd(b: &mut Bencher, key: &u8, pos: usize) {
    b.iter(|| assert_eq!(Some(pos), lin_simd(&TARGET_ARR, *key)));
}

#[cfg(all(
    feature = "nightly",
    target_feature = "sse2",
    any(target_arch = "x86", target_arch = "x86_64"),
))]
#[inline(always)]
fn lin_simd(arr: &[u8], key: u8) -> Option<usize> {
    let ks = u8x16::from_slice_unaligned(&arr);
    // In this case we are searching the whole 16 byte slice,
    // but 16 can replaced with the actual amount of "values" inside
    let mask = (1 << (16 as usize)) - 1;
    let d_splat = u8x16::splat(key);
    let comps = d_splat.eq(ks);
    let bits = unsafe { x86::_mm_movemask_epi8(std::mem::transmute(comps)) } & mask;
    if bits == 0 {
        return None;
    } else {
        let target = bits.trailing_zeros() as usize;
        return Some(target);
    }
}

criterion_group!(benches, search);
criterion_main!(benches);
