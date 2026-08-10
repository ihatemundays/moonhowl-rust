use crate::component::IComponent;
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex};

struct ComponentEntry {
    component: Box<dyn IComponent>,
    read_by: Mutex<HashSet<TypeId>>,
}

enum PendingOp {
    SetComponent(TypeId, Box<dyn IComponent>),
    UnsetComponent(TypeId),
    SetContext(Box<dyn Any + Send>),
    ClearContext,
    Despawn,
}

/// An object tagged with a marker type `M`, holding typed components and a
/// single opaque context value.
pub struct Entity {
    id: usize,
    components: HashMap<TypeId, ComponentEntry>,
    context: Option<Box<dyn Any + Send>>,
    pending: Mutex<Vec<PendingOp>>,
}

impl Entity {
    /// Creates a new entity with an id unique among entities created with
    /// the same marker type `M`.
    pub fn new<M: 'static>() -> Self {
        Self {
            id: Self::get_new_id::<M>(),
            components: HashMap::new(),
            context: None,
            pending: Mutex::new(Vec::new()),
        }
    }

    fn get_new_id<M: 'static>() -> usize {
        static COUNTERS: LazyLock<Mutex<HashMap<TypeId, AtomicUsize>>> =
            LazyLock::new(|| Mutex::new(HashMap::new()));

        COUNTERS
            .lock()
            .unwrap()
            .entry(TypeId::of::<M>())
            .or_insert_with(|| AtomicUsize::new(1))
            .fetch_add(1, Ordering::Relaxed)
    }

    /// The id this entity was assigned, unique among entities of its marker type.
    pub fn get_id(&self) -> usize {
        self.id
    }

    fn has_component<T: IComponent>(&self) -> bool {
        self.components.contains_key(&TypeId::of::<T>())
    }

    fn has_some_components<T: ComponentSet>(&self) -> bool {
        T::has_some(self)
    }

    fn has_every_component<T: ComponentSet>(&self) -> bool {
        T::has_every(self)
    }

    fn is_component_read<T: IComponent>(&self, system_id: TypeId) -> bool {
        self.components
            .get(&TypeId::of::<T>())
            .is_some_and(|entry| entry.read_by.lock().unwrap().contains(&system_id))
    }

    fn has_read_component<T: IComponent>(&self, system_id: TypeId) -> bool {
        self.is_component_read::<T>(system_id)
    }

    fn has_some_read_components<T: ComponentSet>(&self, system_id: TypeId) -> bool {
        T::has_some_read(self, system_id)
    }

    fn has_every_read_component<T: ComponentSet>(&self, system_id: TypeId) -> bool {
        T::has_every_read(self, system_id)
    }

    fn has_unread_component<T: IComponent>(&self, system_id: TypeId) -> bool {
        self.components
            .get(&TypeId::of::<T>())
            .is_some_and(|entry| !entry.read_by.lock().unwrap().contains(&system_id))
    }

    fn has_some_unread_components<T: ComponentSet>(&self, system_id: TypeId) -> bool {
        T::has_some_unread(self, system_id)
    }

    fn has_every_unread_component<T: ComponentSet>(&self, system_id: TypeId) -> bool {
        T::has_every_unread(self, system_id)
    }

    fn read_component<T: IComponent>(&self, system_id: TypeId) -> Option<&T> {
        let entry = self.components.get(&TypeId::of::<T>())?;
        let component = entry.component.as_any().downcast_ref::<T>();

        if component.is_some() {
            entry.read_by.lock().unwrap().insert(system_id);
        }

        component
    }

    fn read_components<T: ComponentSet>(&self, system_id: TypeId) -> Option<T::Refs<'_>> {
        T::read_every(self, system_id)
    }

    /// Returns the entity's `T` component, if it has one.
    pub fn get_component<T: IComponent>(&self) -> Option<&T> {
        self.components
            .get(&TypeId::of::<T>())
            .and_then(|entry| entry.component.as_any().downcast_ref::<T>())
    }

    /// Returns every component in the tuple `T`, or `None` if any are missing.
    pub fn get_components<T: ComponentSet>(&self) -> Option<T::Refs<'_>> {
        T::get_every(self)
    }

    /// Sets (inserting or overwriting) the entity's `T` component immediately.
    pub fn set_component<T: IComponent>(&mut self, component: T) -> &mut Self {
        self.components.insert(
            TypeId::of::<T>(),
            ComponentEntry {
                component: Box::new(component),
                read_by: Mutex::new(HashSet::new()),
            },
        );
        self
    }

    /// Removes the entity's `T` component immediately, if present.
    pub fn unset_component<T: IComponent>(&mut self) -> &mut Self {
        self.components.remove(&TypeId::of::<T>());
        self
    }

    fn queue_set_component<T: IComponent>(&self, component: T) {
        self.pending
            .lock()
            .unwrap()
            .push(PendingOp::SetComponent(TypeId::of::<T>(), Box::new(component)));
    }

    fn queue_unset_component<T: IComponent>(&self) {
        self.pending
            .lock()
            .unwrap()
            .push(PendingOp::UnsetComponent(TypeId::of::<T>()));
    }

    fn queue_set_context<C: Any + Send>(&self, context: C) {
        self.pending
            .lock()
            .unwrap()
            .push(PendingOp::SetContext(Box::new(context)));
    }

    fn queue_clear_context(&self) {
        self.pending.lock().unwrap().push(PendingOp::ClearContext);
    }

    fn queue_despawn(&self) {
        self.pending.lock().unwrap().push(PendingOp::Despawn);
    }

    pub(crate) fn commit(&mut self) -> bool {
        let mut despawn = false;

        for op in self.pending.get_mut().unwrap().drain(..) {
            match op {
                PendingOp::SetComponent(type_id, component) => {
                    self.components.insert(
                        type_id,
                        ComponentEntry {
                            component,
                            read_by: Mutex::new(HashSet::new()),
                        },
                    );
                }
                PendingOp::UnsetComponent(type_id) => {
                    self.components.remove(&type_id);
                }
                PendingOp::SetContext(context) => {
                    self.context = Some(context);
                }
                PendingOp::ClearContext => {
                    self.context = None;
                }
                PendingOp::Despawn => {
                    despawn = true;
                }
            }
        }

        despawn
    }

    /// Sets the entity's context value immediately, replacing any existing one.
    pub fn set_context<C: Any + Send>(&mut self, context: C) -> &mut Self {
        self.context = Some(Box::new(context));
        self
    }

    /// Returns the entity's context value, if it has one of type `C`.
    pub fn get_context<C: Any + Send>(&self) -> Option<&C> {
        self.context.as_deref()?.downcast_ref::<C>()
    }

    /// Mutable version of [`Self::get_context`].
    pub fn get_context_mut<C: Any + Send>(&mut self) -> Option<&mut C> {
        self.context.as_deref_mut()?.downcast_mut::<C>()
    }

    /// Clears the entity's context value immediately, if it has one.
    pub fn clear_context(&mut self) -> &mut Self {
        self.context = None;
        self
    }
}

