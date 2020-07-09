use britt_marie::{BrittMarie, HashIndex, HashOps, IndexOps, RawStore, ValueIndex, ValueOps};
use std::cell::RefCell;
use std::rc::Rc;
use tempfile::tempdir;

#[derive(BrittMarie)]
pub struct StreamingState {
    watermark: ValueIndex<u64>,
    epoch: ValueIndex<u64>,
    counters: HashIndex<u64, u64>,
}

#[test]
fn streaming_state_test() {
    let temp_dir = tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap();
    let raw_store = Rc::new(RefCell::new(RawStore::new(path)));
    let watermark: ValueIndex<u64> = ValueIndex::new("_watermark", raw_store.clone());
    let epoch: ValueIndex<u64> = ValueIndex::new("_epoch", raw_store.clone());
    let capacity = 128;
    let modificaton_factor: f32 = 0.6;
    let counters: HashIndex<u64, u64> =
        HashIndex::new(capacity, modificaton_factor, raw_store.clone());

    let mut state = StreamingState {
        watermark,
        epoch,
        counters,
    };

    state.watermark().put(100);
    state.epoch().put(1);
    state.counters().put(10, 1);

    assert_eq!(state.watermark().get(), Some(&100));
    assert_eq!(state.epoch().get(), Some(&1));
    assert_eq!(state.counters().get(&10), Some(&1));
    assert_eq!(state.checkpoint(raw_store).is_ok(), true);
}
