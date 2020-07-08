use thiserror::Error;

#[derive(Error, Debug)]
pub enum BrittMarieError {
    #[error("Serde error `{0}`")]
    Serde(String),
    #[error("RawStore Insertion Error `{0}`")]
    Insert(String),
    #[error("RawStore Read Error `{0}`")]
    Read(String),
    #[error("RawStore Checkpoint Error `{0}`")]
    Checkpoint(String),
    #[error("unknown data store error")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, BrittMarieError>;
