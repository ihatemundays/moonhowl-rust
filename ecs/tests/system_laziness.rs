use ecs::systems::AllRefreshedSystem;
use ecs::{Component, Entity, System};
use std::cell::Cell;
use std::rc::Rc;

#[allow(dead_code)]
struct Position {
    x: f32,
}

impl Component for Position {}

#[allow(dead_code)]
struct Velocity {
    dx: f32,
}

impl Component for Velocity {}

/// Default `is_lazy()` (true) -- counts how many times `test` actually ran.
struct LazyCountingSystem(Rc<Cell<u32>>);

impl System for LazyCountingSystem {
    fn test(&self, _entity: &Entity) -> bool {
        self.0.set(self.0.get() + 1);
        true
    }
}

/// Explicit `is_lazy() -> false` -- opts back into running every commit.
struct EagerCountingSystem(Rc<Cell<u32>>);

impl System for EagerCountingSystem {
    fn is_lazy(&self) -> bool {
        false
    }

    fn test(&self, _entity: &Entity) -> bool {
        self.0.set(self.0.get() + 1);
        true
    }
}

#[test]
fn is_lazy_defaults_to_true() {
    struct Plain;
    impl System for Plain {
        fn test(&self, _entity: &Entity) -> bool {
            true
        }
    }

    assert!(Plain.is_lazy());
}

#[test]
fn lazy_system_is_skipped_when_a_commit_has_nothing_queued() {
    let counter = Rc::new(Cell::new(0));
    let mut entity = Entity::new();
    entity.bind_system(LazyCountingSystem(counter.clone()));

    entity.set_component(Position { x: 1.0 }).commit();
    assert_eq!(counter.get(), 1);

    // nothing queued -- a lazy system isn't re-tested
    entity.commit();
    assert_eq!(counter.get(), 1);
    entity.commit();
    assert_eq!(counter.get(), 1);

    // queued again -- back to being tested
    entity.set_component(Velocity { dx: 1.0 }).commit();
    assert_eq!(counter.get(), 2);
}

#[test]
fn eager_system_is_tested_every_commit_regardless_of_queued_commands() {
    let counter = Rc::new(Cell::new(0));
    let mut entity = Entity::new();
    entity.bind_system(EagerCountingSystem(counter.clone()));

    entity.set_component(Position { x: 1.0 }).commit();
    assert_eq!(counter.get(), 1);

    entity.commit(); // nothing queued -- still re-tested
    assert_eq!(counter.get(), 2);
    entity.commit();
    assert_eq!(counter.get(), 3);
}

#[test]
fn commit_still_applies_queued_commands_regardless_of_any_bound_systems_laziness() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0 }).commit();
    assert!(entity.has_component::<Position>());
}

#[test]
fn all_refreshed_system_overrides_is_lazy_so_it_still_decays_to_inactive() {
    type Refresh = AllRefreshedSystem<Position>;

    let mut entity = Entity::new();
    entity.bind_system(Refresh::new());
    entity.set_component(Position { x: 0.0 }).commit();
    assert!(entity.is_system_active::<Refresh>());

    // nothing queued -- but AllRefreshedSystem is not lazy, so it's still
    // re-tested and correctly goes inactive once addresses stop changing
    entity.commit();
    assert!(!entity.is_system_active::<Refresh>());
}