/// Read-only view passed to [`ISystem::check`](crate::ISystem::check),
/// scoped to the currently running system's id.
///
/// The "read"/"unread" queries track, per component per system, whether the
/// *current* system has called `read_component`/`read_components` (via
/// [`ActionContext`]) on that component before, letting a system tell
/// components it has already processed apart from ones it hasn't.
pub struct CheckContext(TypeId);

impl CheckContext {
    pub(crate) fn new(system_id: TypeId) -> Self {
        Self(system_id)
    }

    /// The id of the system this context was created for.
    pub fn get_id(&self) -> TypeId {
        self.0
    }

    /// Whether the entity has a `T` component.
    pub fn has_component<T: IComponent>(&self, entity: &Entity) -> bool {
        entity.has_component::<T>()
    }

    /// Whether this system has read the entity's `T` component before.
    pub fn is_component_read<T: IComponent>(&self, entity: &Entity) -> bool {
        entity.is_component_read::<T>(self.0)
    }

    /// Equivalent to [`Self::is_component_read`].
    pub fn has_read_component<T: IComponent>(&self, entity: &Entity) -> bool {
        entity.has_read_component::<T>(self.0)
    }

    /// Whether the entity has a `T` component this system hasn't read yet.
    pub fn has_unread_component<T: IComponent>(&self, entity: &Entity) -> bool {
        entity.has_unread_component::<T>(self.0)
    }

