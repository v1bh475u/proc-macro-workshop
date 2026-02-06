use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input, Data, Fields};
use quote::quote;

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    println!("INPUT: {:#?}", input);
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
        _ => panic!("Builder can only be derived for structs"),
    };

    let fields = match &data.fields {
        Fields::Named(fields) => &fields.named,
        _ => panic!("Builder can only be derived for structs with named fields"),
    };

    let builder_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = f.ty.clone();
        quote! {
            #name: Option<#ty>
        }          
    });

    let builder_struct = quote! {
        pub struct #builder_name {
            #(#builder_fields,)*
        }
    };

    let field_names = fields.iter().map(|f| {
        &f.ident
    });
    
    let builder_pattern = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = f.ty.clone();
        quote! {
            fn #name(&mut self, #name: #ty) -> &mut Self {
                self.#name = Some(#name);
                self
            }
        }
    });


    let expanded =quote! {
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
        }
    };
    proc_macro::TokenStream::from(expanded)
}    