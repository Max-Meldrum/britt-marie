use criterion::{criterion_group, criterion_main, Bencher, Criterion, Throughput};
use fxhash::FxHashMap;
use rand::Rng;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

const INSERT_COUNT: u64 = 1000;

fn insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash");
    group.throughput(Throughput::Elements(INSERT_COUNT));
    group.bench_function("random range insert", random_range_insert);
    group.finish()
}

fn random_range_insert(b: &mut Bencher) {
    let mut hash_map = FxHashMap::default();
    let mut rng = rand::thread_rng();
    b.iter(|| {
        for _i in 0..INSERT_COUNT {
            let id = rng.gen_range(0, 10000);
            hash_map.insert(id, 1000);
        }
    });
}

criterion_group!(benches, insert);
criterion_main!(benches);
