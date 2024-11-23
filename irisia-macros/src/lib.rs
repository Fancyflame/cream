use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use props2::CastProp;
use quote::quote;
use syn::{
    parse::{ParseStream, Parser},
    parse_macro_input, DeriveInput, Item, Result,
};

mod build_macro;
mod derive_props;
mod inner_impl_listen;
mod main_macro;
mod parse_incomplete;
mod props2;
mod split_generics;
mod style;

#[proc_macro]
pub fn style(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as style::StyleMacro)
        .to_token_stream()
        .into()
}

#[proc_macro_attribute]
pub fn main(_: TokenStream, input: TokenStream) -> TokenStream {
    main_macro::main_macro(input)
}

#[proc_macro_derive(Event)]
pub fn derive_event(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident, generics, ..
    } = parse_macro_input!(input as DeriveInput);

    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_gen irisia::Event for #ident #type_gen
        #where_clause
        {}
    }
    .into()
}

#[proc_macro_attribute]
pub fn user_props(attr: TokenStream, input: TokenStream) -> TokenStream {
    result_into_stream(
        derive_props::DeriveProps::parse(attr.into(), parse_macro_input!(input as Item))
            .map(|x| x.compile()),
    )
}

#[proc_macro_attribute]
pub fn props(_: TokenStream, input: TokenStream) -> TokenStream {
    parse_macro_input!(input as CastProp).generate().into()
}

#[proc_macro]
pub fn build(input: TokenStream) -> TokenStream {
    let mut env = build_macro::Environment::new();
    let parser = |input: ParseStream| env.parse_statements(input).map(|stream| quote! {{#stream}});
    result_into_stream(parser.parse(input))
}

fn result_into_stream(result: Result<TokenStream2>) -> TokenStream {
    match result {
        Ok(t) => t.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn __inner_impl_listen(_: TokenStream) -> TokenStream {
    inner_impl_listen::impl_listen().into()
}
