use moonhowl_ecs::{Entity, IComponent, ISystem, System, World};
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
    fn run(&self, system: &System, entity: &mut Entity) {
        if !system.has_some_unread_components::<(Position, Velocity)>(entity) {
            return;
        }

        let Some(position) = system.get_component::<Position>(entity) else {
            return;
        };
        let Some(velocity) = system.get_component::<Velocity>(entity) else {
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
        .entity_mut(moving)
        .unwrap()
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 0.5 });

    let stationary = world.spawn();
    world
        .entity_mut(stationary)
        .unwrap()
        .set_component(Position { x: 5.0, y: 5.0 });

    world.register_system(MovementLogger);

    // stationary is skipped: it has no Velocity, so MovementLogger::run
    // returns early on it.
    world.run();
}
