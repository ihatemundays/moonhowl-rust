use crate::entity::{ActionContext, CheckContext, Entity};

/// A unit of behavior registered against entities of a given marker type.
///
/// For each entity, `check` runs first; if it returns `true`, `and_then`
/// runs next. Mutations made through `and_then`'s [`ActionContext`] are
/// queued and only take effect once
/// [`World::confirm`](crate::World::confirm) is called.
pub trait ISystem: Send + Sync {
    /// Read-only pass deciding whether this system applies to `entity`.
    fn check(&self, system: &CheckContext, entity: &Entity) -> bool;

    /// Acts on `entity` after a passing `check`.
    fn and_then(&self, system: &ActionContext<'_>, entity: &Entity);
}
