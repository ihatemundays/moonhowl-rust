use crate::archetype::Archetype;
use crate::component::Component;
use crate::entity::Entity;
use crate::sparse_set::SparseSet;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::thread;

#[derive(Default)]
struct FxHasher(u64);

impl Hasher for FxHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;
        for chunk in bytes.chunks(8) {
            let mut buf = [0u8; 8];
            buf[..chunk.len()].copy_from_slice(chunk);
            self.0 = (self.0.rotate_left(5) ^ u64::from_ne_bytes(buf)).wrapping_mul(SEED);
        }
    }
}

pub(crate) trait ComponentStore: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn entities(&self) -> &[Entity];
    fn remove(&mut self, entity: Entity);
    fn version(&self) -> u64;
}

impl<T: Component> ComponentStore for SparseSet<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn entities(&self) -> &[Entity] {
        SparseSet::entities(self)
    }

    fn remove(&mut self, entity: Entity) {
        SparseSet::remove(self, entity);
    }

    fn version(&self) -> u64 {
        SparseSet::version(self)
    }
}

type StoreMap = HashMap<TypeId, Box<dyn ComponentStore>, BuildHasherDefault<FxHasher>>;

#[derive(Default)]
struct QueryCache {
    driving_type: Option<TypeId>,
    driving_version: u64,
    entities: Vec<Entity>,
}

