use crate::entity::{ActionContext, CheckContext, IEntity};

pub trait ISystem<E: IEntity>: Send + Sync {
    fn check(&self, system: &CheckContext, entity: &E) -> bool;
    fn and_then(&self, system: &ActionContext, entity: &E);
}
