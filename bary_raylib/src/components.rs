use bary_core::prelude::EntityId;
use std::collections::BTreeMap;

pub struct Components<E> {
    next_id: EntityId,
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
            next_id: EntityId::default(),
            values: BTreeMap::default(),
        }
    }
}

impl<E> Components<E> {
    pub fn spawn(&mut self, e: E) -> EntityId {
        let id = self.next_id;
        self.next_id.0 += 1;
        self.values.insert(id, e);
        id
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

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut E> {
        self.values.get_mut(&id)
    }

    pub fn get_with_log(&self, id: EntityId) -> Option<&E> {
        let e = self.values.get(&id);
        if e.is_none() {
            println!("Lookup failed: {id}");
        }
        e
    }

    pub fn high_water_mark(&self) -> EntityId {
        self.next_id
    }
}

impl<E> std::fmt::Debug for Components<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} components, {} hwm",
            self.values.len(),
            self.high_water_mark()
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BaryError {
    EntityNotFound,
}

pub type BaryResult<E> = Result<E, BaryError>;
