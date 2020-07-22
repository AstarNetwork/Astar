extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
mod bytes_vec_serde;

#[proc_macro_derive(BytesVecSerde)]
pub fn bytes_vec_serde_derive(input: TokenStream) -> TokenStream {
    // Construct a string representation of the type definition
    let s = input.to_string();

    // Parse the string representation
    let ast = syn::parse_derive_input(&s).unwrap();

    // Build the impl
    let gen = bytes_vec_serde::impl_bytes_vec_serde_macro(&ast);

    // Return the generated impl
    gen.parse().unwrap()
}
