use crate::entity::{ActionContext, CheckContext, IEntity};
use crate::system::ISystem;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::thread;

trait ErasedSystem: Send + Sync {
    fn check_erased(&self, system: &CheckContext, entity: &(dyn Any + Send)) -> bool;
    fn and_then_erased(&self, system: &ActionContext, entity: &(dyn Any + Send));
}

struct SystemWrapper<E, S> {
    system: S,
    // fn() -> E, not E directly: a bare PhantomData<E> would make
    // SystemWrapper's Send/Sync depend on E's, but E only needs to be
    // IEntity (Send), and ErasedSystem must be Send + Sync regardless.
    _marker: PhantomData<fn() -> E>,
}

impl<E: IEntity, S: ISystem<E>> ErasedSystem for SystemWrapper<E, S> {
    fn check_erased(&self, system: &CheckContext, entity: &(dyn Any + Send)) -> bool {
        entity
            .downcast_ref::<E>()
            .is_some_and(|entity| self.system.check(system, entity))
    }

    fn and_then_erased(&self, system: &ActionContext, entity: &(dyn Any + Send)) {
        if let Some(entity) = entity.downcast_ref::<E>() {
            self.system.and_then(system, entity);
        }
    }
}

#[derive(Default)]
pub struct World {
    entities: HashMap<TypeId, HashMap<usize, Box<dyn Any + Send>>>,
    // A system type is a singleton per entity type — registering the same S
    // twice for the same E replaces the old registration rather than running
    // two copies. Kept as a Vec (not a HashMap keyed by system TypeId) so
    // registration order — and therefore and_then execution order — stays
    // deterministic; a HashMap's iteration order is not.
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

    /// Runs every registered system against every entity of the type it was
    /// registered for. Entities run concurrently (one thread per entity);
    /// each thread owns its entity exclusively, so systems can mutate
    /// components without any cross-entity data races. Within an entity's
    /// thread, all matching systems' `check`s run first against the entity's
    /// original state, then the `and_then`s of the systems that passed run
    /// in registration order — so no check is affected by another system's
    /// mutations this pass.
    ///
    /// A system only ever runs against entities of the type it was
    /// registered for: entity types with no matching systems are skipped
    /// entirely, with no per-entity `check` call at all.
    pub fn run(&mut self) {
        let Self { entities, systems } = self;

        thread::scope(|scope| {
            for (type_id, bucket) in entities.iter_mut() {
                let Some(matching_systems) = systems.get(type_id) else {
                    continue;
                };

                for entity in bucket.values_mut() {
                    let entity: &mut (dyn Any + Send) = &mut **entity;
                    scope.spawn(move || {
                        let mut passed: Vec<(TypeId, &Box<dyn ErasedSystem>)> =
                            Vec::with_capacity(matching_systems.len());
                        passed.extend(matching_systems.iter().filter_map(
                            |(system_id, system_impl)| {
                                system_impl
                                    .check_erased(&CheckContext::new(*system_id), entity)
                                    .then_some((*system_id, system_impl))
                            },
                        ));

                        for (system_id, system_impl) in passed {
                            system_impl.and_then_erased(&ActionContext::new(system_id), entity);
                        }
                    });
                }
            }
        });
    }
}
