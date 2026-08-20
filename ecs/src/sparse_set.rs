use crate::entity::Entity;

pub struct SparseSet<T> {
    sparse: Vec<Option<u32>>,
    dense_entities: Vec<Entity>,
    dense: Vec<T>,
}

impl<T> SparseSet<T> {
    pub fn new() -> Self {
        Self { sparse: Vec::new(), dense_entities: Vec::new(), dense: Vec::new() }
    }

    pub fn insert(&mut self, entity: Entity, value: T) -> Option<T> {
        let index = entity.index() as usize;
        if index >= self.sparse.len() {
            self.sparse.resize(index + 1, None);
        }

        if let Some(dense_index) = self.sparse[index]
            && self.dense_entities[dense_index as usize] == entity
        {
            return Some(std::mem::replace(&mut self.dense[dense_index as usize], value));
        }

        let dense_index = self.dense.len() as u32;
        self.sparse[index] = Some(dense_index);
        self.dense_entities.push(entity);
        self.dense.push(value);
        None
    }

    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let dense_index = self.dense_index_of(entity)?;
        self.sparse[entity.index() as usize] = None;

        let removed = self.dense.swap_remove(dense_index);
        self.dense_entities.swap_remove(dense_index);
        if let Some(&moved) = self.dense_entities.get(dense_index) {
            self.sparse[moved.index() as usize] = Some(dense_index as u32);
        }
        Some(removed)
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.dense_index_of(entity).is_some()
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.dense_index_of(entity).map(|index| &self.dense[index])
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        let index = self.dense_index_of(entity)?;
        Some(&mut self.dense[index])
    }

    pub fn entities(&self) -> &[Entity] {
        &self.dense_entities
    }

    pub fn values_mut(&mut self) -> &mut [T] {
        &mut self.dense
    }

    fn dense_index_of(&self, entity: Entity) -> Option<usize> {
        let dense_index = (*self.sparse.get(entity.index() as usize)?)? as usize;
        (self.dense_entities[dense_index] == entity).then_some(dense_index)
    }
}

impl<T> Default for SparseSet<T> {
    fn default() -> Self {
        Self::new()
    }
}
