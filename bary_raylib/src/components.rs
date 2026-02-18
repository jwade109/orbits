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
