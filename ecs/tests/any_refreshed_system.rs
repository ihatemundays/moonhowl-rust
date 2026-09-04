use ecs::{Component, Entity};
use ecs::systems::AnyRefreshedSystem;

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
fn single_component_any_refreshed_is_active_only_after_a_fresh_set() {
    type Refresh = AnyRefreshedSystem<Position>;

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
fn tuple_any_refreshed_is_inactive_while_any_member_is_missing() {
    type Refresh = AnyRefreshedSystem<(Position, Velocity)>;

    let mut entity = Entity::new();
    entity
        .bind_system(Refresh::new())
        .set_component(Position { x: 0.0 })
        .commit();

    assert!(!entity.is_system_active::<Refresh>());
}

#[test]
fn tuple_any_refreshed_stays_active_when_only_one_member_keeps_refreshing() {
    type Refresh = AnyRefreshedSystem<(Position, Velocity, Health)>;

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

    // only Health refreshes -- Position/Velocity repeat, but "any" is enough
    entity.set_component(Health(9)).commit();
    assert!(entity.is_system_active::<Refresh>());

    entity.set_component(Health(8)).commit();
    assert!(entity.is_system_active::<Refresh>());
}

#[test]
fn tuple_any_refreshed_goes_inactive_only_once_nothing_at_all_is_re_set() {
    type Refresh = AnyRefreshedSystem<(Position, Velocity, Health)>;

    let mut entity = Entity::new();
    entity
        .bind_system(Refresh::new())
        .set_component(Position { x: 0.0 })
        .set_component(Velocity { dx: 1.0 })
        .set_component(Health(10))
        .commit();
    assert!(entity.is_system_active::<Refresh>());

    entity.set_component(Health(9)).commit();
    assert!(entity.is_system_active::<Refresh>());

    // nothing queued this time -- every member repeats
    entity.commit();
    assert!(!entity.is_system_active::<Refresh>());
}
