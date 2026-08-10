use std::any::Any;

/// A typed piece of data attached to an [`Entity`](crate::Entity).
pub trait IComponent: Any + Send + Sync {
    /// Enables downcasting from type-erased storage back to `Self`.
    fn as_any(&self) -> &dyn Any;
}
