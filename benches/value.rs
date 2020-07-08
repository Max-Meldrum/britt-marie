use britt_marie::{ValueIndex, ValueOps, RawStore};
use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use std::cell::RefCell;
use std::rc::Rc;


fn value(c: &mut Criterion) {
    let mut group = c.benchmark_group("value");
    group.bench_function("lazy rolling counter", lazy_rolling_counter);
    group.bench_function("cow rolling counter", cow_rolling_counter);
    group.finish()
}

fn lazy_rolling_counter(b: &mut Bencher) {
    let raw_store = Rc::new(RefCell::new(RawStore::new("/tmp/rolling")));
    let mut value_index: ValueIndex<u64> = ValueIndex::new("_rolling_counter", raw_store);
    b.iter(|| {
        value_index.rmw(|v| {
            *v += 1;
        });
    });
}

fn cow_rolling_counter(b: &mut Bencher) {
    let raw_store = Rc::new(RefCell::new(RawStore::new("/tmp/rolling")));
    let mut value_index: ValueIndex<u64> = ValueIndex::cow("_rolling_counter", raw_store);
    b.iter(|| {
        value_index.rmw(|v| {
            *v += 1;
        });
    });
}

criterion_group!(benches, value);
criterion_main!(benches);
