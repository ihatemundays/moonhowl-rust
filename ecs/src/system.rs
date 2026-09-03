use crate::Entity;
use std::any::Any;

pub trait System: Any {
    fn test(&self, e: &Entity) -> bool;
}
