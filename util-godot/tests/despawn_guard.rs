use ecs::World;
use godot::classes::notify::{NodeNotification, ObjectNotification};
use util_godot::DespawnGuard;

#[test]
fn predelete_despawns_the_entity_exactly_once() {
    let mut world = World::new();
    let entity = world.spawn();

    let mut guard = DespawnGuard::new(entity);
    assert!(!guard.despawned());

    guard.handle(&mut world, ObjectNotification::PREDELETE);

    assert!(guard.despawned());
    assert!(!world.is_alive(entity));
}

#[test]
fn node_predelete_is_recognized_too() {
    // NodeNotification and ObjectNotification are separate generated types, but both
    // carry the same inherited PREDELETE value -- `handle` must recognize it either way,
    // since a real `INode::on_notification` override receives `NodeNotification`, not
    // `ObjectNotification`.
    let mut world = World::new();
    let entity = world.spawn();

    let mut guard = DespawnGuard::new(entity);
    guard.handle(&mut world, NodeNotification::PREDELETE);

    assert!(guard.despawned());
    assert!(!world.is_alive(entity));
}

#[test]
fn other_notifications_do_not_despawn() {
    let mut world = World::new();
    let entity = world.spawn();

    let mut guard = DespawnGuard::new(entity);
    guard.handle(&mut world, NodeNotification::READY);
    guard.handle(&mut world, NodeNotification::EXIT_TREE);
    guard.handle(&mut world, NodeNotification::PROCESS);

    assert!(!guard.despawned());
    assert!(world.is_alive(entity));
}

#[test]
fn a_duplicate_predelete_does_not_despawn_a_reused_slot() {
    let mut world = World::new();
    let first = world.spawn();
    let mut guard = DespawnGuard::new(first);

    guard.handle(&mut world, ObjectNotification::PREDELETE);
    assert!(!world.is_alive(first));

    // The freed index gets reused with a bumped generation.
    let second = world.spawn();
    assert_eq!(first.index(), second.index());

    // A duplicate/late PREDELETE on the same (now-stale) guard must not touch whatever
    // now occupies that slot.
    guard.handle(&mut world, ObjectNotification::PREDELETE);
    assert!(world.is_alive(second));
}
