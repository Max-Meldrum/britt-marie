use britt_marie::{BrittMarie, IndexOps, RawStore, ValueIndex, ValueOps};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(BrittMarie)]
pub struct StreamingState {
    watermark: ValueIndex<u64>,
    epoch: ValueIndex<u64>,
}

#[test]
fn streaming_state_test() {
    let raw_store = Rc::new(RefCell::new(RawStore::new("/tmp/state")));
    let watermark: ValueIndex<u64> = ValueIndex::new("_watermark", raw_store.clone());
    let epoch: ValueIndex<u64> = ValueIndex::new("_epoch", raw_store.clone());
    let mut state = StreamingState { watermark, epoch };

    state.watermark().put(100);
    state.epoch().put(1);

    assert_eq!(state.watermark().get(), Some(&100));
    assert_eq!(state.epoch().get(), Some(&1));
    assert_eq!(state.checkpoint(raw_store).is_ok(), true);
}
