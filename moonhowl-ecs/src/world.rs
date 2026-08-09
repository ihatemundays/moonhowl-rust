use crate::entity::{ActionContext, CheckContext, Entity};
use crate::system::ISystem;
use std::any::TypeId;
use std::collections::HashMap;
use std::thread;

#[derive(Default)]
pub struct World {
    entities: HashMap<TypeId, HashMap<usize, Entity>>,
    systems: HashMap<TypeId, Vec<(TypeId, Box<dyn ISystem>)>>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn<M: 'static>(&mut self) -> usize {
        let entity = Entity::new::<M>();
        let id = entity.get_id();
        self.entities
            .entry(TypeId::of::<M>())
            .or_default()
            .insert(id, entity);
        id
    }

    pub fn despawn<M: 'static>(&mut self, id: usize) -> Option<Entity> {
        self.entities.get_mut(&TypeId::of::<M>())?.remove(&id)
    }

    pub fn despawn_all(&mut self) {
        self.entities.clear();
    }

    pub fn contains<M: 'static>(&self, id: usize) -> bool {
        self.entities
            .get(&TypeId::of::<M>())
            .is_some_and(|bucket| bucket.contains_key(&id))
    }

    pub fn get_entity<M: 'static>(&self, id: usize) -> Option<&Entity> {
        self.entities.get(&TypeId::of::<M>())?.get(&id)
    }

    pub fn get_entity_mut<M: 'static>(&mut self, id: usize) -> Option<&mut Entity> {
        self.entities.get_mut(&TypeId::of::<M>())?.get_mut(&id)
    }

    pub fn len(&self) -> usize {
        self.entities.values().map(HashMap::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.values().all(HashMap::is_empty)
    }

    pub fn iter<M: 'static>(&self) -> impl Iterator<Item = &Entity> {
        self.entities
            .get(&TypeId::of::<M>())
            .into_iter()
            .flat_map(|bucket| bucket.values())
    }

    pub fn register_system<M: 'static, S: ISystem + 'static>(&mut self, system: S) -> bool {
        let system_id = TypeId::of::<S>();
        let wrapper: Box<dyn ISystem> = Box::new(system);

        let bucket = self.systems.entry(TypeId::of::<M>()).or_default();

        if let Some(entry) = bucket.iter_mut().find(|(id, _)| *id == system_id) {
            entry.1 = wrapper;
            true
        } else {
            bucket.push((system_id, wrapper));
            false
        }
    }

    pub fn deregister_system<M: 'static, S: 'static>(&mut self) -> bool {
        let Some(bucket) = self.systems.get_mut(&TypeId::of::<M>()) else {
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
        let Self { entities, systems } = self;

        thread::scope(|scope| {
            for (type_id, bucket) in entities.iter_mut() {
                let Some(matching_systems) = systems.get(type_id) else {
                    continue;
                };

                scope.spawn(move || Self::run_bucket(bucket, matching_systems));
            }
        });
    }
    
    pub fn run_sync(&mut self) {
        let Self { entities, systems } = self;

        for (type_id, bucket) in entities.iter_mut() {
            let Some(matching_systems) = systems.get(type_id) else {
                continue;
            };

            Self::run_bucket(bucket, matching_systems);
        }
    }

    pub fn run_checked_sync(&mut self) {
        let Self { entities, systems } = self;

        let matches: Vec<(TypeId, Vec<(usize, TypeId)>)> = thread::scope(|scope| {
            let handles: Vec<_> = entities
                .iter_mut()
                .filter_map(|(type_id, bucket)| {
                    let matching_systems = systems.get(type_id)?;
                    let type_id = *type_id;
                    Some(scope.spawn(move || (type_id, Self::check_bucket(bucket, matching_systems))))
                })
                .collect();

            handles.into_iter().map(|handle| handle.join().unwrap()).collect()
        });

        for (type_id, entity_matches) in matches {
            let Some(bucket) = entities.get(&type_id) else {
                continue;
            };
            let Some(matching_systems) = systems.get(&type_id) else {
                continue;
            };

            for (entity_id, system_id) in entity_matches {
                let Some(entity) = bucket.get(&entity_id) else {
                    continue;
                };
                let Some((_, system_impl)) = matching_systems.iter().find(|(id, _)| *id == system_id) else {
                    continue;
                };

                system_impl.and_then(&ActionContext::new(system_id), entity);
            }
        }
    }

    fn run_bucket(bucket: &mut HashMap<usize, Entity>, matching_systems: &[(TypeId, Box<dyn ISystem>)]) {
        for entity in bucket.values_mut() {
            let mut passed: Vec<&(TypeId, Box<dyn ISystem>)> =
                Vec::with_capacity(matching_systems.len());
            passed.extend(matching_systems.iter().filter(|(system_id, system_impl)| {
                system_impl.check(&CheckContext::new(*system_id), entity)
            }));

            for (system_id, system_impl) in passed {
                system_impl.and_then(&ActionContext::new(*system_id), entity);
            }
        }
    }

    fn check_bucket(bucket: &HashMap<usize, Entity>, matching_systems: &[(TypeId, Box<dyn ISystem>)]) -> Vec<(usize, TypeId)> {
        let mut matches = Vec::new();

        for (entity_id, entity) in bucket.iter() {
            for (system_id, system_impl) in matching_systems {
                if system_impl.check(&CheckContext::new(*system_id), entity) {
                    matches.push((*entity_id, *system_id));
                }
            }
        }

        matches
    }
}
