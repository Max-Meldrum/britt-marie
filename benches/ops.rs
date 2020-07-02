use criterion::{criterion_group, criterion_main, Bencher, Criterion, Throughput};
use fxhash::FxHashMap;
use once_cell::sync::Lazy;
use rand::Rng;
use std::collections::BTreeMap;
use std::{collections::hash_map::HashMap, hash::Hash};

use britt_marie::index::hash::HashIndex;
use britt_marie::index::HashOps;
use britt_marie::raw_store::RawStore;
use std::cell::RefCell;
use std::rc::Rc;

const INSERT_COUNT: u64 = 1000;

static RANDOM_INDEXES: Lazy<Vec<u64>> = Lazy::new(|| {
    let mut rng = rand::thread_rng();
    let mut indexes = Vec::with_capacity(INSERT_COUNT as usize);
    for _i in 0..INSERT_COUNT {
        indexes.push(rng.gen_range(0, INSERT_COUNT));
    }
    indexes
});

fn insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");
    group.throughput(Throughput::Elements(INSERT_COUNT));
    group.bench_function("random insert hash", random_range_insert_hash);
    group.bench_function(
        "random insert britt-marie::HashIndex",
        random_range_insert_hash_index,
    );
    group.bench_function("random insert btreemap", random_range_insert_btreemap);

    group.bench_function("ordered insert hash", ordered_insert_hash);
    group.bench_function(
        "ordered insert britt-marie::HashIndex",
        ordered_insert_hash_index,
    );
    group.bench_function("ordered insert btreemap", ordered_insert_btreemap);

    group.bench_function("ordered rmw hash", rmw_ordered_hash);
    group.bench_function("ordered rmw btreemap", rmw_ordered_btreemap);
    group.bench_function("ordered rmw britt-marie::HashIndex", rmw_ordered_hash_index);

    group.bench_function("random rmw hash", rmw_random_hash);
    group.bench_function("random rmw britt-marie::HashIndex", rmw_random_hash_index);
    group.bench_function("random rmw btreemap", rmw_random_btreemap);
    group.finish()
}

// RANDOM INSERT (IDS: 0-1000)

fn random_range_insert_hash(b: &mut Bencher) {
    let mut hash_map =
        HashMap::with_capacity_and_hasher(INSERT_COUNT as usize, fxhash::FxBuildHasher::default());
    b.iter(|| {
        for id in RANDOM_INDEXES.iter() {
            hash_map.insert(id, 1000);
        }
    });
}
fn random_range_insert_hash_index(b: &mut Bencher) {
    let raw_store = Rc::new(RefCell::new(RawStore::new("/tmp/bench")));
    let mut hash_index: HashIndex<u64, u64> = HashIndex::new(INSERT_COUNT as usize, raw_store);
    b.iter(|| {
        for id in RANDOM_INDEXES.iter() {
            hash_index.put(*id, 1000);
        }
    });
}

fn random_range_insert_btreemap(b: &mut Bencher) {
    let mut map = BTreeMap::new();
    b.iter(|| {
        for id in RANDOM_INDEXES.iter() {
            map.insert(id, 1000);
        }
    });
}

// ORDERED INSERT

fn ordered_insert_hash(b: &mut Bencher) {
    let mut hash_map = FxHashMap::default();
    b.iter(|| {
        for i in 0..INSERT_COUNT {
            hash_map.insert(i, 1000);
        }
    });
}
fn ordered_insert_hash_index(b: &mut Bencher) {
    let raw_store = Rc::new(RefCell::new(RawStore::new("/tmp/ordered")));
    let mut hash_index: HashIndex<u64, u64> = HashIndex::new(INSERT_COUNT as usize, raw_store);
    b.iter(|| {
        for i in 0..INSERT_COUNT {
            hash_index.put(i, 1000);
        }
    });
}

fn ordered_insert_btreemap(b: &mut Bencher) {
    let mut map = BTreeMap::new();
    b.iter(|| {
        for i in 0..INSERT_COUNT {
            map.insert(i, 1000);
        }
    });
}

// RMW ORDERED

fn rmw_ordered_hash(b: &mut Bencher) {
    let mut hash_map = FxHashMap::default();
    for i in 0..INSERT_COUNT {
        hash_map.insert(i, 1000);
    }
    b.iter(|| {
        for i in 0..INSERT_COUNT {
            if let Some(val) = hash_map.get_mut(&i) {
                *val += 10;
            }
        }
    });
}
fn rmw_ordered_hash_index(b: &mut Bencher) {
    let raw_store = Rc::new(RefCell::new(RawStore::new("/tmp/rmw_ordered")));
    let mut hash_index: HashIndex<u64, u64> = HashIndex::new(INSERT_COUNT as usize, raw_store);
    for i in 0..INSERT_COUNT {
        hash_index.put(i, 1000);
    }
    b.iter(|| {
        for i in 0..INSERT_COUNT {
            hash_index.rmw(&i, |val| {
                *val += 10;
            });
        }
    });
}

fn rmw_ordered_btreemap(b: &mut Bencher) {
    let mut map = BTreeMap::new();
    for i in 0..INSERT_COUNT {
        map.insert(i, 1000);
    }
    b.iter(|| {
        for i in 0..INSERT_COUNT {
            if let Some(val) = map.get_mut(&i) {
                *val += 10;
            }
        }
    });
}

// RMW RANDOM

fn rmw_random_hash(b: &mut Bencher) {
    let mut hash_map = FxHashMap::default();
    for i in 0..INSERT_COUNT {
        hash_map.insert(i, 1000);
    }
    b.iter(|| {
        for id in RANDOM_INDEXES.iter() {
            if let Some(val) = hash_map.get_mut(&id) {
                *val += 10;
            }
        }
    });
}

fn rmw_random_hash_index(b: &mut Bencher) {
    let raw_store = Rc::new(RefCell::new(RawStore::new("/tmp/rmw_ordered")));
    let mut hash_index: HashIndex<u64, u64> = HashIndex::new(INSERT_COUNT as usize, raw_store);
    for i in 0..INSERT_COUNT {
        hash_index.put(i, 1000);
    }
    b.iter(|| {
        for id in RANDOM_INDEXES.iter() {
            hash_index.rmw(&id, |val| {
                *val += 10;
            });
        }
    });
}

fn rmw_random_btreemap(b: &mut Bencher) {
    let mut map = BTreeMap::new();
    for i in 0..INSERT_COUNT {
        map.insert(i, 1000);
    }
    b.iter(|| {
        for id in RANDOM_INDEXES.iter() {
            if let Some(val) = map.get_mut(&id) {
                *val += 10;
            }
        }
    });
}
criterion_group!(benches, insert);
criterion_main!(benches);
