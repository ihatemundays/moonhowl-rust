use crate::entity::{ActionContext, CheckContext, Entity};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct System(usize);

impl System {
    pub fn new() -> Self {
        Self(Self::get_new_id())
    }

    fn get_new_id() -> usize {
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    pub fn get_id(&self) -> usize {
        self.0
    }
}

pub trait ISystem: Send + Sync {
    fn check(&self, system: &CheckContext, entity: &Entity) -> bool;
    fn and_then(&self, system: &ActionContext, entity: &Entity);
}
