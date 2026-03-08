use crate::result::*;
use bary_core::prelude::Ent;
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntitySpawner {
    next_id: Ent,
}

impl Default for EntitySpawner {
    fn default() -> Self {
        Self { next_id: Ent(0) }
    }
}

impl EntitySpawner {
    pub fn spawn(&mut self) -> Ent {
        let ret = self.next_id;
        self.next_id.0 += 1;
        ret
    }

    pub fn next(&self) -> Ent {
        self.next_id
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Components<E> {
    values: BTreeMap<Ent, E>,
}

impl<E> std::ops::Deref for Components<E> {
    type Target = BTreeMap<Ent, E>;
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl<E> std::ops::DerefMut for Components<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl<E> Default for Components<E> {
    fn default() -> Self {
        Self {
            values: BTreeMap::default(),
        }
    }
}

impl<E> Components<E> {
    pub fn spawn(&mut self, id: Ent, e: E) {
        self.values.insert(id, e);
    }

    pub fn despawn(&mut self, id: Ent) -> BaryResult<E> {
        let e = self.values.remove(&id);
        if let Some(e) = e {
            Ok(e)
        } else {
            Err(BaryError::EntityNotFound(id))
        }
    }

    pub fn despawn_or_log(&mut self, id: Ent) {
        if let Err(e) = self.despawn(id) {
            error!("Failed to despawn {}: {:?}", id, e);
        }
    }

    pub fn get(&self, id: Ent) -> Option<&E> {
        self.values.get(&id)
    }

    pub fn try_get(&self, id: Ent) -> BaryResult<&E> {
        self.values.get(&id).ok_or(BaryError::EntityNotFound(id))
    }

    pub fn get_mut(&mut self, id: Ent) -> Option<&mut E> {
        self.values.get_mut(&id)
    }

    pub fn try_get_mut(&mut self, id: Ent) -> BaryResult<&mut E> {
        self.values.get_mut(&id).ok_or(BaryError::EntityNotFound(id))
    }

    pub fn get_with_log(&self, id: Ent) -> Option<&E> {
        let e = self.values.get(&id);
        if e.is_none() {
            println!("Lookup failed: {id}");
        }
        e
    }
}

impl<E> std::fmt::Debug for Components<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} components", self.values.len(),)
    }
}