pub struct World {
    generations: Vec<u32>,
    free_indices: Vec<u32>,
    stores: StoreMap,
    query_cache: RefCell<HashMap<TypeId, QueryCache, BuildHasherDefault<FxHasher>>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            generations: Vec::new(),
            free_indices: Vec::new(),
            stores: StoreMap::default(),
            query_cache: RefCell::new(HashMap::default()),
        }
    }

    pub fn spawn(&mut self) -> Entity {
        if let Some(index) = self.free_indices.pop() {
            Entity::new(index, self.generations[index as usize])
        } else {
            let index = self.generations.len() as u32;
            self.generations.push(0);
            Entity::new(index, 0)
        }
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }

        for store in self.stores.values_mut() {
            store.remove(entity);
        }

        self.generations[entity.index() as usize] += 1;
        self.free_indices.push(entity.index());
        true
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.generations.get(entity.index() as usize) == Some(&entity.generation())
    }

    pub fn len(&self) -> usize {
        self.generations.len() - self.free_indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn set_component<T: Component>(&mut self, entity: Entity, component: T) -> &mut Self {
        self.store_mut_or_init::<T>().insert(entity, component);
        self
    }

    pub fn unset_component<T: Component>(&mut self, entity: Entity) -> &mut Self {
        if let Some(store) = self.store_mut::<T>() {
            store.remove(entity);
        }
        self
    }

    pub fn has_component<T: Component>(&self, entity: Entity) -> bool {
        self.store::<T>().is_some_and(|store| store.contains(entity))
    }

    pub fn get<A: Archetype>(&self, entity: Entity) -> Option<A::Ref<'_>> {
        A::fetch(self, entity)
    }

    pub fn get_mut<A: Archetype>(&mut self, entity: Entity) -> Option<A::RefMut<'_>> {
        A::fetch_mut(self, entity)
    }

    pub fn with_archetype<A: Archetype>(&self, mut f: impl FnMut(A::Ref<'_>, Entity)) {
        let Some(view) = A::resolve(self) else { return };
        let entities = self.driving_entities::<A>();
        for &entity in &entities {
            if let Some(value) = A::fetch_view(&view, entity) {
                f(value, entity);
            }
        }
        self.restore_driving_entities::<A>(entities);
    }

    pub fn with_archetype_mut<A: Archetype>(&mut self, mut f: impl FnMut(A::RefMut<'_>, Entity)) {
        let entities = self.driving_entities::<A>();
        if let Some(mut view) = A::resolve_mut(self) {
            for &entity in &entities {
                if let Some(value) = A::fetch_view_mut(&mut view, entity) {
                    f(value, entity);
                }
            }
        }
        self.restore_driving_entities::<A>(entities);
    }

    pub fn with_archetype_async<A>(&self, f: impl Fn(A::Ref<'_>, Entity) + Sync)
    where
        A: Archetype,
        for<'a> A::View<'a>: Sync,
    {
        let Some(view) = A::resolve(self) else { return };
        let f = &f;
        let view = &view;
        let entities = self.driving_entities::<A>();
        for_each_chunk(&entities, |chunk| {
            for &entity in chunk {
                if let Some(value) = A::fetch_view(view, entity) {
                    f(value, entity);
                }
            }
        });
        self.restore_driving_entities::<A>(entities);
    }

    pub fn with_archetype_async_mut<T: Component>(&mut self, f: impl Fn(&mut T, Entity) + Sync) {
        let f = &f;
        if let Some(store) = self.store_mut::<T>() {
            let (entities, values) = store.entities_and_values_mut();
            for_each_indexed_chunk_mut(entities, values, |entity_chunk, value_chunk| {
                for (&entity, value) in entity_chunk.iter().zip(value_chunk) {
                    f(value, entity);
                }
            });
        }
    }

    pub(crate) fn component<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.store::<T>()?.get(entity)
    }

    pub(crate) fn component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        self.store_mut::<T>()?.get_mut(entity)
    }

    pub(crate) fn disjoint_stores_mut<const N: usize>(
        &mut self,
        ids: [TypeId; N],
    ) -> [Option<&mut Box<dyn ComponentStore>>; N] {
        self.stores.get_disjoint_mut(ids.each_ref())
    }

    pub(crate) fn store<T: Component>(&self) -> Option<&SparseSet<T>> {
        self.stores.get(&TypeId::of::<T>())?.as_any().downcast_ref()
    }

    pub(crate) fn store_mut<T: Component>(&mut self) -> Option<&mut SparseSet<T>> {
        self.stores.get_mut(&TypeId::of::<T>())?.as_any_mut().downcast_mut()
    }

    fn store_mut_or_init<T: Component>(&mut self) -> &mut SparseSet<T> {
        self.stores
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(SparseSet::<T>::new()))
            .as_any_mut()
            .downcast_mut()
            .expect("component store type mismatch for TypeId")
    }
    
    fn driving_entities<A: Archetype>(&self) -> Vec<Entity> {
        let type_ids = A::type_ids();
        let mut driving: Option<(TypeId, usize)> = None;
        for &id in &type_ids {
            let Some(store) = self.stores.get(&id) else {
                return Vec::new();
            };
            let len = store.entities().len();
            if driving.is_none_or(|(_, current_len)| len < current_len) {
                driving = Some((id, len));
            }
        }
        let Some((driving_type, _)) = driving else {
            return Vec::new();
        };
        let driving_version = self.stores[&driving_type].version();

        let mut cache = self.query_cache.borrow_mut();
        let entry = cache.entry(TypeId::of::<A>()).or_default();
        if entry.driving_type != Some(driving_type) || entry.driving_version != driving_version {
            entry.entities.clear();
            entry.entities.extend_from_slice(self.stores[&driving_type].entities());
            entry.driving_type = Some(driving_type);
            entry.driving_version = driving_version;
        }
        std::mem::take(&mut entry.entities)
    }
    
    fn restore_driving_entities<A: Archetype>(&self, entities: Vec<Entity>) {
        if let Some(entry) = self.query_cache.borrow_mut().get_mut(&TypeId::of::<A>()) {
            entry.entities = entities;
        }
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

fn for_each_indexed_chunk_mut<T: Send>(
    entities: &[Entity],
    values: &mut [T],
    work: impl Fn(&[Entity], &mut [T]) + Sync,
) {
    let work = &work;
    let size = chunk_size(values.len());
    thread::scope(|scope| {
        for (entity_chunk, value_chunk) in entities.chunks(size).zip(values.chunks_mut(size)) {
            scope.spawn(move || work(entity_chunk, value_chunk));
        }
    });
}
