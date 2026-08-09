use crate::entity::{ActionContext, CheckContext, Entity};
use crate::system::ISystem;
use std::any::TypeId;
use std::collections::HashMap;
use std::thread;

#[derive(Default)]
pub struct World {
    entities: HashMap<usize, Entity>,
    systems: Vec<(TypeId, Box<dyn ISystem>)>,
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

    pub fn register_system<S: ISystem + 'static>(&mut self, system: S) -> bool {
        let system_id = TypeId::of::<S>();
        let wrapper: Box<dyn ISystem> = Box::new(system);

        if let Some(entry) = self.systems.iter_mut().find(|(id, _)| *id == system_id) {
            entry.1 = wrapper;
            true
        } else {
            self.systems.push((system_id, wrapper));
            false
        }
    }

    pub fn deregister_system<S: 'static>(&mut self) -> bool {
        let system_id = TypeId::of::<S>();
        let Some(pos) = self.systems.iter().position(|(id, _)| *id == system_id) else {
            return false;
        };

        self.systems.remove(pos);
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
        let Self { entities, systems } = self;

        thread::scope(|scope| {
            for entity in entities.values_mut() {
                let systems = &*systems;
                scope.spawn(move || {
                    let mut passed: Vec<&(TypeId, Box<dyn ISystem>)> =
                        Vec::with_capacity(systems.len());
                    passed.extend(systems.iter().filter(|(system_id, system_impl)| {
                        system_impl.check(&CheckContext::new(*system_id), entity)
                    }));

                    for (system_id, system_impl) in passed {
                        system_impl.and_then(&ActionContext::new(*system_id), entity);
                    }
                });
            }
        });
    }
}
