use crate::component::Component;
use crate::entity::Entity;

pub trait Archetype {
    type Ref<'a>;

    fn fetch(entity: &Entity) -> Option<Self::Ref<'_>>;
}

impl<T: Component> Archetype for T {
    type Ref<'a> = &'a T;

    fn fetch(entity: &Entity) -> Option<Self::Ref<'_>> {
        entity.get_component::<T>()
    }
}

macro_rules! impl_archetype_for_tuple {
    ($($t:ident),+) => {
        impl<$($t: Component),+> Archetype for ($($t,)+) {
            type Ref<'a> = ($(&'a $t,)+);

            fn fetch(entity: &Entity) -> Option<Self::Ref<'_>> {
                Some(($(entity.get_component::<$t>()?,)+))
            }
        }
    };
}

impl_archetype_for_tuple!(A, B);
impl_archetype_for_tuple!(A, B, C);
impl_archetype_for_tuple!(A, B, C, D);
impl_archetype_for_tuple!(A, B, C, D, E);
impl_archetype_for_tuple!(A, B, C, D, E, F);
