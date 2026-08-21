use crate::component::Component;
use crate::entity::Entity;
use crate::table::ColumnMap;
use crate::world::World;
use std::any::TypeId;

pub trait Archetype: 'static {
    type Ref<'a>;
    type RefMut<'a>;
    /// The columns this archetype reads from, resolved once per matching
    /// table instead of once per entity -- see `Archetype::resolve_columns`.
    type Columns<'a>: 'a;
    /// Same idea as `Archetype::Columns`, for `_mut` queries.
    type ColumnsMut<'a>: 'a;

    fn type_ids() -> Vec<TypeId>;

    fn fetch(world: &World, entity: Entity) -> Option<Self::Ref<'_>>;
    fn fetch_mut(world: &mut World, entity: Entity) -> Option<Self::RefMut<'_>>;

    /// Looks each component's column up by `TypeId` and downcasts it once
    /// per matching table. Bulk queries call this once per table and then
    /// index the returned columns directly per row, instead of repeating
    /// the lookup/downcast for every entity.
    fn resolve_columns(columns: &ColumnMap) -> Option<Self::Columns<'_>>;
    fn resolve_columns_mut(columns: &mut ColumnMap) -> Option<Self::ColumnsMut<'_>>;

    fn row<'a>(columns: &Self::Columns<'a>, row: usize) -> Self::Ref<'a>;
    fn row_mut<'a>(columns: &'a mut Self::ColumnsMut<'_>, row: usize) -> Self::RefMut<'a>;
    /// Splits every column in `columns` at row `mid`, returning (first `mid`
    /// rows, the rest) -- used to peel off disjoint row chunks for parallel
    /// mutation. Takes `columns` by value (they're already owned slice
    /// values, not a borrow of them) so both halves keep the original
    /// lifetime instead of being shortened to a fresh reborrow, the same way
    /// `<[T]>::split_at_mut` does.
    fn split_columns_mut<'a>(columns: Self::ColumnsMut<'a>, mid: usize) -> (Self::ColumnsMut<'a>, Self::ColumnsMut<'a>);
}

impl<T: Component> Archetype for T {
    type Ref<'a> = &'a T;
    type RefMut<'a> = &'a mut T;
    type Columns<'a> = &'a [T];
    type ColumnsMut<'a> = &'a mut [T];

    fn type_ids() -> Vec<TypeId> {
        vec![TypeId::of::<T>()]
    }

    fn fetch(world: &World, entity: Entity) -> Option<Self::Ref<'_>> {
        world.component::<T>(entity)
    }

    fn fetch_mut(world: &mut World, entity: Entity) -> Option<Self::RefMut<'_>> {
        world.component_mut::<T>(entity)
    }

    fn resolve_columns(columns: &ColumnMap) -> Option<Self::Columns<'_>> {
        Some(columns.get(&TypeId::of::<T>())?.as_any().downcast_ref::<Vec<T>>()?.as_slice())
    }

    fn resolve_columns_mut(columns: &mut ColumnMap) -> Option<Self::ColumnsMut<'_>> {
        Some(columns.get_mut(&TypeId::of::<T>())?.as_any_mut().downcast_mut::<Vec<T>>()?.as_mut_slice())
    }

    fn row<'a>(columns: &Self::Columns<'a>, row: usize) -> Self::Ref<'a> {
        &columns[row]
    }

    fn row_mut<'a>(columns: &'a mut Self::ColumnsMut<'_>, row: usize) -> Self::RefMut<'a> {
        &mut columns[row]
    }

    fn split_columns_mut<'a>(columns: Self::ColumnsMut<'a>, mid: usize) -> (Self::ColumnsMut<'a>, Self::ColumnsMut<'a>) {
        columns.split_at_mut(mid)
    }
}

macro_rules! impl_archetype_for_tuple {
    ($(($t:ident, $v:ident)),+) => {
        impl<$($t: Component),+> Archetype for ($($t,)+) {
            type Ref<'a> = ($(&'a $t,)+);
            type RefMut<'a> = ($(&'a mut $t,)+);
            type Columns<'a> = ($(&'a [$t],)+);
            type ColumnsMut<'a> = ($(&'a mut [$t],)+);

            fn type_ids() -> Vec<TypeId> {
                vec![$(TypeId::of::<$t>()),+]
            }

            fn fetch(world: &World, entity: Entity) -> Option<Self::Ref<'_>> {
                Some(($(world.component::<$t>(entity)?,)+))
            }

            fn fetch_mut(world: &mut World, entity: Entity) -> Option<Self::RefMut<'_>> {
                let loc = world.location(entity)?;
                let table = world.registry_mut().table_mut(loc.table);
                let row = loc.row as usize;
                let [$($v,)+] = table.columns_mut().get_disjoint_mut([$(&TypeId::of::<$t>()),+]);
                Some(($(
                    $v.and_then(|c| c.as_any_mut().downcast_mut::<Vec<$t>>())
                        .and_then(|v| v.get_mut(row))?,
                )+))
            }

            fn resolve_columns(columns: &ColumnMap) -> Option<Self::Columns<'_>> {
                Some(($(columns.get(&TypeId::of::<$t>())?.as_any().downcast_ref::<Vec<$t>>()?.as_slice(),)+))
            }

            fn resolve_columns_mut(columns: &mut ColumnMap) -> Option<Self::ColumnsMut<'_>> {
                let [$($v,)+] = columns.get_disjoint_mut([$(&TypeId::of::<$t>()),+]);
                Some(($($v?.as_any_mut().downcast_mut::<Vec<$t>>()?.as_mut_slice(),)+))
            }

            fn row<'a>(columns: &Self::Columns<'a>, row: usize) -> Self::Ref<'a> {
                let ($($v,)+) = *columns;
                ($(&$v[row],)+)
            }

            fn row_mut<'a>(columns: &'a mut Self::ColumnsMut<'_>, row: usize) -> Self::RefMut<'a> {
                let ($($v,)+) = columns;
                ($(&mut $v[row],)+)
            }

            fn split_columns_mut<'a>(columns: Self::ColumnsMut<'a>, mid: usize) -> (Self::ColumnsMut<'a>, Self::ColumnsMut<'a>) {
                let ($($v,)+) = columns;
                $(let $v = $v.split_at_mut(mid);)+
                (($($v.0,)+), ($($v.1,)+))
            }
        }
    };
}

impl_archetype_for_tuple!((A, a), (B, b));
impl_archetype_for_tuple!((A, a), (B, b), (C, c));
impl_archetype_for_tuple!((A, a), (B, b), (C, c), (D, d));
impl_archetype_for_tuple!((A, a), (B, b), (C, c), (D, d), (E, e));
impl_archetype_for_tuple!((A, a), (B, b), (C, c), (D, d), (E, e), (F, f));
