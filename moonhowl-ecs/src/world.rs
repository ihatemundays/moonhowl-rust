use crate::entity::{ActionContext, CheckContext, Entity};
use crate::system::ISystem;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;

/// Owns every entity (grouped by marker type) and every registered system.
#[derive(Default)]
pub struct World {
    entities: HashMap<TypeId, HashMap<usize, Entity>>,
    systems: HashMap<TypeId, Vec<(TypeId, Box<dyn ISystem>)>>,
    spawn_queue: Mutex<Vec<(TypeId, Entity)>>,
}

impl World {
    /// Creates an empty world.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new `M`-tagged entity with no components and returns its id.
    pub fn spawn<M: 'static>(&mut self) -> usize {
        let entity = Entity::new::<M>();
        let id = entity.get_id();
        self.entities
            .entry(TypeId::of::<M>())
            .or_default()
            .insert(id, entity);
        id
    }

    /// Removes and returns the `M`-tagged entity with the given id, if any.
    pub fn despawn<M: 'static>(&mut self, id: usize) -> Option<Entity> {
        self.entities.get_mut(&TypeId::of::<M>())?.remove(&id)
    }

    /// Removes every entity of every marker type.
    pub fn despawn_all(&mut self) {
        self.entities.clear();
    }

    /// Whether an `M`-tagged entity with the given id exists.
    pub fn contains<M: 'static>(&self, id: usize) -> bool {
        self.entities
            .get(&TypeId::of::<M>())
            .is_some_and(|bucket| bucket.contains_key(&id))
    }

    /// Returns the `M`-tagged entity with the given id, if any.
    pub fn get_entity<M: 'static>(&self, id: usize) -> Option<&Entity> {
        self.entities.get(&TypeId::of::<M>())?.get(&id)
    }

    /// Mutable version of [`Self::get_entity`].
    pub fn get_entity_mut<M: 'static>(&mut self, id: usize) -> Option<&mut Entity> {
        self.entities.get_mut(&TypeId::of::<M>())?.get_mut(&id)
    }

    /// The total number of entities across every marker type.
    pub fn len(&self) -> usize {
        self.entities.values().map(HashMap::len).sum()
    }

    /// Whether the world has no entities.
    pub fn is_empty(&self) -> bool {
        self.entities.values().all(HashMap::is_empty)
    }

    /// Iterates over every `M`-tagged entity.
    pub fn iter<M: 'static>(&self) -> impl Iterator<Item = &Entity> {
        self.entities
            .get(&TypeId::of::<M>())
            .into_iter()
            .flat_map(|bucket| bucket.values())
    }

    /// Registers `system` to run against `M`-tagged entities. Returns `true`
    /// if it replaced an already-registered `S`, keeping that system's
    /// original registration order; `false` if it was newly registered
    /// (appended after every other system already registered for `M`).
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

    /// Deregisters the `S` system from `M`-tagged entities. Returns `false`
    /// if it wasn't registered.
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

    /// Deregisters every system for every marker type.
    pub fn deregister_all_systems(&mut self) {
        self.systems.clear();
    }

    /// Equivalent to [`Self::despawn_all`] followed by [`Self::deregister_all_systems`].
    pub fn reset(&mut self) {
        self.despawn_all();
        self.deregister_all_systems();
    }

    /// Runs `check` then `and_then` for every entity, one thread per marker
    /// type. Queued mutations aren't applied until [`Self::confirm`] is called.
    pub fn run(&mut self) {
        let Self {
            entities,
            systems,
            spawn_queue,
        } = self;
        let spawn_queue: &Mutex<Vec<(TypeId, Entity)>> = &*spawn_queue;

        thread::scope(|scope| {
            for (type_id, bucket) in entities.iter_mut() {
                let Some(matching_systems) = systems.get(type_id) else {
                    continue;
                };

                scope.spawn(move || Self::run_bucket(bucket, matching_systems, spawn_queue));
            }
        });
    }

    /// Like [`Self::run`], but everything runs on the calling thread.
    pub fn run_sync(&mut self) {
        let Self {
            entities,
            systems,
            spawn_queue,
        } = self;
        let spawn_queue: &Mutex<Vec<(TypeId, Entity)>> = &*spawn_queue;

        for (type_id, bucket) in entities.iter_mut() {
            let Some(matching_systems) = systems.get(type_id) else {
                continue;
            };

            Self::run_bucket(bucket, matching_systems, spawn_queue);
        }
    }

    /// Like [`Self::run`], but `check` runs one thread per marker type while
    /// every `and_then` call happens afterward on the calling thread, in
    /// system registration order per entity.
    pub fn run_checked_sync(&mut self) {
        let Self {
            entities,
            systems,
            spawn_queue,
        } = self;
        let spawn_queue: &Mutex<Vec<(TypeId, Entity)>> = &*spawn_queue;

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

                system_impl.and_then(&ActionContext::new(system_id, spawn_queue), entity);
            }
        }
    }

    /// Applies every component/context/despawn/spawn operation queued via
    /// [`ActionContext`] since the last call to `confirm`.
    pub fn confirm(&mut self) {
        for bucket in self.entities.values_mut() {
            bucket.retain(|_, entity| !entity.commit());
        }

        for (type_id, entity) in self.spawn_queue.get_mut().unwrap().drain(..) {
            let id = entity.get_id();
            self.entities.entry(type_id).or_default().insert(id, entity);
        }
    }

    fn run_bucket(
        bucket: &mut HashMap<usize, Entity>,
        matching_systems: &[(TypeId, Box<dyn ISystem>)],
        spawn_queue: &Mutex<Vec<(TypeId, Entity)>>,
    ) {
        for entity in bucket.values_mut() {
            let mut passed: Vec<&(TypeId, Box<dyn ISystem>)> =
                Vec::with_capacity(matching_systems.len());
            passed.extend(matching_systems.iter().filter(|(system_id, system_impl)| {
                system_impl.check(&CheckContext::new(*system_id), entity)
            }));

            for (system_id, system_impl) in passed {
                system_impl.and_then(&ActionContext::new(*system_id, spawn_queue), entity);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::IComponent;
    use std::any::Any;
    use std::collections::HashSet as StdHashSet;

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Position(i32);

    impl IComponent for Position {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Velocity(i32);

    impl IComponent for Velocity {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Health(i32);

    impl IComponent for Health {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    struct Movable;
    struct Immovable;

    struct AddVelocitySystem;

    impl ISystem for AddVelocitySystem {
        fn check(&self, system: &CheckContext, entity: &Entity) -> bool {
            system.has_component::<Position>(entity)
        }

        fn and_then(&self, system: &ActionContext<'_>, entity: &Entity) {
            let position = *system.get_component::<Position>(entity).unwrap();
            system.set_component(entity, Velocity(position.0 * 2));
        }
    }

    struct SetHealthToOneSystem;

    impl ISystem for SetHealthToOneSystem {
        fn check(&self, system: &CheckContext, entity: &Entity) -> bool {
            system.has_component::<Position>(entity)
        }

        fn and_then(&self, system: &ActionContext<'_>, entity: &Entity) {
            system.set_component(entity, Health(1));
        }
    }

    struct SetHealthToTwoSystem;

    impl ISystem for SetHealthToTwoSystem {
        fn check(&self, system: &CheckContext, entity: &Entity) -> bool {
            system.has_component::<Position>(entity)
        }

        fn and_then(&self, system: &ActionContext<'_>, entity: &Entity) {
            system.set_component(entity, Health(2));
        }
    }

    struct DespawnDeadSystem;

    impl ISystem for DespawnDeadSystem {
        fn check(&self, system: &CheckContext, entity: &Entity) -> bool {
            system
                .get_component::<Health>(entity)
                .is_some_and(|health| health.0 <= 0)
        }

        fn and_then(&self, system: &ActionContext<'_>, entity: &Entity) {
            system.despawn(entity);
        }
    }

    struct SpawnChildSystem;

    impl ISystem for SpawnChildSystem {
        fn check(&self, system: &CheckContext, entity: &Entity) -> bool {
            system.has_component::<Position>(entity)
        }

        fn and_then(&self, system: &ActionContext<'_>, entity: &Entity) {
            let position = *system.get_component::<Position>(entity).unwrap();
            system.spawn::<Movable>(|child| {
                child.set_component(Position(position.0 + 1));
            });
        }
    }

    struct SetContextSystem;

    impl ISystem for SetContextSystem {
        fn check(&self, _system: &CheckContext, _entity: &Entity) -> bool {
            true
        }

        fn and_then(&self, system: &ActionContext<'_>, entity: &Entity) {
            system.set_context(entity, "tagged");
        }
    }

    #[test]
    fn spawn_assigns_and_get_entity_finds_it() {
        let mut world = World::new();
        let id = world.spawn::<Movable>();
        assert!(world.contains::<Movable>(id));
        assert!(world.get_entity::<Movable>(id).is_some());
        assert!(!world.contains::<Immovable>(id));
        assert!(world.get_entity::<Immovable>(id).is_none());
    }

    #[test]
    fn spawn_ids_do_not_collide_within_same_marker_type() {
        let mut world = World::new();
        let mut ids = StdHashSet::new();
        for _ in 0..50 {
            let id = world.spawn::<Movable>();
            assert!(ids.insert(id), "duplicate id returned by spawn");
        }
    }

    #[test]
    fn get_entity_mut_allows_mutation() {
        let mut world = World::new();
        let id = world.spawn::<Movable>();
        world
            .get_entity_mut::<Movable>(id)
            .unwrap()
            .set_component(Position(1));
        assert_eq!(
            world.get_entity::<Movable>(id).unwrap().get_component::<Position>(),
            Some(&Position(1))
        );
    }

    #[test]
    fn get_entity_mut_none_for_missing() {
        let mut world = World::new();
        assert!(world.get_entity_mut::<Movable>(999).is_none());
    }

    #[test]
    fn despawn_removes_and_returns_entity() {
        let mut world = World::new();
        let id = world.spawn::<Movable>();
        assert!(world.despawn::<Movable>(id).is_some());
        assert!(!world.contains::<Movable>(id));
        assert!(world.despawn::<Movable>(id).is_none());
    }

    #[test]
    fn despawn_all_clears_every_bucket() {
        let mut world = World::new();
        world.spawn::<Movable>();
        world.spawn::<Immovable>();
        world.despawn_all();
        assert!(world.is_empty());
        assert_eq!(world.len(), 0);
    }

    #[test]
    fn len_and_is_empty_across_types() {
        let mut world = World::new();
        assert!(world.is_empty());
        world.spawn::<Movable>();
        world.spawn::<Movable>();
        world.spawn::<Immovable>();
        assert_eq!(world.len(), 3);
        assert!(!world.is_empty());
    }

    #[test]
    fn iter_yields_only_matching_marker_type() {
        let mut world = World::new();
        world.spawn::<Movable>();
        world.spawn::<Movable>();
        world.spawn::<Immovable>();
        assert_eq!(world.iter::<Movable>().count(), 2);
        assert_eq!(world.iter::<Immovable>().count(), 1);
    }

    #[test]
    fn register_system_reports_new_vs_replace() {
        let mut world = World::new();
        assert!(!world.register_system::<Movable, _>(AddVelocitySystem));
        assert!(world.register_system::<Movable, _>(AddVelocitySystem));
    }

    #[test]
    fn deregister_system_reports_presence() {
        let mut world = World::new();
        world.register_system::<Movable, _>(AddVelocitySystem);
        assert!(world.deregister_system::<Movable, AddVelocitySystem>());
        assert!(!world.deregister_system::<Movable, AddVelocitySystem>());
        assert!(!world.deregister_system::<Immovable, AddVelocitySystem>());
    }

    #[test]
    fn deregister_all_systems_clears_registry() {
        let mut world = World::new();
        world.register_system::<Movable, _>(AddVelocitySystem);
        world.deregister_all_systems();
        assert!(!world.deregister_system::<Movable, AddVelocitySystem>());
    }

    #[test]
    fn reset_clears_entities_and_systems() {
        let mut world = World::new();
        world.spawn::<Movable>();
        world.register_system::<Movable, _>(AddVelocitySystem);
        world.reset();
        assert!(world.is_empty());
        assert!(!world.deregister_system::<Movable, AddVelocitySystem>());
    }

    #[test]
    fn run_variants_on_empty_world_do_not_panic() {
        let mut world = World::new();
        world.run();
        world.run_sync();
        world.run_checked_sync();
        world.confirm();
        assert!(world.is_empty());
    }

    #[test]
    fn run_sync_queues_changes_that_appear_after_confirm() {
        let mut world = World::new();
        let id = world.spawn::<Movable>();
        world
            .get_entity_mut::<Movable>(id)
            .unwrap()
            .set_component(Position(3));
        world.register_system::<Movable, _>(AddVelocitySystem);

        world.run_sync();
        assert!(world.get_entity::<Movable>(id).unwrap().get_component::<Velocity>().is_none());

        world.confirm();
        assert_eq!(
            world.get_entity::<Movable>(id).unwrap().get_component::<Velocity>(),
            Some(&Velocity(6))
        );
    }

    #[test]
    fn run_applies_same_result_as_run_sync() {
        let mut world = World::new();
        let id = world.spawn::<Movable>();
        world
            .get_entity_mut::<Movable>(id)
            .unwrap()
            .set_component(Position(4));
        world.register_system::<Movable, _>(AddVelocitySystem);

        world.run();
        world.confirm();

        assert_eq!(
            world.get_entity::<Movable>(id).unwrap().get_component::<Velocity>(),
            Some(&Velocity(8))
        );
    }

    #[test]
    fn run_checked_sync_applies_same_result() {
        let mut world = World::new();
        let id = world.spawn::<Movable>();
        world
            .get_entity_mut::<Movable>(id)
            .unwrap()
            .set_component(Position(5));
        world.register_system::<Movable, _>(AddVelocitySystem);

        world.run_checked_sync();
        world.confirm();

        assert_eq!(
            world.get_entity::<Movable>(id).unwrap().get_component::<Velocity>(),
            Some(&Velocity(10))
        );
    }

    #[test]
    fn run_skips_entities_that_fail_check() {
        let mut world = World::new();
        let id = world.spawn::<Movable>(); // no Position component
        world.register_system::<Movable, _>(AddVelocitySystem);

        world.run_sync();
        world.confirm();

        assert!(world.get_entity::<Movable>(id).unwrap().get_component::<Velocity>().is_none());
    }

    #[test]
    fn run_skips_types_with_no_registered_systems() {
        let mut world = World::new();
        let id = world.spawn::<Immovable>();
        world
            .get_entity_mut::<Immovable>(id)
            .unwrap()
            .set_component(Position(1));
        world.register_system::<Movable, _>(AddVelocitySystem); // registered for a different marker

        world.run_sync();
        world.confirm();

        assert!(world.get_entity::<Immovable>(id).unwrap().get_component::<Velocity>().is_none());
    }

    #[test]
    fn run_processes_multiple_marker_types_independently() {
        let mut world = World::new();
        let movable_id = world.spawn::<Movable>();
        world
            .get_entity_mut::<Movable>(movable_id)
            .unwrap()
            .set_component(Position(1));
        let immovable_id = world.spawn::<Immovable>();
        world
            .get_entity_mut::<Immovable>(immovable_id)
            .unwrap()
            .set_component(Position(2));

        world.register_system::<Movable, _>(AddVelocitySystem);
        world.register_system::<Immovable, _>(AddVelocitySystem);

        world.run();
        world.confirm();

        assert_eq!(
            world.get_entity::<Movable>(movable_id).unwrap().get_component::<Velocity>(),
            Some(&Velocity(2))
        );
        assert_eq!(
            world
                .get_entity::<Immovable>(immovable_id)
                .unwrap()
                .get_component::<Velocity>(),
            Some(&Velocity(4))
        );
    }

    #[test]
    fn queued_writes_apply_in_system_registration_order() {
        let mut world = World::new();
        let id = world.spawn::<Movable>();
        world
            .get_entity_mut::<Movable>(id)
            .unwrap()
            .set_component(Position(1));

        world.register_system::<Movable, _>(SetHealthToOneSystem);
        world.register_system::<Movable, _>(SetHealthToTwoSystem);

        world.run_sync();
        world.confirm();

        assert_eq!(
            world.get_entity::<Movable>(id).unwrap().get_component::<Health>(),
            Some(&Health(2))
        );
    }

    #[test]
    fn queued_writes_respect_registration_order_regardless_of_call_order() {
        let mut world = World::new();
        let id = world.spawn::<Movable>();
        world
            .get_entity_mut::<Movable>(id)
            .unwrap()
            .set_component(Position(1));

        world.register_system::<Movable, _>(SetHealthToTwoSystem);
        world.register_system::<Movable, _>(SetHealthToOneSystem);

        world.run_sync();
        world.confirm();

        assert_eq!(
            world.get_entity::<Movable>(id).unwrap().get_component::<Health>(),
            Some(&Health(1))
        );
    }

    #[test]
    fn run_checked_sync_respects_registration_order_too() {
        let mut world = World::new();
        let id = world.spawn::<Movable>();
        world
            .get_entity_mut::<Movable>(id)
            .unwrap()
            .set_component(Position(1));

        world.register_system::<Movable, _>(SetHealthToOneSystem);
        world.register_system::<Movable, _>(SetHealthToTwoSystem);

        world.run_checked_sync();
        world.confirm();

        assert_eq!(
            world.get_entity::<Movable>(id).unwrap().get_component::<Health>(),
            Some(&Health(2))
        );
    }

    #[test]
    fn confirm_with_nothing_queued_is_a_noop() {
        let mut world = World::new();
        let id = world.spawn::<Movable>();
        world
            .get_entity_mut::<Movable>(id)
            .unwrap()
            .set_component(Position(1));
        world.confirm();
        assert_eq!(
            world.get_entity::<Movable>(id).unwrap().get_component::<Position>(),
            Some(&Position(1))
        );
    }

    #[test]
    fn queued_despawn_removes_entity_only_after_confirm() {
        let mut world = World::new();
        let id = world.spawn::<Movable>();
        world.get_entity_mut::<Movable>(id).unwrap().set_component(Health(0));
        world.register_system::<Movable, _>(DespawnDeadSystem);

        world.run_sync();
        assert!(world.contains::<Movable>(id)); // still visible mid-run

        world.confirm();
        assert!(!world.contains::<Movable>(id));
    }

    #[test]
    fn queued_despawn_leaves_other_entities_untouched() {
        let mut world = World::new();
        let dead = world.spawn::<Movable>();
        world.get_entity_mut::<Movable>(dead).unwrap().set_component(Health(0));
        let alive = world.spawn::<Movable>();
        world.get_entity_mut::<Movable>(alive).unwrap().set_component(Health(5));

        world.register_system::<Movable, _>(DespawnDeadSystem);
        world.run_sync();
        world.confirm();

        assert!(!world.contains::<Movable>(dead));
        assert!(world.contains::<Movable>(alive));
    }

    #[test]
    fn queued_spawn_is_invisible_until_confirm_then_present_with_components() {
        let mut world = World::new();
        let id = world.spawn::<Movable>();
        world.get_entity_mut::<Movable>(id).unwrap().set_component(Position(10));
        world.register_system::<Movable, _>(SpawnChildSystem);

        let before = world.len();
        world.run_sync();
        assert_eq!(world.len(), before); // nothing inserted yet

        world.confirm();
        assert_eq!(world.len(), before + 1);

        let child = world
            .iter::<Movable>()
            .find(|entity| entity.get_id() != id)
            .expect("child entity should exist after confirm");
        assert_eq!(child.get_component::<Position>(), Some(&Position(11)));
    }

    #[test]
    fn queued_spawn_id_matches_the_entity_actually_inserted() {
        let mut world = World::new();
        let id = world.spawn::<Movable>();
        world.get_entity_mut::<Movable>(id).unwrap().set_component(Position(0));
        world.register_system::<Movable, _>(SpawnChildSystem);

        world.run_sync();
        world.confirm();

        let child_id = world
            .iter::<Movable>()
            .find(|entity| entity.get_id() != id)
            .unwrap()
            .get_id();
        assert!(world.contains::<Movable>(child_id));
    }

    #[test]
    fn queued_set_context_via_run_and_confirm() {
        let mut world = World::new();
        let id = world.spawn::<Movable>();
        world.register_system::<Movable, _>(SetContextSystem);

        world.run_sync();
        assert!(world.get_entity::<Movable>(id).unwrap().get_context::<&str>().is_none());

        world.confirm();
        assert_eq!(
            world.get_entity::<Movable>(id).unwrap().get_context::<&str>(),
            Some(&"tagged")
        );
    }
}
