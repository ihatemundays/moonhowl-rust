use ecs::{Component, Entity};
use ecs::systems::AllRefreshedSystem;

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

#[allow(dead_code)]
struct Health(i32);

impl Component for Health {}

#[test]
fn single_component_refreshed_is_active_only_after_a_fresh_set() {
    type Refresh = AllRefreshedSystem<Position>;

    let mut entity = Entity::new();
    entity
        .bind_system(Refresh::new())
        .set_component(Position { x: 0.0 })
        .commit();
    assert!(entity.is_system_active::<Refresh>());

    entity.commit();
    assert!(!entity.is_system_active::<Refresh>());

    entity.set_component(Position { x: 1.0 }).commit();
    assert!(entity.is_system_active::<Refresh>());
}

#[test]
fn tuple_refreshed_is_inactive_while_any_member_is_missing() {
    type Refresh = AllRefreshedSystem<(Position, Velocity)>;

    let mut entity = Entity::new();
    entity
        .bind_system(Refresh::new())
        .set_component(Position { x: 0.0 })
        .commit();

    assert!(!entity.is_system_active::<Refresh>());
}

#[test]
fn tuple_refreshed_goes_inactive_once_settled_then_active_again_once_all_members_refresh() {
    type Refresh = AllRefreshedSystem<(Position, Velocity, Health)>;

    let mut entity = Entity::new();
    entity
        .bind_system(Refresh::new())
        .set_component(Position { x: 0.0 })
        .set_component(Velocity { dx: 1.0 })
        .set_component(Health(10))
        .commit();
    assert!(entity.is_system_active::<Refresh>());

    entity.commit();
    assert!(!entity.is_system_active::<Refresh>());

    // only one of three members refreshed -- still inactive
    entity.set_component(Health(9)).commit();
    assert!(!entity.is_system_active::<Refresh>());

    // all three refreshed -- active again
    entity
        .set_component(Position { x: 1.0 })
        .set_component(Velocity { dx: 1.0 })
        .set_component(Health(8))
        .commit();
    assert!(entity.is_system_active::<Refresh>());
}
