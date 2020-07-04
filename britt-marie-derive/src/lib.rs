#![recursion_limit = "128"]
extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::ToTokens;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, FieldsNamed, FieldsUnnamed};

#[proc_macro_derive(BrittMarie)]
pub fn britt_marie(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as DeriveInput);
    let name = &item.ident;

    if let syn::Data::Struct(ref s) = item.data {
        let mut idents = Vec::new();
        for field in s.fields.iter() {
            match field.ident {
                Some(ref ident) => idents.push((ident.clone(), &field.ty)),
                None => panic!("Struct missing identiy"),
            }
        }

        let mut persist_quotes = Vec::new();
        for (ident, _) in idents.iter() {
            let field_gen = quote! { self.#ident.persist()?; };
            persist_quotes.push(field_gen);
        }

        let mut field_getters = Vec::new();
        for (ident, ty) in idents.iter() {
            let field_gen = quote! { pub fn #ident(&mut self) -> &mut #ty { &mut self.#ident } };
            field_getters.push(field_gen);
        }

        let output: proc_macro2::TokenStream = {
            quote! {
                impl #name  {
                    #[inline]
                    pub fn checkpoint(&self, raw_store: std::rc::Rc<std::cell::RefCell<::britt_marie::RawStore>>) -> Result<(), ::britt_marie::BrittMarieError>{
                        #(#persist_quotes)*
                        raw_store.borrow_mut().checkpoint()
                    }
                    #(#field_getters)*
                }
            }
        };

        return proc_macro::TokenStream::from(output);
    } else {
        panic!("#[derive(BrittMarie)] only works for structs");
    }
}
