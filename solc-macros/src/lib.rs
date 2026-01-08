use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(Id)]
pub fn derive_id(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let ty = match &input.data {
        Data::Struct(data) => match data.fields {
            Fields::Unnamed(ref fields) => fields
                .unnamed
                .iter()
                .next()
                .expect("single field tuple struct")
                .ty
                .clone(),
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    };

    quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            pub const DUMMY: Self = Self(#ty::MAX);

            pub fn new(id: #ty) -> Self {
                Self(id)
            }
        }

        impl PartialEq for #name {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        impl Eq for #name { }

        impl ::std::hash::Hash for #name {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.0.hash(state);
            }
        }
    }
    .into()
}
