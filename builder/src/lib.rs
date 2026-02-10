use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Fields, GenericArgument, PathArguments, PathSegment,
    Type, TypePath,
};

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // eprintln!("INPUT: {:#?}", input);
    let input = parse_macro_input!(input as DeriveInput);
    let tokens = generate_builder(&input);
    eprintln!("TOKENS:\n{}", tokens);
    tokens
}

fn generate_builder(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let builder_name = syn::Ident::new(&format!("{}Builder", name), name.span());

    let data = match &input.data {
        Data::Struct(data) => data,
        _ => unimplemented!("Builder can only be derived for structs"),
    };

    let fields = match &data.fields {
        Fields::Named(fields) => &fields.named,
        _ => unimplemented!("Builder can only be derived for structs with named fields"),
    };

    let builder_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;

        if is_option_type(ty) {
            quote! {
                #name : #ty
            }
        } else {
            quote! {
                #name: Option<#ty>
            }
        }
    });

    let builder_struct = quote! {
        pub struct #builder_name {
            #(#builder_fields,)*
        }
    };

    let field_names = fields.iter().map(|f| &f.ident);

    let builder_pattern = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;

        if let Some(inner_ty) = extract_option_inner_type(ty) {
            quote! {
                fn #name(&mut self, #name: #inner_ty) -> &mut Self {
                    self.#name = Some(#name);
                    self
                }
            }
        } else {
            quote! {
                fn #name(&mut self, #name: #ty) -> &mut Self {
                    self.#name = Some(#name);
                    self
                }
            }
        }
    });

    let build_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let name_str = name.as_ref().unwrap().to_string();

        if is_option_type(&f.ty)
        {
            quote! {
                #name : self.#name.clone()
            }
        } else {
            quote! {
                #name : self.#name.clone().ok_or_else(|| format!("field {} is required",#name_str ))?
            }
        }
    });

    let expanded = quote! {
        use std::error::Error;
        #builder_struct

        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #(#field_names : None,)*
                }
            }
        }

        impl #builder_name {
            #(#builder_pattern )*
            pub fn build(&mut self) -> Result<#name, Box<dyn Error>> {
                Ok(#name {
                    #(#build_fields,)*
                })
            }
        }
    };
    proc_macro::TokenStream::from(expanded)
}

fn is_option_type(ty: &Type) -> bool {
    extract_option_inner_type(ty).is_some()
}

fn extract_option_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { qself: None, path }) = ty {
        if let Some(PathSegment { ident, arguments }) = path.segments.last() {
            if ident == "Option" {
                if let PathArguments::AngleBracketed(args) = arguments {
                    if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}
