use std::any::Any;

pub trait IComponent: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}
