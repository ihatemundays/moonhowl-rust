use ecs::{Component, Entity};
use ecs::systems::AnyRefreshedSystem;

struct Position { x: f32, y: f32 }
impl Component for Position {}

struct Velocity { dx: f32, dy: f32 }
impl Component for Velocity {}

struct Health(i32);
impl Component for Health {}

type Refresh = AnyRefreshedSystem<(Position, Velocity, Health)>;

fn main() {
    let mut entity = Entity::new();
    entity
        .bind_system(Refresh::new())
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 0.0 })
        .set_component(Health(10))
        .commit();
    println!("1) first commit, all fresh:       {}", entity.is_system_active::<Refresh>());

    entity.commit(); // nothing queued, nothing refreshed
    println!("2) commit again, nothing re-set:  {}", entity.is_system_active::<Refresh>());

    entity.set_component(Health(9)).commit(); // only Health refreshed -- still active
    println!("3) only Health re-set:            {}", entity.is_system_active::<Refresh>());

    entity.commit(); // nothing queued again
    println!("4) commit again, nothing re-set:  {}", entity.is_system_active::<Refresh>());

    entity
        .set_component(Position { x: 1.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 0.0 })
        .set_component(Health(8))
        .commit(); // all three re-set -- active too, since "any" includes "all"
    println!("5) all three re-set:              {}", entity.is_system_active::<Refresh>());

    let (pos, vel, health) = entity.with_archetype::<(Position, Velocity, Health)>().unwrap();
    println!(
        "final state: pos=({}, {}) vel=({}, {}) health={}",
        pos.x, pos.y, vel.dx, vel.dy, health.0
    );
}
