use super::*;
use syn::Ident;

pub fn generate_serializable_struct_name(name: &str) -> Ident {
    Ident::new(format!("{}Serializable", name).to_string())
}

pub fn impl_bytes_vec_serde_macro(ast: &syn::DeriveInput) -> quote::Tokens {
    let name = &ast.ident;
    // Decline the struct #name Serializable.
    // let seralizableTokens = impl_serializable_struct_macro(ast, )

    // Impl converting form Serializable to the original structure.
    quote! {
        impl #name {
            fn hello_macro() -> String {
                format!("Hello, Macro! My name is {}", stringify!(#name)).to_string()
            }
        }
    }
}

// fn impl_serializable_struct_macro(ast: &syn::DeriveInput, serializable: Ident) -> quote::Tokens {
//     quote! {
//
//     }
// }
//
// fn impl_convert_to_original_macro(ast: &syn::DeriveInput, serializable: Ident) -> quote::Tokens {
//     let name = &ast.ident;
//     quote! {
//         impl From<#serializable> for #name {
//             fn from(s: #serializable) -> #name {
//                 #name {
//                     ...,
//                 }
//             }
//         }
//     }
// }
