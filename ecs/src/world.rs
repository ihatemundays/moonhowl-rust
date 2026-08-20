use crate::archetype::Archetype;
use crate::entity::Entity;
use std::thread;

pub struct World {
    entities: Vec<Entity>,
}

impl World {
    pub fn new() -> Self {
        Self { entities: Vec::new() }
    }

    pub fn spawn(&mut self) -> usize {
        self.insert(Entity::new())
    }

    pub fn insert(&mut self, entity: Entity) -> usize {
        self.entities.push(entity);
        self.entities.len() - 1
    }

    pub fn entity(&self, id: usize) -> Option<&Entity> {
        self.entities.get(id)
    }

    pub fn entity_mut(&mut self, id: usize) -> Option<&mut Entity> {
        self.entities.get_mut(id)
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Runs `f` against every entity matching archetype `A`, sequentially on the calling thread.
    pub fn with_archetype<A: Archetype>(&self, mut f: impl FnMut(A::Ref<'_>)) {
        for entity in &self.entities {
            entity.with_archetype::<A, _>(&mut f);
        }
    }

    /// Runs `f` against every entity matching archetype `A`, sequentially on the calling thread.
    pub fn with_archetype_mut<A: Archetype>(&mut self, mut f: impl FnMut(A::RefMut<'_>)) {
        for entity in &mut self.entities {
            entity.with_archetype_mut::<A, _>(&mut f);
        }
    }

    /// Runs `f` against every entity matching archetype `A`, splitting entities into disjoint
    /// chunks and running them across a scoped thread pool sized to the available parallelism.
    pub fn with_archetype_async<A: Archetype>(&self, f: impl Fn(A::Ref<'_>) + Sync) {
        let f = &f;
        for_each_chunk(&self.entities, |chunk| {
            for entity in chunk {
                entity.with_archetype::<A, _>(f);
            }
        });
    }

    /// Runs `f` against every entity matching archetype `A`, splitting entities into disjoint
    /// chunks and running them across a scoped thread pool sized to the available parallelism.
    pub fn with_archetype_async_mut<A: Archetype>(&mut self, f: impl Fn(A::RefMut<'_>) + Sync) {
        let f = &f;
        for_each_chunk_mut(&mut self.entities, |chunk| {
            for entity in chunk {
                entity.with_archetype_mut::<A, _>(f);
            }
        });
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

fn chunk_size(len: usize) -> usize {
    let threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    len.div_ceil(threads).max(1)
}

fn for_each_chunk<T: Sync>(items: &[T], work: impl Fn(&[T]) + Sync) {
    let work = &work;
    thread::scope(|scope| {
        for chunk in items.chunks(chunk_size(items.len())) {
            scope.spawn(move || work(chunk));
        }
    });
}

fn for_each_chunk_mut<T: Send>(items: &mut [T], work: impl Fn(&mut [T]) + Sync) {
    let work = &work;
    let size = chunk_size(items.len());
    thread::scope(|scope| {
        for chunk in items.chunks_mut(size) {
            scope.spawn(move || work(chunk));
        }
    });
}
