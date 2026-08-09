use crate::entity::{ActionContext, CheckContext, IEntity};
use crate::system::ISystem;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;

trait ErasedSystem {
    fn check_erased(&self, system: &CheckContext, entity: &dyn Any) -> bool;
    fn and_then_erased(&self, system: &ActionContext, entity: &dyn Any);
}

struct SystemWrapper<E, S> {
    system: S,
    _marker: PhantomData<E>,
}

impl<E: IEntity, S: ISystem<E>> ErasedSystem for SystemWrapper<E, S> {
    fn check_erased(&self, system: &CheckContext, entity: &dyn Any) -> bool {
        entity
            .downcast_ref::<E>()
            .is_some_and(|entity| self.system.check(system, entity))
    }

    fn and_then_erased(&self, system: &ActionContext, entity: &dyn Any) {
        if let Some(entity) = entity.downcast_ref::<E>() {
            self.system.and_then(system, entity);
        }
    }
}

#[derive(Default)]
pub struct World {
    entities: HashMap<TypeId, HashMap<usize, Box<dyn Any>>>,
    systems: HashMap<TypeId, Vec<(TypeId, Box<dyn ErasedSystem>)>>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn<E: IEntity>(&mut self, entity: E) -> usize {
        let id = entity.get_id();
        self.entities
            .entry(TypeId::of::<E>())
            .or_default()
            .insert(id, Box::new(entity));
        id
    }

    pub fn despawn<E: IEntity>(&mut self, id: usize) -> Option<E> {
        let boxed = self.entities.get_mut(&TypeId::of::<E>())?.remove(&id)?;
        Some(*boxed.downcast::<E>().unwrap())
    }

    pub fn despawn_all(&mut self) {
        self.entities.clear();
    }

    pub fn contains<E: IEntity>(&self, id: usize) -> bool {
        self.entities
            .get(&TypeId::of::<E>())
            .is_some_and(|bucket| bucket.contains_key(&id))
    }

    pub fn get_entity<E: IEntity>(&self, id: usize) -> Option<&E> {
        self.entities
            .get(&TypeId::of::<E>())?
            .get(&id)?
            .downcast_ref::<E>()
    }

    pub fn get_entity_mut<E: IEntity>(&mut self, id: usize) -> Option<&mut E> {
        self.entities
            .get_mut(&TypeId::of::<E>())?
            .get_mut(&id)?
            .downcast_mut::<E>()
    }

    pub fn len(&self) -> usize {
        self.entities.values().map(HashMap::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.values().all(HashMap::is_empty)
    }

    pub fn iter<E: IEntity>(&self) -> impl Iterator<Item = &E> {
        self.entities
            .get(&TypeId::of::<E>())
            .into_iter()
            .flat_map(|bucket| bucket.values())
            .filter_map(|entity| entity.downcast_ref::<E>())
    }

    pub fn register_system<E: IEntity, S: ISystem<E> + 'static>(&mut self, system: S) -> bool {
        let system_id = TypeId::of::<S>();
        let wrapper: Box<dyn ErasedSystem> = Box::new(SystemWrapper::<E, S> {
            system,
            _marker: PhantomData,
        });

        let bucket = self.systems.entry(TypeId::of::<E>()).or_default();

        if let Some(entry) = bucket.iter_mut().find(|(id, _)| *id == system_id) {
            entry.1 = wrapper;
            true
        } else {
            bucket.push((system_id, wrapper));
            false
        }
    }

    pub fn deregister_system<E: IEntity, S: 'static>(&mut self) -> bool {
        let Some(bucket) = self.systems.get_mut(&TypeId::of::<E>()) else {
            return false;
        };

        let system_id = TypeId::of::<S>();
        let Some(pos) = bucket.iter().position(|(id, _)| *id == system_id) else {
            return false;
        };

        bucket.remove(pos);
        true
    }

    pub fn deregister_all_systems(&mut self) {
        self.systems.clear();
    }

    pub fn reset(&mut self) {
        self.despawn_all();
        self.deregister_all_systems();
    }

    pub fn run(&mut self) {
        for (type_id, bucket) in &self.entities {
            let Some(matching_systems) = self.systems.get(type_id) else {
                continue;
            };

            for entity in bucket.values() {
                let entity = entity.as_ref();

                for (system_id, system_impl) in matching_systems {
                    if system_impl.check_erased(&CheckContext::new(*system_id), entity) {
                        system_impl.and_then_erased(&ActionContext::new(*system_id), entity);
                    }
                }
            }
        }
    }
}
