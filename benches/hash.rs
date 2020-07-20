use criterion::{criterion_group, criterion_main, Bencher, BenchmarkId, Criterion, Throughput};
use itertools::Itertools;
use once_cell::sync::Lazy;
use rand::Rng;
use tempfile::tempdir;

use britt_marie::{HashIndex, HashOps, RawStore};
use std::cell::RefCell;
use std::rc::Rc;

const MOD_FACTORS: [f32; 3] = [0.3, 0.5, 0.8];
const CAPACITY: [usize; 3] = [512, 4096, 10024];
const TOTAL_KEYS: u64 = 10000;
const TOTAL_OPERATIONS: u64 = 1000;

static RANDOM_INDEXES: Lazy<Vec<u64>> = Lazy::new(|| {
    let mut rng = rand::thread_rng();
    let mut indexes = Vec::with_capacity(TOTAL_OPERATIONS as usize);
    for _i in 0..TOTAL_OPERATIONS {
        indexes.push(rng.gen_range(0, TOTAL_KEYS));
    }
    indexes
});

#[derive(prost::Message, Clone)]
pub struct SmallStruct {
    #[prost(int64, tag = "1")]
    pub x1: i64,
    #[prost(uint32, tag = "2")]
    pub x2: u32,
    #[prost(double, tag = "3")]
    pub x3: f64,
}

impl SmallStruct {
    pub fn new() -> SmallStruct {
        SmallStruct {
            x1: 100,
            x2: 500,
            x3: 1000.0,
        }
    }
}

#[derive(prost::Message, Clone)]
pub struct LargeStruct {
    #[prost(int64, tag = "1")]
    pub x1: i64,
    #[prost(uint32, tag = "2")]
    pub x2: u32,
    #[prost(double, tag = "3")]
    pub x3: f64,
    #[prost(int64, repeated, tag = "4")]
    pub x4: Vec<i64>,
    #[prost(uint64, repeated, tag = "5")]
    pub x5: Vec<u64>,
    #[prost(double, repeated, tag = "6")]
    pub x6: Vec<f64>,
}

impl LargeStruct {
    pub fn new() -> LargeStruct {
        LargeStruct {
            x1: 50,
            x2: 1000,
            x3: 500.0,
            x4: vec![200, 300, 1000, 5000, 200, 350, 100],
            x5: vec![20, 50, 100, 20, 40, 100, 900, 100],
            x6: vec![150.0, 500.1, 35.5, 20.5, 40.9, 80.5, 400.5, 350.0],
        }
    }
}

fn hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash");
    group.throughput(Throughput::Elements(TOTAL_OPERATIONS));

    for input in MOD_FACTORS.iter().cartesian_product(CAPACITY.iter()) {
        let (mod_factor, capacity) = input;
        let description = format!("mod_factor: {}, capacity: {}", mod_factor, capacity);

        group.bench_with_input(
            BenchmarkId::new("Random Get SmallStruct", description.clone()),
            &(mod_factor, capacity),
            |b, (&mod_factor, &capacity)| random_get_small(b, capacity, mod_factor),
        );
        group.bench_with_input(
            BenchmarkId::new("Random Get LargeStruct", description.clone()),
            &(mod_factor, capacity),
            |b, (&mod_factor, &capacity)| random_get_large(b, capacity, mod_factor),
        );

        group.bench_with_input(
            BenchmarkId::new("Insert SmallStruct", description.clone()),
            &(mod_factor, capacity),
            |b, (&mod_factor, &capacity)| insert_small(b, capacity, mod_factor),
        );

        group.bench_with_input(
            BenchmarkId::new("Insert LargeStruct", description.clone()),
            &(mod_factor, capacity),
            |b, (&mod_factor, &capacity)| insert_large(b, capacity, mod_factor),
        );

        group.bench_with_input(
            BenchmarkId::new("RMW SmallStruct", description.clone()),
            &(mod_factor, capacity),
            |b, (&mod_factor, &capacity)| rmw_small(b, capacity, mod_factor),
        );
        group.bench_with_input(
            BenchmarkId::new("RMW LargeStruct", description.clone()),
            &(mod_factor, capacity),
            |b, (&mod_factor, &capacity)| rmw_large(b, capacity, mod_factor),
        );
    }
    group.bench_function("Random Get Small RawStore", raw_store_random_small_get);
    group.bench_function("Random Get Large RawStore", raw_store_random_large_get);
    group.bench_function("Insert SmallStruct RawStore", insert_raw_store_small);
    group.bench_function("Insert LargeStruct RawStore", insert_raw_store_large);
    // TODO: merge operator
    group.bench_function("RMW SmallStruct RawStore", rmw_raw_store_small);
    group.bench_function("RMW LargeStruct RawStore", rmw_raw_store_large);

    group.finish()
}

fn insert_small(b: &mut Bencher, capacity: usize, mod_factor: f32) {
    let temp_dir = tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap();
    let raw_store = Rc::new(RefCell::new(RawStore::new(path)));
    let mut hash_index: HashIndex<u64, SmallStruct> =
        HashIndex::new(capacity, mod_factor, raw_store.clone());

    b.iter(|| {
        for id in RANDOM_INDEXES.iter() {
            hash_index.put(*id, SmallStruct::new());
        }
    });
}

fn insert_raw_store_small(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap();
    let mut raw_store = RawStore::new(path);

    b.iter(|| {
        for id in RANDOM_INDEXES.iter() {
            let _ = raw_store.put(&*id, &SmallStruct::new());
        }
    });
}

