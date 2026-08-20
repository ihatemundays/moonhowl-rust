use crate::entity::Entity;

pub struct SparseSet<T> {
    sparse: Vec<Option<u32>>,
    dense_entities: Vec<Entity>,
    dense: Vec<T>,
    version: u64,
}

impl<T> SparseSet<T> {
    pub fn new() -> Self {
        Self { sparse: Vec::new(), dense_entities: Vec::new(), dense: Vec::new(), version: 0 }
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
        self.version += 1;
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
        self.version += 1;
        Some(removed)
    }

    pub(crate) fn version(&self) -> u64 {
        self.version
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

    /// `entities()` and `values_mut()` at once, index-aligned -- a single call so both
    /// borrows can be held together (two separate calls can't, since one needs `&mut self`).
    pub fn entities_and_values_mut(&mut self) -> (&[Entity], &mut [T]) {
        (&self.dense_entities, &mut self.dense)
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
