use crate::Entity;
use std::any::Any;

pub trait System: Any {
    fn is_lazy(&self) -> bool {
        true
    }

    fn test(&self, e: &Entity) -> bool;
}
