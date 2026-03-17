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
    use proc_macro2::TokenStream;
    use quote::{quote, quote_spanned};
    use std::collections::HashSet;
    use syn::{
        parse_quote, Attribute, Data, DeriveInput, Expr, ExprLit, Fields, GenericParam, Ident,
        Index, Lit, Meta, Result, Type,
    };
    use syn::{spanned::Spanned, GenericArgument, PathArguments};

    pub(crate) fn expand_debug(mut input: DeriveInput) -> Result<TokenStream> {
        let name = &input.ident;
        let name_str = &name.to_string();
        let to_modify: HashSet<Ident> = input
            .generics
            .params
            .iter()
            .filter_map(|param| {
                if only_used_in_phantom_data(&input, param) {
                    None
                } else {
                    Some(generic_param_to_ident(param).clone())
                }
            })
            .collect();
        for param in input.generics.params.iter_mut() {
            if let GenericParam::Type(type_param) = param {
                if to_modify.contains(&type_param.ident) {
                    type_param.bounds.push(parse_quote!(std::fmt::Debug));
                }
            }
        }
        let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

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
        eprintln!("{}", expanded);
        Ok(expanded)
    }
    fn only_used_in_phantom_data(input: &DeriveInput, param: &GenericParam) -> bool {
        let fields = match input.data {
            Data::Struct(ref data) => &data.fields,
            _ => unimplemented!(),
        };
        let param = generic_param_to_ident(param);

        fields.iter().all(|f| {
            let ty = &f.ty;
            !contains_ident(ty, param) || is_phantom_data_of(ty, param)
        })
    }

    fn is_phantom_data_of(ty: &Type, param: &Ident) -> bool {
        let Type::Path(tp) = ty else { return false };
        let seg = tp.path.segments.last().unwrap();
        if seg.ident != "PhantomData" {
            return false;
        }
        let PathArguments::AngleBracketed(args) = &seg.arguments else {
            return false;
        };
        args.args
            .iter()
            .any(|a| matches!(a, GenericArgument::Type(t) if contains_ident(t, param)))
    }

    fn contains_ident(ty: &Type, param: &Ident) -> bool {
        let Type::Path(tp) = ty else { return false };
        tp.path.segments.iter().any(|s| s.ident == *param)
    }

    fn generic_param_to_ident(param: &GenericParam) -> &Ident {
        match param {
            GenericParam::Type(t) => &t.ident,
            GenericParam::Const(c) => &c.ident,
            GenericParam::Lifetime(l) => &l.lifetime.ident,
        }
    }

    fn handle_data(data: &Data) -> Result<TokenStream> {
        match *data {
            Data::Struct(ref data) => match &data.fields {
                Fields::Named(ref fields) => {
                    let recurse = fields.named.iter().map(|f| -> Result<TokenStream> {
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
                }
                Fields::Unnamed(ref fields) => {
                    let recurse = fields.unnamed.iter().enumerate().map(|(i, f)| {
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
                }
                Fields::Unit => Ok(quote!()),
            },
            Data::Enum(_) | Data::Union(_) => unimplemented!(),
        }
    }
    fn extract_debug_fmt(attrs: &Vec<Attribute>) -> Result<Option<String>> {
        for attr in attrs {
            if attr.path().is_ident("debug") {
                if let Meta::NameValue(nv) = &attr.meta {
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(lit_str),
                        ..
                    }) = &nv.value
                    {
                        let format_str = lit_str.value();
                        return Ok(Some(format_str));
                    } else {
                        return Err(syn::Error::new_spanned(
                            &nv.value,
                            "expected a string literal, e.g. `#[debug = \"{:#010x}\"]`",
                        ));
                    }
                }
            }
        }
        Ok(None)
    }
}