    /// Returns the entity's `T` component without marking it as read.
    pub fn get_component<'e, T: IComponent>(&self, entity: &'e Entity) -> Option<&'e T> {
        entity.get_component()
    }

    /// Returns every component in the tuple `T` without marking any as read.
    pub fn get_components<'e, T: ComponentSet>(&self, entity: &'e Entity) -> Option<T::Refs<'e>> {
        entity.get_components::<T>()
    }

    /// Whether the entity has at least one component from the tuple `T`.
    pub fn has_some_components<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_some_components::<T>()
    }

    /// Whether the entity has every component in the tuple `T`.
    pub fn has_every_component<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_every_component::<T>()
    }

    /// Whether this system has read at least one component from `T`.
    pub fn has_some_read_components<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_some_read_components::<T>(self.0)
    }

    /// Whether this system has read every component in `T`.
    pub fn has_every_read_component<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_every_read_component::<T>(self.0)
    }

    /// Whether the entity has at least one component from `T` this system hasn't read.
    pub fn has_some_unread_components<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_some_unread_components::<T>(self.0)
    }

    /// Whether every component from `T` the entity has is unread by this system.
    pub fn has_every_unread_component<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_every_unread_component::<T>(self.0)
    }
}

/// Mutable-intent view passed to
/// [`ISystem::and_then`](crate::ISystem::and_then), scoped to the currently
/// running system's id.
///
/// `get_component`/`get_components` read without marking anything as read.
/// `read_component`/`read_components` mark the component as read by this
/// system, which [`CheckContext`]'s read/unread queries observe on
/// subsequent runs. `set_component`, `unset_component`, `set_context`,
/// `clear_context`, `despawn`, and `spawn` are all queued: none of their
/// effects are visible (on the entity, or in the case of `spawn`, in the
/// [`World`](crate::World)) until
/// [`World::confirm`](crate::World::confirm) is called. Queued operations on
/// a given entity are applied in the order the systems touching it ran,
/// i.e. system registration order.
pub struct ActionContext<'w> {
    system_id: TypeId,
    spawn_queue: &'w Mutex<Vec<(TypeId, Entity)>>,
}

impl<'w> ActionContext<'w> {
    pub(crate) fn new(system_id: TypeId, spawn_queue: &'w Mutex<Vec<(TypeId, Entity)>>) -> Self {
        Self {
            system_id,
            spawn_queue,
        }
    }

    /// The id of the system this context was created for.
    pub fn get_id(&self) -> TypeId {
        self.system_id
    }

    /// Returns the entity's `T` component without marking it as read.
    pub fn get_component<'e, T: IComponent>(&self, entity: &'e Entity) -> Option<&'e T> {
        entity.get_component()
    }

    /// Returns every component in the tuple `T` without marking any as read.
    pub fn get_components<'e, T: ComponentSet>(&self, entity: &'e Entity) -> Option<T::Refs<'e>> {
        entity.get_components::<T>()
    }

    /// Returns the entity's `T` component, marking it as read by this system.
    pub fn read_component<'e, T: IComponent>(&self, entity: &'e Entity) -> Option<&'e T> {
        entity.read_component::<T>(self.system_id)
    }

    /// Returns every component in the tuple `T`, marking each as read by this system.
    pub fn read_components<'e, T: ComponentSet>(&self, entity: &'e Entity) -> Option<T::Refs<'e>> {
        entity.read_components::<T>(self.system_id)
    }

    /// Queues `entity`'s `T` component to be set (inserted or overwritten).
    pub fn set_component<T: IComponent>(&self, entity: &Entity, component: T) {
        entity.queue_set_component(component);
    }

    /// Queues `entity`'s `T` component to be removed.
    pub fn unset_component<T: IComponent>(&self, entity: &Entity) {
        entity.queue_unset_component::<T>();
    }

    /// Queues `entity`'s context value to be set, replacing any existing one.
    pub fn set_context<C: Any + Send>(&self, entity: &Entity, context: C) {
        entity.queue_set_context(context);
    }

    /// Queues `entity`'s context value to be cleared.
    pub fn clear_context(&self, entity: &Entity) {
        entity.queue_clear_context();
    }

    /// Queues `entity` to be removed from the world. It stays visible to
    /// other systems for the rest of the current run.
    pub fn despawn(&self, entity: &Entity) {
        entity.queue_despawn();
    }

    /// Builds (via `build`) and queues a new `M`-tagged entity for insertion.
    /// Returns the id it will be inserted under.
    pub fn spawn<M: 'static>(&self, build: impl FnOnce(&mut Entity)) -> usize {
        let mut entity = Entity::new::<M>();
        build(&mut entity);
        let id = entity.get_id();
        self.spawn_queue.lock().unwrap().push((TypeId::of::<M>(), entity));
        id
    }
}

