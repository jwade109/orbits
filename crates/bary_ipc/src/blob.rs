use bary_core::prelude::TableIdent;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Blob {
    data: Vec<u8>,
    ident: TableIdent,
}

impl Blob {
    pub fn new(data: Vec<u8>, ident: TableIdent) -> Self {
        Self { data, ident }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn consume(self) -> Vec<u8> {
        self.data
    }

    pub fn table(&self) -> TableIdent {
        self.ident
    }
}

impl std::fmt::Debug for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Blob({:?}, {} bytes)", self.ident, self.data.len())
    }
}

impl std::fmt::Display for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Blob({:?}, {} bytes)", self.ident, self.data.len())
    }
}
