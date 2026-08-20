use crate::component::Component;
use crate::entity::Entity;
use crate::sparse_set::SparseSet;
use crate::world::World;
use std::any::TypeId;

pub trait Archetype {
    type Ref<'a>;
    type RefMut<'a>;

    fn type_ids() -> Vec<TypeId>;

    fn fetch(world: &World, entity: Entity) -> Option<Self::Ref<'_>>;
    fn fetch_mut(world: &mut World, entity: Entity) -> Option<Self::RefMut<'_>>;
}

impl<T: Component> Archetype for T {
    type Ref<'a> = &'a T;
    type RefMut<'a> = &'a mut T;

    fn type_ids() -> Vec<TypeId> {
        vec![TypeId::of::<T>()]
    }

    fn fetch(world: &World, entity: Entity) -> Option<Self::Ref<'_>> {
        world.component::<T>(entity)
    }

    fn fetch_mut(world: &mut World, entity: Entity) -> Option<Self::RefMut<'_>> {
        world.component_mut::<T>(entity)
    }
}

macro_rules! impl_archetype_for_tuple {
    ($(($t:ident, $v:ident)),+) => {
        impl<$($t: Component),+> Archetype for ($($t,)+) {
            type Ref<'a> = ($(&'a $t,)+);
            type RefMut<'a> = ($(&'a mut $t,)+);

            fn type_ids() -> Vec<TypeId> {
                vec![$(TypeId::of::<$t>()),+]
            }

            fn fetch(world: &World, entity: Entity) -> Option<Self::Ref<'_>> {
                Some(($(world.component::<$t>(entity)?,)+))
            }

            fn fetch_mut(world: &mut World, entity: Entity) -> Option<Self::RefMut<'_>> {
                let [$($v,)+] = world.disjoint_stores_mut([$(TypeId::of::<$t>()),+]);
                Some(($(
                    $v.and_then(|store| store.as_any_mut().downcast_mut::<SparseSet<$t>>())
                        .and_then(|set| set.get_mut(entity))?,
                )+))
            }
        }
    };
}

impl_archetype_for_tuple!((A, a), (B, b));
impl_archetype_for_tuple!((A, a), (B, b), (C, c));
impl_archetype_for_tuple!((A, a), (B, b), (C, c), (D, d));
impl_archetype_for_tuple!((A, a), (B, b), (C, c), (D, d), (E, e));
impl_archetype_for_tuple!((A, a), (B, b), (C, c), (D, d), (E, e), (F, f));