/// Implemented for tuples of up to 8 [`IComponent`] types, enabling
/// multi-component queries like `get_components::<(A, B)>()`.
pub trait ComponentSet {
    type Refs<'a>;

    fn get_every<'a>(entity: &'a Entity) -> Option<Self::Refs<'a>>;
    fn read_every<'a>(entity: &'a Entity, system_id: TypeId) -> Option<Self::Refs<'a>>;
    fn has_some(entity: &Entity) -> bool;
    fn has_every(entity: &Entity) -> bool;
    fn has_some_read(entity: &Entity, system_id: TypeId) -> bool;
    fn has_every_read(entity: &Entity, system_id: TypeId) -> bool;
    fn has_some_unread(entity: &Entity, system_id: TypeId) -> bool;
    fn has_every_unread(entity: &Entity, system_id: TypeId) -> bool;
}

macro_rules! impl_component_set {
    ($($t:ident),+) => {
        impl<$($t: IComponent),+> ComponentSet for ($($t,)+) {
            type Refs<'a> = ($(&'a $t,)+);

            fn get_every<'a>(entity: &'a Entity) -> Option<Self::Refs<'a>> {
                if !Self::has_every(entity) {
                    return None;
                }

                Some(($(entity.get_component::<$t>().unwrap(),)+))
            }

            fn read_every<'a>(entity: &'a Entity, system_id: TypeId) -> Option<Self::Refs<'a>> {
                if !Self::has_every(entity) {
                    return None;
                }

                Some(($(entity.read_component::<$t>(system_id).unwrap(),)+))
            }

            fn has_some(entity: &Entity) -> bool {
                $(entity.has_component::<$t>())||+
            }

            fn has_every(entity: &Entity) -> bool {
                $(entity.has_component::<$t>())&&+
            }

            fn has_some_read(entity: &Entity, system_id: TypeId) -> bool {
                $(entity.has_read_component::<$t>(system_id))||+
            }

            fn has_every_read(entity: &Entity, system_id: TypeId) -> bool {
                $(entity.has_read_component::<$t>(system_id))&&+
            }

            fn has_some_unread(entity: &Entity, system_id: TypeId) -> bool {
                $(entity.has_unread_component::<$t>(system_id))||+
            }

            fn has_every_unread(entity: &Entity, system_id: TypeId) -> bool {
                $(entity.has_unread_component::<$t>(system_id))&&+
            }
        }
    };
}

