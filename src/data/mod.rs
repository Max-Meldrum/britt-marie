use std::fmt::Debug;

pub mod entry;

pub(crate) use entry::{Entry, LazyEntry, RawEntry};

pub trait Serialisable: Send + Debug {
    fn serialise(&self) -> Vec<u8>;
}
pub trait Deserialisable {
    fn deserialise(self, bytes: Vec<u8>) -> Self;
}

pub trait Key: Serialisable + Deserialisable
where
    Self: std::marker::Sized,
{
    fn raw_key(&self) -> Vec<u8> {
        self.serialise()
    }
}
pub trait Value: Serialisable + Deserialisable
where
    Self: std::marker::Sized,
{
    fn raw_value(&self) -> Vec<u8> {
        self.serialise()
    }
}
