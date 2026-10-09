use proc_macro::TokenStream;

mod constraints;

/// todo
/// 
/// ## Examples
/// ```
/// constraint!()
/// ```
#[proc_macro]
pub fn constraint(input: TokenStream) -> TokenStream {
    constraints::constraint(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// todo
/// 
/// ## Examples
/// ```
/// constraints! {}
/// ```
#[proc_macro]
pub fn constraints(input: TokenStream) -> TokenStream {
    constraints::constraints(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// todo
/// 
/// ## Examples
/// ```
/// 
/// ```
#[proc_macro]
pub fn lin(input: TokenStream) -> TokenStream {
    constraints::lin(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