impl_component_set!(A);
impl_component_set!(A, B);
impl_component_set!(A, B, C);
impl_component_set!(A, B, C, D);
impl_component_set!(A, B, C, D, E);
impl_component_set!(A, B, C, D, E, F);
impl_component_set!(A, B, C, D, E, F, G);
impl_component_set!(A, B, C, D, E, F, G, H);

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Position(i32);

    impl IComponent for Position {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Velocity(i32);

    impl IComponent for Velocity {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Health(i32);

    impl IComponent for Health {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[derive(Debug, PartialEq)]
    struct Ctx(i32);

    struct MarkerA;
    struct MarkerB;

    #[test]
    fn new_assigns_unique_incrementing_ids_per_marker() {
        let a1 = Entity::new::<MarkerA>();
        let a2 = Entity::new::<MarkerA>();
        assert_ne!(a1.get_id(), a2.get_id());
        assert!(a2.get_id() > a1.get_id());
    }

    #[test]
    fn get_set_unset_component_roundtrip() {
        let mut entity = Entity::new::<MarkerA>();
        assert!(entity.get_component::<Position>().is_none());
        assert!(!entity.has_component::<Position>());

        entity.set_component(Position(1));
        assert_eq!(entity.get_component::<Position>(), Some(&Position(1)));
        assert!(entity.has_component::<Position>());

        entity.unset_component::<Position>();
        assert!(entity.get_component::<Position>().is_none());
        assert!(!entity.has_component::<Position>());
    }

    #[test]
    fn set_component_overwrites_existing() {
        let mut entity = Entity::new::<MarkerA>();
        entity.set_component(Position(1));
        entity.set_component(Position(2));
        assert_eq!(entity.get_component::<Position>(), Some(&Position(2)));
    }

    #[test]
    fn component_set_single_tuple() {
        let mut entity = Entity::new::<MarkerA>();
        assert!(!entity.has_every_component::<(Position,)>());

        entity.set_component(Position(7));
        assert!(entity.has_every_component::<(Position,)>());
        assert_eq!(entity.get_components::<(Position,)>(), Some((&Position(7),)));
    }

    #[test]
    fn component_set_has_some_and_has_every() {
        let mut entity = Entity::new::<MarkerA>();
        assert!(!entity.has_some_components::<(Position, Velocity)>());
        assert!(!entity.has_every_component::<(Position, Velocity)>());

        entity.set_component(Position(1));
        assert!(entity.has_some_components::<(Position, Velocity)>());
        assert!(!entity.has_every_component::<(Position, Velocity)>());

        entity.set_component(Velocity(2));
        assert!(entity.has_every_component::<(Position, Velocity)>());
    }

    #[test]
    fn component_set_three_tuple() {
        let mut entity = Entity::new::<MarkerA>();
        entity.set_component(Position(1));
        entity.set_component(Velocity(2));
        entity.set_component(Health(3));

        assert!(entity.has_every_component::<(Position, Velocity, Health)>());
        let (p, v, h) = entity.get_components::<(Position, Velocity, Health)>().unwrap();
        assert_eq!((p, v, h), (&Position(1), &Velocity(2), &Health(3)));
    }

    #[test]
    fn get_components_tuple() {
        let mut entity = Entity::new::<MarkerA>();
        entity.set_component(Position(1));
        entity.set_component(Velocity(2));

        let (position, velocity) = entity.get_components::<(Position, Velocity)>().unwrap();
        assert_eq!(position, &Position(1));
        assert_eq!(velocity, &Velocity(2));
    }

    #[test]
    fn get_components_none_when_missing_one() {
        let mut entity = Entity::new::<MarkerA>();
        entity.set_component(Position(1));
        assert!(entity.get_components::<(Position, Velocity)>().is_none());
    }

    #[test]
    fn read_component_marks_as_read_for_that_system_only() {
        let mut entity = Entity::new::<MarkerA>();
        entity.set_component(Position(1));

        let system_a = TypeId::of::<MarkerA>();
        let system_b = TypeId::of::<MarkerB>();

        assert!(!entity.is_component_read::<Position>(system_a));
        assert!(entity.has_unread_component::<Position>(system_a));
        assert!(!entity.has_read_component::<Position>(system_a));

        assert_eq!(entity.read_component::<Position>(system_a), Some(&Position(1)));

        assert!(entity.is_component_read::<Position>(system_a));
        assert!(entity.has_read_component::<Position>(system_a));
        assert!(!entity.has_unread_component::<Position>(system_a));

        assert!(!entity.is_component_read::<Position>(system_b));
        assert!(entity.has_unread_component::<Position>(system_b));
    }

    #[test]
    fn read_component_returns_none_when_missing() {
        let entity = Entity::new::<MarkerA>();
        assert!(entity.read_component::<Position>(TypeId::of::<MarkerA>()).is_none());
    }

    #[test]
    fn read_components_tuple_marks_all_as_read() {
        let mut entity = Entity::new::<MarkerA>();
        entity.set_component(Position(1));
        entity.set_component(Velocity(2));
        let system_id = TypeId::of::<MarkerA>();

        assert!(!entity.has_every_read_component::<(Position, Velocity)>(system_id));
        assert!(entity.read_components::<(Position, Velocity)>(system_id).is_some());
        assert!(entity.has_every_read_component::<(Position, Velocity)>(system_id));
    }

    #[test]
    fn read_components_none_when_one_missing() {
        let mut entity = Entity::new::<MarkerA>();
        entity.set_component(Position(1));
        let system_id = TypeId::of::<MarkerA>();
        assert!(entity.read_components::<(Position, Velocity)>(system_id).is_none());
    }

    #[test]
    fn has_some_and_every_read_unread_component_sets() {
        let mut entity = Entity::new::<MarkerA>();
        entity.set_component(Position(1));
        entity.set_component(Velocity(2));
        let system_id = TypeId::of::<MarkerA>();

        assert!(entity.has_every_unread_component::<(Position, Velocity)>(system_id));
        assert!(!entity.has_some_read_components::<(Position, Velocity)>(system_id));

        entity.read_component::<Position>(system_id);

        assert!(entity.has_some_read_components::<(Position, Velocity)>(system_id));
        assert!(!entity.has_every_read_component::<(Position, Velocity)>(system_id));
        assert!(entity.has_some_unread_components::<(Position, Velocity)>(system_id));
        assert!(!entity.has_every_unread_component::<(Position, Velocity)>(system_id));
    }

    #[test]
    fn context_roundtrip() {
        let mut entity = Entity::new::<MarkerA>();
        assert!(entity.get_context::<Ctx>().is_none());

        entity.set_context(Ctx(5));
        assert_eq!(entity.get_context::<Ctx>().unwrap().0, 5);

        entity.get_context_mut::<Ctx>().unwrap().0 = 6;
        assert_eq!(entity.get_context::<Ctx>().unwrap().0, 6);

        entity.clear_context();
        assert!(entity.get_context::<Ctx>().is_none());
    }

    #[test]
    fn queued_component_ops_are_invisible_until_commit() {
        let mut entity = Entity::new::<MarkerA>();
        entity.queue_set_component(Position(1));
        assert!(entity.get_component::<Position>().is_none());

        assert!(!entity.commit());
        assert_eq!(entity.get_component::<Position>(), Some(&Position(1)));
    }

    #[test]
    fn queued_unset_component_applies_on_commit() {
        let mut entity = Entity::new::<MarkerA>();
        entity.set_component(Position(1));
        entity.queue_unset_component::<Position>();
        assert!(entity.get_component::<Position>().is_some());

        entity.commit();
        assert!(entity.get_component::<Position>().is_none());
    }

    #[test]
    fn queued_ops_apply_in_queue_order() {
        let mut entity = Entity::new::<MarkerA>();
        entity.queue_set_component(Position(1));
        entity.queue_set_component(Position(2));
        entity.queue_unset_component::<Position>();
        entity.queue_set_component(Position(3));

        entity.commit();
        assert_eq!(entity.get_component::<Position>(), Some(&Position(3)));
    }

    #[test]
    fn queued_set_component_resets_read_tracking() {
        let mut entity = Entity::new::<MarkerA>();
        entity.set_component(Position(1));
        let system_id = TypeId::of::<MarkerA>();
        entity.read_component::<Position>(system_id);
        assert!(entity.is_component_read::<Position>(system_id));

        entity.queue_set_component(Position(2));
        entity.commit();

        assert!(!entity.is_component_read::<Position>(system_id));
    }

    #[test]
    fn queued_context_ops_apply_on_commit() {
        let mut entity = Entity::new::<MarkerA>();
        entity.queue_set_context(Ctx(1));
        assert!(entity.get_context::<Ctx>().is_none());

        entity.commit();
        assert_eq!(entity.get_context::<Ctx>().unwrap().0, 1);

        entity.queue_clear_context();
        entity.commit();
        assert!(entity.get_context::<Ctx>().is_none());
    }

    #[test]
    fn queue_despawn_flags_commit_result() {
        let mut entity = Entity::new::<MarkerA>();
        entity.set_component(Position(1));
        entity.queue_despawn();

        assert!(entity.commit());
        assert_eq!(entity.get_component::<Position>(), Some(&Position(1)));
    }

    #[test]
    fn commit_with_no_pending_ops_is_a_noop() {
        let mut entity = Entity::new::<MarkerA>();
        entity.set_component(Position(1));
        assert!(!entity.commit());
        assert_eq!(entity.get_component::<Position>(), Some(&Position(1)));
    }

    #[test]
    fn commit_drains_the_queue_so_it_does_not_reapply() {
        let mut entity = Entity::new::<MarkerA>();
        entity.queue_set_component(Position(1));
        entity.commit();
        entity.unset_component::<Position>();
        entity.commit();
        assert!(entity.get_component::<Position>().is_none());
    }

    #[test]
    fn check_context_wraps_entity_queries() {
        let mut entity = Entity::new::<MarkerA>();
        entity.set_component(Position(1));
        let system_id = TypeId::of::<MarkerA>();
        let ctx = CheckContext::new(system_id);

        assert_eq!(ctx.get_id(), system_id);
        assert!(ctx.has_component::<Position>(&entity));
        assert!(!ctx.has_component::<Velocity>(&entity));
        assert!(ctx.has_unread_component::<Position>(&entity));
        assert!(!ctx.has_read_component::<Position>(&entity));
        assert!(!ctx.is_component_read::<Position>(&entity));
        assert_eq!(ctx.get_component::<Position>(&entity), Some(&Position(1)));
        assert!(ctx.has_some_components::<(Position, Velocity)>(&entity));
        assert!(!ctx.has_every_component::<(Position, Velocity)>(&entity));
        assert!(!ctx.has_some_read_components::<(Position, Velocity)>(&entity));
        assert!(!ctx.has_every_read_component::<(Position, Velocity)>(&entity));
        assert!(ctx.has_some_unread_components::<(Position, Velocity)>(&entity));
        assert!(!ctx.has_every_unread_component::<(Position, Velocity)>(&entity));
        assert!(ctx.get_components::<(Position, Velocity)>(&entity).is_none());

        entity.set_component(Velocity(2));
        assert!(ctx.has_every_component::<(Position, Velocity)>(&entity));
        assert!(ctx.get_components::<(Position, Velocity)>(&entity).is_some());
    }

    #[test]
    fn action_context_get_and_read_component() {
        let mut entity = Entity::new::<MarkerA>();
        entity.set_component(Position(1));
        entity.set_component(Velocity(2));
        let queue: Mutex<Vec<(TypeId, Entity)>> = Mutex::new(Vec::new());
        let system_id = TypeId::of::<MarkerA>();
        let ctx = ActionContext::new(system_id, &queue);

        assert_eq!(ctx.get_id(), system_id);
        assert_eq!(ctx.get_component::<Position>(&entity), Some(&Position(1)));
        assert!(ctx.get_components::<(Position, Velocity)>(&entity).is_some());

        assert!(!entity.is_component_read::<Position>(system_id));
        assert_eq!(ctx.read_component::<Position>(&entity), Some(&Position(1)));
        assert!(entity.is_component_read::<Position>(system_id));

        assert!(ctx.read_components::<(Position, Velocity)>(&entity).is_some());
    }

    #[test]
    fn action_context_set_and_unset_component_are_queued_not_immediate() {
        let mut entity = Entity::new::<MarkerA>();
        let queue: Mutex<Vec<(TypeId, Entity)>> = Mutex::new(Vec::new());
        let system_id = TypeId::of::<MarkerA>();

        ActionContext::new(system_id, &queue).set_component(&entity, Position(1));
        assert!(entity.get_component::<Position>().is_none());

        entity.commit();
        assert_eq!(entity.get_component::<Position>(), Some(&Position(1)));

        ActionContext::new(system_id, &queue).unset_component::<Position>(&entity);
        assert!(entity.get_component::<Position>().is_some());

        entity.commit();
        assert!(entity.get_component::<Position>().is_none());
    }

    #[test]
    fn action_context_set_and_clear_context_are_queued() {
        let mut entity = Entity::new::<MarkerA>();
        let queue: Mutex<Vec<(TypeId, Entity)>> = Mutex::new(Vec::new());
        let system_id = TypeId::of::<MarkerA>();

        ActionContext::new(system_id, &queue).set_context(&entity, Ctx(9));
        assert!(entity.get_context::<Ctx>().is_none());

        entity.commit();
        assert_eq!(entity.get_context::<Ctx>().unwrap().0, 9);

        ActionContext::new(system_id, &queue).clear_context(&entity);
        assert!(entity.get_context::<Ctx>().is_some());

        entity.commit();
        assert!(entity.get_context::<Ctx>().is_none());
    }

    #[test]
    fn action_context_despawn_is_queued() {
        let mut entity = Entity::new::<MarkerA>();
        let queue: Mutex<Vec<(TypeId, Entity)>> = Mutex::new(Vec::new());
        let system_id = TypeId::of::<MarkerA>();

        ActionContext::new(system_id, &queue).despawn(&entity);
        assert!(entity.commit());
    }

    #[test]
    fn action_context_spawn_queues_a_new_entity_and_returns_its_id() {
        let queue: Mutex<Vec<(TypeId, Entity)>> = Mutex::new(Vec::new());
        let ctx = ActionContext::new(TypeId::of::<MarkerA>(), &queue);

        let id = ctx.spawn::<MarkerB>(|entity| {
            entity.set_component(Position(42));
        });

        let queued = queue.into_inner().unwrap();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].0, TypeId::of::<MarkerB>());
        assert_eq!(queued[0].1.get_id(), id);
        assert_eq!(queued[0].1.get_component::<Position>(), Some(&Position(42)));
    }
}
