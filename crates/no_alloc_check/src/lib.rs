//! Proc-macro attribute expanding to a cfg-gated tool-attribute marker.
//!
//! On a normal build `cfg_attr` is evaluated in the *caller's* crate and the
//! `no_alloc_check` cfg is unset, so the whole attribute evaluates away:
//! no nightly features, no marker residue, no codegen impact. Under the
//! checker driver (`--cfg no_alloc_check`) it expands to
//! `#[no_alloc_tool::root]`, legible only because the driver also passes
//! `-Zcrate-attr=feature(register_tool)` and
//! `-Zcrate-attr=register_tool(no_alloc_tool)`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{ImplItemFn, ItemFn, TraitItemFn};

#[proc_macro_attribute]
pub fn no_alloc(args: TokenStream, item: TokenStream) -> TokenStream {
    if !args.is_empty() {
        let args = proc_macro2::TokenStream::from(args);
        return syn::Error::new_spanned(args, "#[no_alloc] takes no arguments")
            .to_compile_error()
            .into();
    }

    let tokens = proc_macro2::TokenStream::from(item);
    let (is_async, is_trait_declaration) =
        if let Ok(function) = syn::parse2::<ItemFn>(tokens.clone()) {
            (function.sig.asyncness.is_some(), false)
        } else if let Ok(function) = syn::parse2::<ImplItemFn>(tokens.clone()) {
            (function.sig.asyncness.is_some(), false)
        } else if let Ok(function) = syn::parse2::<TraitItemFn>(tokens.clone()) {
            (function.sig.asyncness.is_some(), function.default.is_none())
        } else {
            return syn::Error::new_spanned(
                tokens,
                "#[no_alloc] can only be applied to functions and methods",
            )
            .to_compile_error()
            .into();
        };

    if is_async {
        return syn::Error::new_spanned(tokens, "#[no_alloc] does not yet support async functions")
            .to_compile_error()
            .into();
    }
    if is_trait_declaration {
        return syn::Error::new_spanned(
            tokens,
            "#[no_alloc] cannot be applied to a trait method without a body",
        )
        .to_compile_error()
        .into();
    }

    quote! {
        #[cfg_attr(no_alloc_check, no_alloc_tool::root)]
        #tokens
    }
    .into()
}
