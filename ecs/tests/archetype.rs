use ecs::{CommandOrder, Component, Entity};

struct Position {
    x: f32,
    y: f32,
}

impl Component for Position {}

struct Velocity {
    dx: f32,
    dy: f32,
}

impl Component for Velocity {}

struct Name(&'static str);

impl Component for Name {}

struct Health(i32);

impl Component for Health {}

struct Scale(f32);

impl Component for Scale {}

struct Tag(&'static str);

impl Component for Tag {}

#[test]
fn single_component_is_a_one_component_archetype() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 }).commit();

    let pos = entity.with_archetype::<Position>();

    assert_eq!(pos.map(|pos| pos.x + pos.y), Some(3.0));
}

#[test]
fn has_archetype_matches_single_component() {
    let mut entity = Entity::new();
    assert!(!entity.has_archetype::<Position>());

    entity.set_component(Position { x: 1.0, y: 2.0 }).commit();
    assert!(entity.has_archetype::<Position>());
}

#[test]
fn has_archetype_matches_only_when_every_tuple_member_is_present() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 }).commit();

    assert!(!entity.has_archetype::<(Position, Velocity)>());

    entity.set_component(Velocity { dx: 0.5, dy: -0.5 }).commit();
    assert!(entity.has_archetype::<(Position, Velocity)>());

    entity.unset_component::<Velocity>().commit();
    assert!(!entity.has_archetype::<(Position, Velocity)>());
}

#[test]
fn tuple_archetype_matches_only_when_every_component_is_present() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 }).commit();

    assert!(entity.with_archetype::<(Position, Velocity)>().is_none());

    entity.set_component(Velocity { dx: 0.5, dy: -0.5 }).commit();

    let moved = entity
        .with_archetype::<(Position, Velocity)>()
        .map(|(pos, vel)| (pos.x + vel.dx, pos.y + vel.dy));
    assert_eq!(moved, Some((1.5, 1.5)));
}

#[test]
fn larger_tuple_archetype_and_unset_component() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("player"))
        .commit();

    let label = entity
        .with_archetype::<(Position, Velocity, Name)>()
        .map(|(pos, vel, name)| {
            format!("{} at ({}, {}) moving ({}, {})", name.0, pos.x, pos.y, vel.dx, vel.dy)
        });
    assert_eq!(label, Some("player at (0, 0) moving (1, 1)".to_string()));

    entity.unset_component::<Name>().commit();
    assert!(!entity.has_component::<Name>());

    assert!(entity.with_archetype::<(Position, Velocity, Name)>().is_none());
}

#[test]
fn set_component_is_deferred_until_commit() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 });

    assert!(!entity.has_component::<Position>());

    entity.commit();

    assert!(entity.has_component::<Position>());
}

#[test]
fn commands_apply_in_order_on_commit() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 1.0, y: 2.0 })
        .set_component(Position { x: 3.0, y: 4.0 })
        .unset_component::<Position>()
        .commit();

    assert!(!entity.has_component::<Position>());
}

#[test]
fn set_component_overwrites_the_previous_value_on_commit() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 }).commit();

    entity.set_component(Position { x: 11.0, y: 2.0 }).commit();

    let x = entity.with_archetype::<Position>().map(|pos| pos.x);
    assert_eq!(x, Some(11.0));
}

#[test]
fn tuple_archetype_reflects_component_overwrites_after_commit() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: -1.0 })
        .commit();

    entity
        .set_component(Position { x: 1.0, y: -1.0 })
        .set_component(Velocity { dx: 2.0, dy: -1.0 })
        .commit();

    let state = entity
        .with_archetype::<(Position, Velocity)>()
        .map(|(pos, vel)| (pos.x, pos.y, vel.dx, vel.dy));
    assert_eq!(state, Some((1.0, -1.0, 2.0, -1.0)));
}

#[test]
fn unset_component_on_an_absent_type_is_a_harmless_no_op() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 }).commit();

    entity.unset_component::<Velocity>().commit();

    assert!(entity.has_component::<Position>());
    assert!(!entity.has_component::<Velocity>());
}

