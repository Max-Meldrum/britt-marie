use anyhow::Result;
use std::fmt::Debug;

pub mod entry;

pub(crate) use entry::{EvictedEntry, LazyEntry, RawEntry};

pub trait Serialisable: Send + Debug {
    fn serialise(&self) -> Vec<u8>;
}
pub trait Deserialisable {
    fn deserialise(self, bytes: Vec<u8>) -> Self;
}

pub trait Value: prost::Message + Default + Clone + 'static {
    fn into_raw(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(self.encoded_len());
        self.encode(&mut buf)?;
        Ok(buf)
    }
    fn from_raw(bytes: &[u8]) -> Self {
        Self::decode(bytes).unwrap()
    }
}
impl<T> Value for T where T: prost::Message + Default + Clone + 'static {}

pub trait Key: prost::Message + Default + Clone + 'static {
    fn into_raw(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(self.encoded_len());
        self.encode(&mut buf)?;
        Ok(buf)
    }
    fn from_raw(bytes: &[u8]) -> Self {
        Self::decode(bytes).unwrap()
    }
}
impl<T> Key for T where T: prost::Message + Default + Clone + 'static {}
