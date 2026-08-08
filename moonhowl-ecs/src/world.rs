use crate::entity::Entity;
use crate::system::{ISystem, System};
use std::collections::HashMap;
use std::thread;

#[derive(Default)]
pub struct World {
    entities: HashMap<usize, Entity>,
    systems: Vec<(System, Box<dyn ISystem>)>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&mut self) -> usize {
        let entity = Entity::new();
        let id = entity.get_id();
        self.entities.insert(id, entity);
        id
    }

    pub fn despawn(&mut self, id: usize) -> Option<Entity> {
        self.entities.remove(&id)
    }

    pub fn despawn_all(&mut self) {
        self.entities.clear();
    }

    pub fn contains(&self, id: usize) -> bool {
        self.entities.contains_key(&id)
    }

    pub fn get_entity(&self, id: usize) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_entity_mut(&mut self, id: usize) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values()
    }

    pub fn register_system<S: ISystem + 'static>(&mut self, system: S) -> usize {
        let handle = System::new();
        let id = handle.get_id();
        self.systems.push((handle, Box::new(system)));
        id
    }

    /// Runs every registered system against every entity. Entities run
    /// concurrently (one thread per entity); each thread owns its entity
    /// exclusively via `&mut Entity`, so systems can mutate components
    /// without any cross-entity data races. Within an entity's thread, all
    /// systems' `check`s run first against the entity's original state, then
    /// the `and_then`s of the systems that passed run in registration order —
    /// so no check is affected by another system's mutations this pass.
    pub fn run(&mut self) {
        let Self { entities, systems } = self;

        thread::scope(|scope| {
            for entity in entities.values_mut() {
                let systems = &*systems;
                scope.spawn(move || {
                    let passed: Vec<&(System, Box<dyn ISystem>)> = systems
                        .iter()
                        .filter(|(system, system_impl)| system_impl.check(system, entity))
                        .collect();

                    for (system, system_impl) in passed {
                        system_impl.and_then(system, entity);
                    }
                });
            }
        });
    }
}