#[test]
fn three_component_archetype_mutation() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("three"))
        .commit();

    entity.set_component(Position { x: 1.0, y: 1.0 }).commit();

    let position = entity.with_archetype::<Position>().map(|pos| (pos.x, pos.y));
    assert_eq!(position, Some((1.0, 1.0)));

    entity.unset_component::<Name>().commit();
    assert!(entity.with_archetype::<(Position, Velocity, Name)>().is_none());
}

#[test]
fn single_component_archetype_is_none_when_absent() {
    let entity = Entity::new();

    let result = entity.with_archetype::<Position>().map(|pos| pos.x);

    assert_eq!(result, None);
}

#[test]
fn four_component_archetype() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 1.0, y: 1.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("four"))
        .set_component(Health(10))
        .commit();

    let snapshot = entity
        .with_archetype::<(Position, Velocity, Name, Health)>()
        .map(|(pos, vel, name, health)| (pos.x, vel.dx, name.0, health.0));
    assert_eq!(snapshot, Some((1.0, 1.0, "four", 10)));

    entity.unset_component::<Health>().commit();
    assert!(entity.with_archetype::<(Position, Velocity, Name, Health)>().is_none());
}

#[test]
fn four_component_archetype_mutation() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("four"))
        .set_component(Health(10))
        .commit();

    entity
        .set_component(Position { x: 1.0, y: 0.0 })
        .set_component(Health(9))
        .commit();

    let health = entity.with_archetype::<Health>().map(|h| h.0);
    assert_eq!(health, Some(9));

    entity.unset_component::<Name>().commit();
    assert!(entity.with_archetype::<(Position, Velocity, Name, Health)>().is_none());
}

#[test]
fn five_component_archetype() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 1.0, y: 1.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("five"))
        .set_component(Health(10))
        .set_component(Scale(2.0))
        .commit();

    let snapshot = entity
        .with_archetype::<(Position, Velocity, Name, Health, Scale)>()
        .map(|(pos, vel, name, health, scale)| (pos.x, vel.dx, name.0, health.0, scale.0));
    assert_eq!(snapshot, Some((1.0, 1.0, "five", 10, 2.0)));

    entity.unset_component::<Scale>().commit();
    assert!(entity
        .with_archetype::<(Position, Velocity, Name, Health, Scale)>()
        .is_none());
}

#[test]
fn five_component_archetype_mutation() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("five"))
        .set_component(Health(10))
        .set_component(Scale(2.0))
        .commit();

    entity
        .set_component(Position { x: 1.0, y: 0.0 })
        .set_component(Health(9))
        .set_component(Scale(4.0))
        .commit();

    let scale = entity.with_archetype::<Scale>().map(|s| s.0);
    assert_eq!(scale, Some(4.0));

    entity.unset_component::<Health>().commit();
    assert!(entity
        .with_archetype::<(Position, Velocity, Name, Health, Scale)>()
        .is_none());
}

#[test]
fn six_component_archetype() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 1.0, y: 1.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("six"))
        .set_component(Health(10))
        .set_component(Scale(2.0))
        .set_component(Tag("player"))
        .commit();

    let snapshot = entity
        .with_archetype::<(Position, Velocity, Name, Health, Scale, Tag)>()
        .map(|(pos, vel, name, health, scale, tag)| {
            (pos.x, vel.dx, name.0, health.0, scale.0, tag.0)
        });
    assert_eq!(snapshot, Some((1.0, 1.0, "six", 10, 2.0, "player")));

    entity.unset_component::<Tag>().commit();
    assert!(entity
        .with_archetype::<(Position, Velocity, Name, Health, Scale, Tag)>()
        .is_none());
}

#[test]
fn six_component_archetype_mutation() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("six"))
        .set_component(Health(10))
        .set_component(Scale(2.0))
        .set_component(Tag("player"))
        .commit();

    entity
        .set_component(Position { x: 1.0, y: 0.0 })
        .set_component(Health(9))
        .set_component(Scale(4.0))
        .commit();

    let scale = entity.with_archetype::<Scale>().map(|s| s.0);
    assert_eq!(scale, Some(4.0));

    entity.unset_component::<Tag>().commit();
    assert!(entity
        .with_archetype::<(Position, Velocity, Name, Health, Scale, Tag)>()
        .is_none());
}

