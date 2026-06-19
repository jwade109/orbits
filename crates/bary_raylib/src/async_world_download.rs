use bary_core::prelude::TableIdent;
use bary_ipc::{Blob, apply_blob};
use bary_sim::*;
use log::*;
use std::collections::BTreeSet;

pub struct AsyncWorldDownload {
    world: World,
    tables_completed: BTreeSet<TableIdent>,
    n_tables_expected: usize,
    status: String,
    is_fail: bool,
}

impl AsyncWorldDownload {
    pub fn new() -> Self {
        Self {
            world: World::empty(),
            tables_completed: BTreeSet::new(),
            n_tables_expected: TableIdent::all().count(),
            status: "Requesting world from server...".to_string(),
            is_fail: false,
        }
    }

    pub fn add_blob(&mut self, blob: Blob) {
        info!("Got blob: {blob}");
        let table = blob.table();
        if apply_blob(&mut self.world, blob) {
            info!("Unpacked blob data for table {table}");
            self.tables_completed.insert(table);
            self.status = format!("Got entity table \"{table}\"...");
        } else {
            error!("Failed to unpack blob for table {table}");
            self.status = format!("Failed to unpack \"{table}\"!");
            self.is_fail = true;
        }
    }

    pub fn steps(&self) -> u32 {
        self.tables_completed.len() as u32
    }

    pub fn total(&self) -> u32 {
        self.n_tables_expected as u32
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn is_fail(&self) -> bool {
        self.is_fail
    }

    pub fn is_complete(&self) -> bool {
        self.n_tables_expected == self.tables_completed.len()
    }

    pub fn world(self) -> World {
        self.world
    }
}
