use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemStruct, parse_macro_input};

#[proc_macro_attribute]
pub fn ecs_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;

    quote! {
        #input

        impl moonhowl_ecs::IComponent for #name {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn ecs_entity(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;

    quote! {
        #input

        impl moonhowl_ecs::IEntity for #name {
            fn get_core(&self) -> &moonhowl_ecs::EntityCore {
                &self.entity_core
            }

            fn get_core_mut(&mut self) -> &mut moonhowl_ecs::EntityCore {
                &mut self.entity_core
            }
        }
    }
    .into()
}