#[test]
fn duplicate_type_tuple_archetype_reads_the_same_component_twice() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 3.0, y: 4.0 }).commit();

    let sum = entity
        .with_archetype::<(Position, Position)>()
        .map(|(a, b)| a.x + b.y);

    assert_eq!(sum, Some(7.0));
}

#[test]
fn explicit_high_order_applies_last_regardless_of_queue_position() {
    let mut entity = Entity::new();
    entity
        .set_component_with_order(Tag("high"), CommandOrder::High)
        .set_component(Tag("default"))
        .set_component_with_order(Tag("low"), CommandOrder::Low)
        .commit();

    assert_eq!(entity.get_component::<Tag>().map(|tag| tag.0), Some("high"));
}

#[test]
fn explicit_pos_values_sort_numerically_independent_of_queue_order() {
    let mut entity = Entity::new();
    entity
        .set_component_with_order(Tag("second"), CommandOrder::Pos(5))
        .set_component_with_order(Tag("first"), CommandOrder::Pos(1))
        .commit();

    assert_eq!(entity.get_component::<Tag>().map(|tag| tag.0), Some("second"));
}

#[test]
fn explicit_pos_outranks_every_default_seq_ordered_command() {
    let mut entity = Entity::new();
    entity
        .set_component_with_order(Tag("pos"), CommandOrder::Pos(0))
        .set_component(Tag("default_a"))
        .set_component(Tag("default_b"))
        .commit();

    assert_eq!(entity.get_component::<Tag>().map(|tag| tag.0), Some("pos"));
}

#[test]
fn a_later_default_set_still_wins_after_a_low_order_command() {
    let mut entity = Entity::new();
    entity
        .set_component_with_order(Tag("low"), CommandOrder::Low)
        .set_component(Tag("default1"))
        .set_component(Tag("default2"))
        .commit();

    assert_eq!(entity.get_component::<Tag>().map(|tag| tag.0), Some("default2"));
}

#[test]
fn redundant_sets_of_the_same_component_collapse_to_the_last_one() {
    let mut entity = Entity::new();
    entity
        .set_component(Tag("a"))
        .set_component_with_order(Tag("b"), CommandOrder::Low)
        .set_component_with_order(Tag("c"), CommandOrder::High)
        .commit();

    assert_eq!(entity.get_component::<Tag>().map(|tag| tag.0), Some("c"));
}

#[test]
fn a_default_order_unset_does_not_beat_an_explicit_high_order_set() {
    let mut entity = Entity::new();
    entity
        .set_component_with_order(Tag("high"), CommandOrder::High)
        .set_component(Tag("default"))
        .unset_component::<Tag>()
        .commit();

    assert_eq!(entity.get_component::<Tag>().map(|tag| tag.0), Some("high"));
}

#[test]
fn a_high_order_unset_still_beats_a_same_commit_high_order_set() {
    let mut entity = Entity::new();
    entity.set_component(Tag("initial")).commit();

    entity
        .set_component_with_order(Tag("resurrected"), CommandOrder::High)
        .unset_component_with_order::<Tag>(CommandOrder::High)
        .commit();

    assert!(!entity.has_component::<Tag>());
}

#[test]
fn a_low_order_unset_lets_a_later_default_set_rebuild_the_component() {
    let mut entity = Entity::new();
    entity.set_component(Tag("initial")).commit();

    entity
        .unset_component_with_order::<Tag>(CommandOrder::Low)
        .set_component(Tag("rebuilt"))
        .commit();

    assert_eq!(entity.get_component::<Tag>().map(|tag| tag.0), Some("rebuilt"));
}

#[test]
fn equal_rank_conflicts_resolve_in_place_to_whichever_was_queued_last() {
    let mut entity = Entity::new();
    entity
        .set_component_with_order(Tag("first"), CommandOrder::Pos(5))
        .set_component_with_order(Tag("second"), CommandOrder::Pos(5))
        .commit();

    assert_eq!(entity.get_component::<Tag>().map(|tag| tag.0), Some("second"));

    entity
        .set_component_with_order(Tag("set"), CommandOrder::High)
        .unset_component_with_order::<Tag>(CommandOrder::High)
        .commit();

    assert!(!entity.has_component::<Tag>());
}
