use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Fields, GenericArgument, Ident, LitStr,
    PathArguments, PathSegment, Result, Type, TypePath,
};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // eprintln!("INPUT: {:#?}", input);
    let input = parse_macro_input!(input as DeriveInput);
    let tokens = generate_builder(&input);
    eprintln!("TOKENS:\n{}", tokens);
    tokens
}
struct BuilderField<'a> {
    name: &'a Ident,
    kind: TypeKind<'a>,
    each: Option<Ident>,
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

    let fields: Vec<BuilderField> = fields
        .iter()
        .map(|f| {
            let name = f.ident.as_ref().unwrap();
            let kind = classify_type(&f.ty);

            let each = extract_builder_each(&f.attrs).ok().flatten();

            BuilderField { name, kind, each }
        })
        .collect();

    let builder_fields = fields.iter().map(|f| {
        let name = f.name;
        match f.kind {
            TypeKind::Option(ty) => quote! { #name: Option<#ty> },
            TypeKind::Vec(ty) => quote! { #name: Vec<#ty>},
            TypeKind::Other(ty) => quote! { #name: Option<#ty> },
        }
    });

    let builder_init = fields.iter().map(|f| {
        let name = f.name;
        match &f.kind {
            TypeKind::Vec(_) => quote! { #name: Vec::new()},
            _ => quote! {#name: None},
        }
    });

    let field_setters = fields.iter().map(|f| generate_builder_pattern(f));

    let field_set = fields.iter().map(|f| {
        let name = f.name;
        let name_str = name.to_string();
        match &f.kind {
            TypeKind::Other(_) => quote! {
                #name: self.#name.clone().ok_or_else(||format!("field {} is required",#name_str))?
            },
            _ => quote! { #name : self.#name.clone() },
        }
    });

    let expanded = quote! {
        pub struct #builder_name {
            #(#builder_fields,)*
        }

        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #(#builder_init,)*
                }
            }
        }
        use std::error::Error;
        impl #builder_name {
            pub fn build(&mut self) -> Result<#name, Box<dyn Error>> {
                Ok( #name{
                        #(#field_set,)*
                    }
                )
            }
            #(#field_setters)*
        }
    };
    TokenStream::from(expanded)
}

enum TypeKind<'a> {
    Option(&'a Type),
    Vec(&'a Type),
    Other(&'a Type),
}

fn classify_type(ty: &Type) -> TypeKind<'_> {
    if let Type::Path(TypePath { qself: None, path }) = ty {
        if let Some(PathSegment { ident, arguments }) = path.segments.last() {
            if let PathArguments::AngleBracketed(args) = arguments {
                if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                    match ident.to_string().as_str() {
                        "Option" => return TypeKind::Option(inner_ty),
                        "Vec" => return TypeKind::Vec(inner_ty),
                        _ => return TypeKind::Other(ty),
                    }
                }
            }
        }
    }
    TypeKind::Other(ty)
}

fn extract_builder_each(attrs: &Vec<Attribute>) -> Result<Option<Ident>> {
    for attr in attrs {
        if attr.path().is_ident("builder") {
            let mut each_val = None;
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("each") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    each_val = Some(Ident::new(&s.value(), Span::call_site()));
                    Ok(())
                } else {
                    Err(meta.error("unsupported builder property"))
                }
            })?;
            return Ok(each_val);
        }
    }
    Ok(None)
}

fn generate_builder_pattern(field: &BuilderField) -> proc_macro2::TokenStream {
    let name = field.name;
    match field.kind {
        TypeKind::Option(ty) => quote! {
            fn #name(&mut self, #name: #ty) -> &mut Self {
                self.#name = Some(#name);
                self
            }
        },
        TypeKind::Vec(ty) => {
            let mut element_builder = proc_macro2::TokenStream::new();
            if let Some(each_val) = &field.each {
                if name != each_val {
                    element_builder = quote! {
                        fn #each_val(&mut self, #each_val: #ty) -> &mut Self {
                            self.#name.push(#each_val);
                            self
                        }
                    };
                }
            }
            quote! {
                #element_builder

                fn #name(&mut self, #name: Vec<#ty>) -> &mut Self {
                    self.#name = #name;
                    self
                }
            }
        }
        TypeKind::Other(ty) => quote! {
            fn #name(&mut self, #name: #ty) -> &mut Self {
                self.#name = Some(#name);
                self
            }
        },
    }
}
