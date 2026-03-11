use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    debug::expand_debug(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

mod debug {
    use proc_macro2::{TokenStream};
    use syn::spanned::Spanned;
    use quote::{quote, quote_spanned};
    use syn::{
        Attribute, Data, DeriveInput, Fields,
        Result, Index, Lit, ExprLit, Meta, Expr, GenericParam, Generics, parse_quote,
    };
    pub(crate) fn expand_debug(input: DeriveInput) -> Result<TokenStream> {
        let name = &input.ident;
        let name_str = &name.to_string();
        let generics = add_trait_bounds(input.generics);
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let fields = handle_data(&input.data)?;
        let expanded = quote! {
            impl #impl_generics ::std::fmt::Debug for #name #ty_generics #where_clause {
                fn fmt(&self,f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    f.debug_struct(#name_str)
                    #fields
                    .finish()
                    }
            }
        };
        eprintln!("{}",expanded);
        Ok(expanded)
    }

    fn add_trait_bounds(mut generics: Generics) -> Generics {
        for param in &mut generics.params {
            if let GenericParam::Type(ref mut type_param) = *param {
                type_param.bounds.push(parse_quote!(std::fmt::Debug));
            }
        }
        generics
    }

    fn handle_data(data: &Data) -> Result<TokenStream> {
        match *data {
            Data::Struct( ref data)=> {
                match &data.fields {
                    Fields::Named(ref fields) => {
                        let recurse = fields.named.iter().map(|f| -> Result<TokenStream>{
                           let name = f.ident.as_ref().unwrap();
                           let name_str = &name.to_string();
                           let field_fmt = extract_debug_fmt(&f.attrs)?;
                           Ok(match field_fmt {
                               Some(fmt) => quote_spanned! {f.span() =>
                                .field(#name_str, &format_args!(#fmt, self.#name))
                                },
                                None => quote_spanned! {f.span() =>
                                    .field(#name_str,&self.#name)
                                },
                            })
                        });
                        let recurse = recurse.collect::<Result<Vec<_>>>()?;
                        let t = quote! {
                               #(#recurse)*
                        };
                        Ok(t)
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
                        Ok(quote! {
                            #(#recurse)*
                        })
                    },
                    Fields::Unit => {
                        Ok(quote!())
                    }
                }
            },
            Data::Enum(_) | Data::Union(_) => unimplemented!(),
        }
    }
    fn extract_debug_fmt(attrs: &Vec<Attribute>) -> Result<Option<String>> {
        for attr in attrs {
            if attr.path().is_ident("debug") {
                if let Meta::NameValue(nv) = &attr.meta {
                    if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = &nv.value {
                        let format_str = lit_str.value();
                        return Ok(Some(format_str));
                    } else {
                        return Err(syn::Error::new_spanned(
                            &nv.value,
                            "expected a string literal, e.g. `#[debug = \"{:#010x}\"]`"
                        ));
                    }
                }
            }
        }
        Ok(None)
    }
}
