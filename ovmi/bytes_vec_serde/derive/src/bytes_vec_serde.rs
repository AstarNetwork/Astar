use super::*;
use quote::quote;
use quote::ToTokens;
use syn::{Body, DeriveInput, Ident};

pub fn generate_serializable_struct_name(name: &str) -> Ident {
    Ident::new(format!("{}Serializable", name).to_string())
}

pub fn impl_bytes_vec_serde_macro(ast: &syn::DeriveInput) -> quote::Tokens {
    let name = &ast.ident;
    let serializable_name = generate_serializable_struct_name(&name.to_string());
    // Decline the struct #name Serializable.
    let seralizableTokens = derive(&serializable_name, &ast.body);
    println!("{:?}", seralizableTokens);

    // Impl converting form Serializable to the original structure.
    let convertingTokens = impl_convert_to_original_macro(ast, &serializable_name);
    println!("{:?}", convertingTokens);
    quote! {
        #seralizableTokens
        #convertingTokens
    }
}

/// Derive the inner implementation of `Debug::fmt` function.
pub fn derive(name_ident: &Ident, data: &syn::Body) -> quote::Tokens {
    match *data {
        Body::Struct(ref s) => derive_struct(name_ident, &s.fields()),
        _ => quote! { #name_ident },
    }
}

pub fn change_from_vec_u8_to_string(from: &str) -> Ident {
    let new_type_name = from.replace("Vec < u8 >", "String");
    Ident::new(new_type_name)
}

pub fn change_type_name(ty: &syn::Ty) -> Ident {
    change_from_vec_u8_to_string(quote! {#ty}.as_str())
}

pub fn derive_struct(name_ident: &Ident, fields: &[syn::Field]) -> quote::Tokens {
    for field in fields.iter() {
        let vis_ident = &field.vis;
        let field_ident = &field.ident;
        let type_ident = change_type_name(&field.ty);
        println!(
            "derive_struct: {:?}",
            quote! { #vis_ident #field_ident: #type_ident }
        );
    }
    let vis_idents: Vec<&syn::Visibility> = fields.iter().map(|field| &field.vis).collect();
    let field_idents: Vec<Option<&Ident>> =
        fields.iter().map(|field| field.ident.as_ref()).collect();
    let type_idents: Vec<Ident> = fields
        .iter()
        .map(|field| change_type_name(&field.ty))
        .collect();
    quote! {
        #[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
        struct #name_ident {
            #(#vis_idents #field_idents: #type_idents,)*
        }
    }
}

pub fn change_if_str_to_vec(ty: &syn::Ty) -> Option<Ident> {
    let ty_str = (quote! {#ty}).as_str();
    match ty_str.find("Vec < u8 >") {
        Some(_) => match ty_str.find("Vec < Vec < u8 > >") {
            Some(_) => Some(Ident::new("map(|s| s.to_bytes()).collect()")),
            None => Some(Ident::new(".to_bytes()")),
        },
        None => None,
    }
}

fn impl_convert_to_original_macro(ast: &syn::DeriveInput, serializable: Ident) -> quote::Tokens {
    let name = &ast.ident;
    let field_idents: Vec<Option<&Ident>> =
        fields.iter().map(|field| field.ident.as_ref()).collect();
    let to_bytes_indents: Vec<Option<Ident>> = field
        .iter()
        .map(|field| change_if_str_to_vec(&field.ty))
        .collect();
    quote! {
        impl From<#serializable> for #name {
            fn from(s: #serializable) -> #name {
                #name {
                    #(s.#field_idents#to_bytes_indents,)*
                }
            }
        }
    }
}
