use ecs::systems::AllRefreshedSystem;
use ecs::{Component, Entity, System};
use std::cell::Cell;
use std::rc::Rc;

struct Position {
    x: f32,
}

impl Component for Position {}

struct Velocity {
    dx: f32,
}

impl Component for Velocity {}

/// Counts how many times `test` actually ran, so tests can tell whether a
/// `lazy_commit()` call skipped system evaluation entirely or not.
struct CountingSystem(Rc<Cell<u32>>);

impl System for CountingSystem {
    fn test(&self, _entity: &Entity) -> bool {
        self.0.set(self.0.get() + 1);
        true
    }
}

#[test]
fn lazy_commit_applies_queued_commands_just_like_commit() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0 });

    assert!(!entity.has_component::<Position>());

    entity.lazy_commit();

    assert!(entity.has_component::<Position>());
}

#[test]
fn lazy_commit_applies_unset_just_like_commit() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0 }).lazy_commit();
    assert!(entity.has_component::<Position>());

    entity.unset_component::<Position>().lazy_commit();
    assert!(!entity.has_component::<Position>());
}

#[test]
fn lazy_commit_with_no_queued_commands_is_a_no_op() {
    let counter = Rc::new(Cell::new(0));
    let mut entity = Entity::new();
    entity.bind_system(CountingSystem(counter.clone()));

    entity.set_component(Position { x: 1.0 }).commit();
    assert_eq!(counter.get(), 1);

    // nothing queued -- systems should not be re-tested
    entity.lazy_commit();
    assert_eq!(counter.get(), 1);

    // a regular commit always re-tests, queued or not
    entity.commit();
    assert_eq!(counter.get(), 2);

    // queued again -- lazy_commit behaves like commit
    entity.set_component(Velocity { dx: 1.0 }).lazy_commit();
    assert_eq!(counter.get(), 3);
}

#[test]
fn lazy_commit_leaves_system_active_state_untouched_when_nothing_is_queued() {
    type Refresh = AllRefreshedSystem<Position>;

    let mut entity = Entity::new();
    entity.bind_system(Refresh::new());
    entity.set_component(Position { x: 0.0 }).commit();
    assert!(entity.is_system_active::<Refresh>());

    // a regular commit re-tests and finds the address unchanged -- goes inactive
    entity.commit();
    assert!(!entity.is_system_active::<Refresh>());

    // reset back to active, then confirm lazy_commit doesn't flip it this time
    entity.set_component(Position { x: 1.0 }).commit();
    assert!(entity.is_system_active::<Refresh>());

    entity.lazy_commit();
    assert!(entity.is_system_active::<Refresh>());
}

#[test]
fn lazy_commit_returns_the_entity_for_chaining() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 1.0 })
        .lazy_commit()
        .set_component(Velocity { dx: 2.0 })
        .lazy_commit();

    let state = entity
        .with_archetype::<(Position, Velocity)>()
        .map(|(pos, vel)| (pos.x, vel.dx));
    assert_eq!(state, Some((1.0, 2.0)));
}
