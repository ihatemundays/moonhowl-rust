use crate::component::Component;
use crate::entity::Entity;
use std::any::TypeId;

pub trait Archetype {
    type Ref<'a>;
    type RefMut<'a>;

    fn fetch(entity: &Entity) -> Option<Self::Ref<'_>>;
    fn fetch_mut(entity: &mut Entity) -> Option<Self::RefMut<'_>>;
}

impl<T: Component> Archetype for T {
    type Ref<'a> = &'a T;
    type RefMut<'a> = &'a mut T;

    fn fetch(entity: &Entity) -> Option<Self::Ref<'_>> {
        entity.get_component::<T>()
    }

    fn fetch_mut(entity: &mut Entity) -> Option<Self::RefMut<'_>> {
        entity.get_component_mut::<T>()
    }
}

macro_rules! impl_archetype_for_tuple {
    ($(($t:ident, $v:ident)),+) => {
        impl<$($t: Component),+> Archetype for ($($t,)+) {
            type Ref<'a> = ($(&'a $t,)+);
            type RefMut<'a> = ($(&'a mut $t,)+);

            fn fetch(entity: &Entity) -> Option<Self::Ref<'_>> {
                Some(($(entity.get_component::<$t>()?,)+))
            }

            fn fetch_mut(entity: &mut Entity) -> Option<Self::RefMut<'_>> {
                let [$($v,)+] = entity.get_components_mut([$(TypeId::of::<$t>()),+]);
                Some(($(
                    $v.and_then(|component| {
                        (component.as_mut() as &mut dyn std::any::Any).downcast_mut::<$t>()
                    })?,
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
