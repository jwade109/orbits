use bary_core::prelude::EntityId;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct EntitySpawner {
    next_id: EntityId,
}

impl Default for EntitySpawner {
    fn default() -> Self {
        Self {
            next_id: EntityId(0),
        }
    }
}

impl EntitySpawner {
    pub fn spawn(&mut self) -> EntityId {
        let ret = self.next_id;
        self.next_id.0 += 1;
        ret
    }
}

pub struct Components<E> {
    values: BTreeMap<EntityId, E>,
}

impl<E> std::ops::Deref for Components<E> {
    type Target = BTreeMap<EntityId, E>;
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
    pub fn spawn(&mut self, id: EntityId, e: E) {
        self.values.insert(id, e);
    }

    pub fn despawn(&mut self, id: EntityId) -> BaryResult<E> {
        let e = self.values.remove(&id);
        if let Some(e) = e {
            Ok(e)
        } else {
            Err(BaryError::EntityNotFound)
        }
    }

    pub fn get(&self, id: EntityId) -> Option<&E> {
        self.values.get(&id)
    }

    pub fn try_get(&self, id: EntityId) -> BaryResult<&E> {
        self.values.get(&id).ok_or(BaryError::EntityNotFound)
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut E> {
        self.values.get_mut(&id)
    }

    pub fn try_get_mut(&mut self, id: EntityId) -> BaryResult<&mut E> {
        self.values.get_mut(&id).ok_or(BaryError::EntityNotFound)
    }

    pub fn get_with_log(&self, id: EntityId) -> Option<&E> {
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

#[derive(Debug, PartialEq, Eq)]
pub enum BaryError {
    EntityNotFound,
    BadPartName,
    BadBlueprint,
}

pub type BaryResult<E> = Result<E, BaryError>;
