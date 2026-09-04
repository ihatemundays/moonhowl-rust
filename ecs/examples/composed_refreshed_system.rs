use ecs::systems::AllRefreshedSystem;
use ecs::{Component, Entity, System};

struct Position { x: f32, y: f32 }
impl Component for Position {}

struct Velocity { dx: f32, dy: f32 }
impl Component for Velocity {}

struct Health(i32);
impl Component for Health {}

/// Fires only once position/velocity/health are all freshly re-set *and*
/// health is still positive. Composes `AllRefreshedSystem` as a plain field
/// and calls its `test` directly, rather than binding it as its own system
/// and checking `is_system_active` -- within a single `commit()`, systems
/// run in unspecified order, so there's no guarantee `AllRefreshedSystem`
/// would have already run when a sibling system asked about it.
struct RevivedAndMoving {
    refresh: AllRefreshedSystem<(Position, Velocity, Health)>,
}

impl RevivedAndMoving {
    fn new() -> Self {
        Self { refresh: AllRefreshedSystem::new() }
    }
}

impl System for RevivedAndMoving {
    fn test(&self, entity: &Entity) -> bool {
        self.refresh.test(entity) && entity.get_component::<Health>().is_some_and(|h| h.0 > 0)
    }
}

fn main() {
    let mut entity = Entity::new();
    entity
        .bind_system(RevivedAndMoving::new())
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 0.0 })
        .set_component(Health(10))
        .commit();
    println!("1) all fresh, health positive:    {}", entity.is_system_active::<RevivedAndMoving>());

    entity.commit(); // nothing queued, addresses unchanged
    println!("2) commit again, nothing re-set:  {}", entity.is_system_active::<RevivedAndMoving>());

    entity
        .set_component(Position { x: 1.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 0.0 })
        .set_component(Health(0))
        .commit(); // all fresh, but health has dropped to zero
    println!("3) all fresh, health zero:        {}", entity.is_system_active::<RevivedAndMoving>());

    entity
        .set_component(Position { x: 2.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 0.0 })
        .set_component(Health(5))
        .commit(); // all fresh again, health positive again
    println!("4) all fresh, health positive:    {}", entity.is_system_active::<RevivedAndMoving>());

    let (pos, vel, health) = entity.with_archetype::<(Position, Velocity, Health)>().unwrap();
    println!(
        "final state: pos=({}, {}) vel=({}, {}) health={}",
        pos.x, pos.y, vel.dx, vel.dy, health.0
    );
}
