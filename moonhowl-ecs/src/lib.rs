//! A minimal, from-scratch entity-component-system.
//!
//! Entities ([`Entity`]) are grouped by a marker type `M` and hold typed
//! components plus a single opaque context value. Systems ([`ISystem`]) are
//! registered against a marker type and run in two phases per entity: `check`
//! (read-only, via [`CheckContext`]) decides whether the system applies, and
//! `and_then` (via [`ActionContext`]) acts on it.
//!
//! [`World::run`], [`World::run_sync`], and [`World::run_checked_sync`] all
//! execute the same check/and_then logic with different threading
//! strategies. Mutations made through [`ActionContext`] (`set_component`,
//! `unset_component`, `set_context`, `clear_context`, `despawn`, `spawn`) are
//! queued rather than applied immediately, so a run can be driven from any
//! thread; call [`World::confirm`] afterward to apply them, in system
//! registration order.

mod component;
mod entity;
mod system;
mod world;

pub use component::IComponent;
pub use entity::{ActionContext, CheckContext, ComponentSet, Entity};
pub use system::ISystem;
pub use world::World;
