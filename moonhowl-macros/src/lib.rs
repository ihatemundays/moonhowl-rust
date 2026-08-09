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
