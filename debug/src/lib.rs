use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};

#[proc_macro_derive(CustomDebug)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    debug::expand_debug(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

mod debug {
    use proc_macro2::{Span, TokenStream};
    use syn::spanned::Spanned;
    use quote::{quote, quote_spanned};
    use syn::{
        Attribute, Data, DeriveInput, Fields, GenericArgument, Ident, LitStr, PathArguments,
        PathSegment, Result, Type, TypePath, Index
    };
    pub(crate) fn expand_debug(input: &DeriveInput) -> Result<TokenStream> {
        let name = &input.ident;
        let name_str = &name.to_string();
        let fields = handle_data(&input.data);
        let expanded = quote! {
            use std::fmt;
            impl fmt::Debug for #name {
                fn fmt(&self,f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.debug_struct(#name_str)
                    #fields
                    .finish()
                    }
            }
        };
        eprintln!("{}",expanded);
        Ok(expanded)
    }

    fn handle_data(data: &Data) -> TokenStream {
        match *data {
            Data::Struct( ref data)=> {
                match &data.fields {
                    Fields::Named(ref fields) => {
                        let recurse = fields.named.iter().map(|f| {
                           let name = f.ident.as_ref().unwrap();
                           let name_str = &name.to_string();
                           quote_spanned! {f.span() =>
                               .field(#name_str, &self.#name)
                           }
                        });
                     let t = quote! {
                            #(#recurse)*
                        };
                     eprintln!("{}",t);
                     t
                    },
                    Fields::Unnamed(ref fields) => {
                        let recurse = fields.unnamed.iter().enumerate().map(|(i,f)| {
                            let index = Index::from(i);
                            let index_str = i.to_string();
                            quote_spanned! {
                                f.span() => 
                                    .field(#index_str, &self.#index)
                            }
                        });
quote! {
                            #(#recurse)*
                        }
                    },
                    Fields::Unit => {
                        quote!()
                    }
                }
            },
            Data::Enum(_) | Data::Union(_) => unimplemented!(),
        }
    }
}
