use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{Field, Fields, FieldsNamed, ItemStruct, parse_macro_input, parse_quote};

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
    let mut input = parse_macro_input!(item as ItemStruct);
    let name = input.ident.clone();

    if let Err(error) = add_entity_core_field(&mut input) {
        return error.to_compile_error().into();
    }

    quote! {
        #input

        impl moonhowl_ecs::IEntity for #name {
            fn core(&self) -> &moonhowl_ecs::EntityCore {
                &self.entity_core
            }

            fn core_mut(&mut self) -> &mut moonhowl_ecs::EntityCore {
                &mut self.entity_core
            }
        }
    }
    .into()
}

fn add_entity_core_field(input: &mut ItemStruct) -> syn::Result<()> {
    match &input.fields {
        Fields::Named(fields) => {
            let already_declared = fields.named.iter().any(|field| {
                field
                    .ident
                    .as_ref()
                    .is_some_and(|ident| ident == "entity_core")
            });

            if already_declared {
                return Err(syn::Error::new_spanned(
                    &input.fields,
                    "#[ecs_entity] adds the `entity_core` field itself — remove \
                     your own `entity_core` field",
                ));
            }
        }
        Fields::Unnamed(_) => {
            return Err(syn::Error::new_spanned(
                &input.fields,
                "#[ecs_entity] requires a struct with named fields (or none at all)",
            ));
        }
        Fields::Unit => {}
    }

    let field = entity_core_field();

    match &mut input.fields {
        Fields::Named(fields) => fields.named.push(field),
        Fields::Unit => {
            input.fields = Fields::Named(FieldsNamed {
                brace_token: Default::default(),
                named: Punctuated::from_iter([field]),
            });
        }
        Fields::Unnamed(_) => unreachable!("checked above"),
    }

    Ok(())
}

fn entity_core_field() -> Field {
    let dummy: ItemStruct = parse_quote! {
        struct Dummy {
            entity_core: moonhowl_ecs::EntityCore,
        }
    };

    match dummy.fields {
        Fields::Named(fields) => fields.named.into_iter().next().unwrap(),
        _ => unreachable!(),
    }
}