fn insert_large(b: &mut Bencher, capacity: usize, mod_factor: f32) {
    let temp_dir = tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap();
    let raw_store = Rc::new(RefCell::new(RawStore::new(path)));
    let mut hash_index: HashIndex<u64, LargeStruct> =
        HashIndex::new(capacity, mod_factor, raw_store.clone());

    b.iter(|| {
        for id in RANDOM_INDEXES.iter() {
            hash_index.put(*id, LargeStruct::new());
        }
    });
}

fn insert_raw_store_large(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap();
    let mut raw_store = RawStore::new(path);

    b.iter(|| {
        for id in RANDOM_INDEXES.iter() {
            let _ = raw_store.put(&*id, &LargeStruct::new());
        }
    });
}

fn rmw_small(b: &mut Bencher, capacity: usize, mod_factor: f32) {
    let temp_dir = tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap();
    let raw_store = Rc::new(RefCell::new(RawStore::new(path)));
    let mut hash_index: HashIndex<u64, SmallStruct> =
        HashIndex::new(capacity, mod_factor, raw_store.clone());
    for i in 0..TOTAL_KEYS {
        hash_index.put(i, SmallStruct::new());
    }
    b.iter(|| {
        for i in RANDOM_INDEXES.iter() {
            assert_eq!(
                hash_index.rmw(&i, |val| {
                    val.x2 += 10;
                }),
                true
            );
        }
    });
}

fn rmw_large(b: &mut Bencher, capacity: usize, mod_factor: f32) {
    let temp_dir = tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap();
    let raw_store = Rc::new(RefCell::new(RawStore::new(path)));
    let mut hash_index: HashIndex<u64, LargeStruct> =
        HashIndex::new(capacity, mod_factor, raw_store.clone());
    for i in 0..TOTAL_KEYS {
        hash_index.put(i, LargeStruct::new());
    }
    b.iter(|| {
        for i in RANDOM_INDEXES.iter() {
            assert_eq!(
                hash_index.rmw(&i, |val| {
                    val.x2 += 10;
                }),
                true
            );
        }
    });
}

fn rmw_raw_store_small(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap();
    let mut raw_store = RawStore::new(path);
    for i in 0..TOTAL_KEYS {
        let _ = raw_store.put(&i, &SmallStruct::new());
    }
    b.iter(|| {
        for i in RANDOM_INDEXES.iter() {
            let val: Option<SmallStruct> = raw_store.get(i).unwrap();
            let mut new_val = val.unwrap();
            new_val.x2 = new_val.x2 + 10;
            assert_eq!(raw_store.put(i, &new_val).is_ok(), true);
        }
    });
}

fn rmw_raw_store_large(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap();
    let mut raw_store = RawStore::new(path);
    for i in 0..TOTAL_KEYS {
        let _ = raw_store.put(&i, &LargeStruct::new());
    }
    b.iter(|| {
        for i in RANDOM_INDEXES.iter() {
            let val: Option<LargeStruct> = raw_store.get(i).unwrap();
            let mut new_val = val.unwrap();
            new_val.x2 = new_val.x2 + 10;
            assert_eq!(raw_store.put(i, &new_val).is_ok(), true);
        }
    });
}

fn random_get_small(b: &mut Bencher, capacity: usize, mod_factor: f32) {
    let temp_dir = tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap();
    let raw_store = Rc::new(RefCell::new(RawStore::new(path)));
    let mut hash_index: HashIndex<u64, SmallStruct> =
        HashIndex::new(capacity, mod_factor, raw_store.clone());
    for i in 0..TOTAL_KEYS {
        hash_index.put(i, SmallStruct::new());
    }
    b.iter(|| {
        for i in RANDOM_INDEXES.iter() {
            assert_eq!(hash_index.get(&i).is_some(), true);
        }
    });
}

fn random_get_large(b: &mut Bencher, capacity: usize, mod_factor: f32) {
    let temp_dir = tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap();
    let raw_store = Rc::new(RefCell::new(RawStore::new(path)));
    let mut hash_index: HashIndex<u64, LargeStruct> =
        HashIndex::new(capacity, mod_factor, raw_store.clone());
    for i in 0..TOTAL_KEYS {
        hash_index.put(i, LargeStruct::new());
    }
    b.iter(|| {
        for i in RANDOM_INDEXES.iter() {
            assert_eq!(hash_index.get(&i).is_some(), true);
        }
    });
}

fn raw_store_random_small_get(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap();
    let mut raw_store = RawStore::new(path);
    for i in 0..TOTAL_KEYS {
        let _ = raw_store.put(&i, &SmallStruct::new());
    }
    b.iter(|| {
        for i in RANDOM_INDEXES.iter() {
            let data: Option<SmallStruct> = raw_store.get(i).unwrap();
            assert_eq!(data.is_some(), true);
        }
    });
}

fn raw_store_random_large_get(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap();
    let mut raw_store = RawStore::new(path);
    for i in 0..TOTAL_KEYS {
        let _ = raw_store.put(&i, &LargeStruct::new());
    }
    b.iter(|| {
        for i in RANDOM_INDEXES.iter() {
            let data: Option<LargeStruct> = raw_store.get(i).unwrap();
            assert_eq!(data.is_some(), true);
        }
    });
}

criterion_group!(benches, hash);
criterion_main!(benches);
