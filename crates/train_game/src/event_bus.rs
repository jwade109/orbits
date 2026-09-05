use bary_core::prelude::Ent;

use crate::terrain::ChunkIndex;

pub struct FontSelection {
    font_id: Option<Ent>,
}

impl FontSelection {
    pub fn new() -> Self {
        Self { font_id: None }
    }

    pub fn clicked(&mut self, font_id: Ent) {
        self.font_id = Some(font_id);
    }

    pub fn new_font_id(&self) -> Option<Ent> {
        self.font_id
    }
}

pub enum TrainEvent {
    ChunkUpdate(Ent),
    Sound,
    Other,
}

pub struct EventBus {
    events: Vec<TrainEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn enqueue(&mut self, event: TrainEvent) {
        self.events.push(event);
    }

    pub fn iter(&self) -> impl Iterator<Item = &TrainEvent> {
        self.events.iter()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}
