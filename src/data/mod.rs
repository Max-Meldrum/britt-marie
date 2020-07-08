use crate::error::*;

// TODO: Fix this mess.
// NOTE: Create common trait for BrittMarie data type and
//       put prost as default behind a cfg flag. 

pub trait Value: prost::Message + Default + Clone + 'static {
    fn into_raw(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(self.encoded_len());
        self.encode(&mut buf)
            .map_err(|e| BrittMarieError::Serde(e.to_string()))?;
        Ok(buf)
    }
    fn from_raw(bytes: &[u8]) -> Result<Self> {
        Self::decode(bytes).map_err(|e| BrittMarieError::Serde(e.to_string()))
    }
}
impl<T> Value for T where T: prost::Message + Default + Clone + 'static {}

pub trait Key: prost::Message + Default + Clone + 'static {
    fn into_raw(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(self.encoded_len());
        self.encode(&mut buf)
            .map_err(|e| BrittMarieError::Serde(e.to_string()))?;
        Ok(buf)
    }
    fn from_raw(bytes: &[u8]) -> Result<Self> {
        Self::decode(bytes).map_err(|e| BrittMarieError::Serde(e.to_string()))
    }
}
impl<T> Key for T where T: prost::Message + Default + Clone + 'static {}
