use ecs::World;
use godot::classes::{INode, Node};
use godot::obj::Base;
use godot::prelude::{GodotClass, godot_api};
use godot::classes::notify::NodeNotification;
use util_godot::DespawnGuard;

#[derive(GodotClass)]
#[class(base = Node, no_init)]
struct Actor {
    base: Base<Node>,
    despawn_guard: DespawnGuard,
}

#[godot_api]
impl INode for Actor {
    fn on_notification(&mut self, what: NodeNotification) {
        // Wired up like this, the ecs::World that owns `self.despawn_guard`'s
        // entity would live outside this struct (e.g. a global/singleton
        // World the game owns); shown here as a throwaway just to type-check.
        let mut world = World::new();
        self.despawn_guard.handle(&mut world, what);
    }
}

fn main() {}
