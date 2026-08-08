use moonhowl_ecs::{ActionContext, CheckContext, Entity, IComponent, ISystem, World};
use std::any::Any;

struct Position {
    x: f32,
    y: f32,
}

impl IComponent for Position {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct Velocity {
    dx: f32,
    dy: f32,
}

impl IComponent for Velocity {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct MovementLogger;

impl ISystem for MovementLogger {
    fn check(&self, system: &CheckContext, entity: &Entity) -> bool {
        system.has_every_unread_component::<(Position, Velocity)>(entity)
    }

    fn and_then(&self, system: &ActionContext, entity: &Entity) {
        let Some((position, velocity)) = system.read_components::<(Position, Velocity)>(entity)
        else {
            return;
        };

        println!(
            "entity {}: ({}, {}) moving by ({}, {})",
            entity.get_id(),
            position.x,
            position.y,
            velocity.dx,
            velocity.dy,
        );
    }
}

fn main() {
    let mut world = World::new();

    let moving = world.spawn();
    world
        .get_entity_mut(moving)
        .unwrap()
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 0.5 });

    let stationary = world.spawn();
    world
        .get_entity_mut(stationary)
        .unwrap()
        .set_component(Position { x: 5.0, y: 5.0 });

    world.register_system(MovementLogger);

    // stationary is skipped: it has no Velocity, so MovementLogger::check
    // never passes for it.
    world.run();
}
