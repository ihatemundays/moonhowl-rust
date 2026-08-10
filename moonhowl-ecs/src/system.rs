use crate::entity::{ActionContext, CheckContext, Entity};

pub trait ISystem: Send + Sync {
    fn check(&self, system: &CheckContext, entity: &Entity) -> bool;
    fn and_then(&self, system: &ActionContext<'_>, entity: &Entity);
}
