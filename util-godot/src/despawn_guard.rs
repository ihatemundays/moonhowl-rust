use ecs::{Entity, World};
use godot::classes::notify::ObjectNotification;

/// Despawns an `Entity` from a `World` exactly once, on Godot's guaranteed
/// pre-destruction notification.
///
/// Call `handle` from `on_notification` (or whichever `IObject`-derived
/// override your class implements) with whatever notification value it
/// receives; every notification other than `PREDELETE` is ignored, and a
/// second `PREDELETE` (or any call after the first) is a no-op, so it's safe
/// to call unconditionally on every notification without guarding it
/// yourself.
pub struct DespawnGuard {
    entity: Entity,
    despawned: bool,
}

impl DespawnGuard {
    pub fn new(entity: Entity) -> Self {
        Self { entity, despawned: false }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn despawned(&self) -> bool {
        self.despawned
    }

    pub fn handle<N: Into<i32>>(&mut self, world: &mut World, what: N) {
        if self.despawned {
            return;
        }
        if what.into() == i32::from(ObjectNotification::PREDELETE) {
            world.despawn(self.entity);
            self.despawned = true;
        }
    }
}
