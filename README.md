# Britt-Marie

An early Proof-of-Concept storage solution for stream processing systems in Rust.

**Motivation:** Existing stream processors tend to use state backends (e.g., RocksDB) as they are without
taking advantage of the system context.

Britt-Marie offers a set of indexes that are backed by a durable state backend. The
implementations are by default lazy. It is however possible to enable COW (Copy on Write) for individual
indexes.

```rust
use britt_marie::{BrittMarie, HashIndex, HashOps, IndexOps, RawStore, ValueIndex, ValueOps};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(BrittMarie)]
pub struct StreamingState {
    watermark: ValueIndex<u64>,
    epoch: ValueIndex<u64>,
    counters: HashIndex<u64, u64>,
}

let raw_store = Rc::new(RefCell::new(RawStore::new("/tmp/state")));
let watermark: ValueIndex<u64> = ValueIndex::new("_watermark", raw_store.clone());
let epoch: ValueIndex<u64> = ValueIndex::new("_epoch", raw_store.clone());
let modificaton_factor: f32 = 0.6;
let counters: HashIndex<u64, u64> =
    HashIndex::new(128, modificaton_factor, raw_store.clone());

let mut state = StreamingState {
    watermark,
    epoch,
    counters,
};

state.watermark().put(100);
state.epoch().put(1);
state.counters().put(10, 1);

// Calls a persist function on each index before running the actual checkpoint
state.checkpoint(raw_store);
```

## License

Licensed under the terms of MIT license.

See [LICENSE](LICENSE) for details.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in britt-marie by you shall be licensed as MIT, without any additional terms or conditions.
