use britt_marie::{ValueIndex, ValueOps, RawStore};
use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use std::cell::RefCell;
use std::rc::Rc;


fn value(c: &mut Criterion) {
    let mut group = c.benchmark_group("value");
    group.bench_function("lazy rolling counter", lazy_rolling_counter);
    group.bench_function("cow rolling counter", cow_rolling_counter);
    group.bench_function("rolling counter raw store", raw_store_rolling_count);
    group.finish()
}

fn lazy_rolling_counter(b: &mut Bencher) {
    let raw_store = Rc::new(RefCell::new(RawStore::new("/tmp/rolling")));
    let value_index: ValueIndex<u64> = ValueIndex::new("_rolling_counter", raw_store);
    counter_bench(b, value_index);

}


fn cow_rolling_counter(b: &mut Bencher) {
    let raw_store = Rc::new(RefCell::new(RawStore::new("/tmp/rolling")));
    let value_index: ValueIndex<u64> = ValueIndex::cow("_rolling_counter", raw_store);
    counter_bench(b, value_index);
}

fn counter_bench(b: &mut Bencher, mut index: ValueIndex<u64>) {
    b.iter(|| {
        index.rmw(|v| {
            *v += 1;
        });
    });
}

// TODO: Should probably move this to RocksDB merge operator..
fn raw_store_rolling_count(b: &mut Bencher) {
    let mut raw_store = RawStore::new("/tmp/rolling");
    let key: Vec<u8> = String::from("_rolling_counter").into();
    b.iter(|| {
        let curr: Option<u64>= raw_store.get(&key).unwrap();
        let new_curr = curr.map_or_else(|| 0, |v| v + 1);
        let _ = raw_store.put(&key, &new_curr);
    });
}

criterion_group!(benches, value);
criterion_main!(benches);
