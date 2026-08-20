use crate::archetype::Archetype;
use crate::component::Component;
use crate::entity::Entity;
use crate::sparse_set::SparseSet;
use std::any::{Any, TypeId};
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
}

type StoreMap = HashMap<TypeId, Box<dyn ComponentStore>, BuildHasherDefault<FxHasher>>;

pub struct World {
    generations: Vec<u32>,
    free_indices: Vec<u32>,
    stores: StoreMap,
}

impl World {
    pub fn new() -> Self {
        Self { generations: Vec::new(), free_indices: Vec::new(), stores: StoreMap::default() }
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

    pub fn edit_component<T: Component>(&mut self, entity: Entity, f: impl FnOnce(&mut T)) -> &mut Self {
        if let Some(component) = self.component_mut::<T>(entity) {
            f(component);
        }
        self
    }

    pub fn get<A: Archetype>(&self, entity: Entity) -> Option<A::Ref<'_>> {
        A::fetch(self, entity)
    }

    pub fn get_mut<A: Archetype>(&mut self, entity: Entity) -> Option<A::RefMut<'_>> {
        A::fetch_mut(self, entity)
    }

    pub fn with_archetype<A: Archetype>(&self, mut f: impl FnMut(A::Ref<'_>)) {
        for entity in self.driving_entities(&A::type_ids()) {
            if let Some(value) = A::fetch(self, entity) {
                f(value);
            }
        }
    }

    pub fn with_archetype_mut<A: Archetype>(&mut self, mut f: impl FnMut(A::RefMut<'_>)) {
        for entity in self.driving_entities(&A::type_ids()) {
            if let Some(value) = A::fetch_mut(self, entity) {
                f(value);
            }
        }
    }

    pub fn with_archetype_async<A: Archetype>(&self, f: impl Fn(A::Ref<'_>) + Sync) {
        let f = &f;
        let entities = self.driving_entities(&A::type_ids());
        for_each_chunk(&entities, |chunk| {
            for &entity in chunk {
                if let Some(value) = A::fetch(self, entity) {
                    f(value);
                }
            }
        });
    }

    pub fn with_archetype_async_mut<T: Component>(&mut self, f: impl Fn(&mut T) + Sync) {
        let f = &f;
        if let Some(store) = self.store_mut::<T>() {
            for_each_chunk_mut(store.values_mut(), |chunk| {
                for value in chunk {
                    f(value);
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

    fn store<T: Component>(&self) -> Option<&SparseSet<T>> {
        self.stores.get(&TypeId::of::<T>())?.as_any().downcast_ref()
    }

    fn store_mut<T: Component>(&mut self) -> Option<&mut SparseSet<T>> {
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

    fn driving_entities(&self, type_ids: &[TypeId]) -> Vec<Entity> {
        let mut driving: Option<&dyn ComponentStore> = None;
        for id in type_ids {
            let Some(store) = self.stores.get(id) else {
                return Vec::new();
            };
            if driving.is_none_or(|current| store.entities().len() < current.entities().len()) {
                driving = Some(store.as_ref());
            }
        }
        driving.map(|store| store.entities().to_vec()).unwrap_or_default()
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
