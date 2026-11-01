//! The `#[no_alloc]` marker attribute for
//! [`cargo-no-alloc`](https://crates.io/crates/cargo-no-alloc).
//!
//! `#[no_alloc]` marks a function or concrete method for `cargo no-alloc` to
//! check: the checker verifies that the marked instance cannot reach the
//! global allocator. On a normal build the attribute has no effect.
//!
//! ```rust,ignore
//! #[no_alloc_check::no_alloc]
//! fn process(sample: f32) -> f32 {
//!     sample * 0.5
//! }
//! ```
//!
//! Add this lint configuration to the crate using the attribute:
//!
//! ```toml
//! [lints.rust]
//! unexpected_cfgs = { level = "warn", check-cfg = ['cfg(no_alloc_check)'] }
//! ```
//!
//! Then run `cargo no-alloc -- build`. The attribute rejects arguments, async
//! functions, trait methods without a body, and non-function items. See the
//! [repository README](https://github.com/DarkWanderer/no_alloc#readme) and
//! the `cargo-no-alloc` crate for installation and the full command line.
//!
//! # How it works
//!
//! The attribute expands to a cfg-gated tool-attribute marker. `cfg_attr` is
//! evaluated in the *caller's* crate and the `no_alloc_check` cfg is unset on
//! a normal build, so the whole attribute evaluates away: no nightly features,
//! no marker residue, no codegen impact. Under the checker driver
//! (`--cfg no_alloc_check`) it expands to `#[no_alloc_tool::root]`, legible
//! only because the driver also passes
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
        #[cfg_attr(no_alloc_check, no_alloc_tool::root, rustc_no_mir_inline)]
        #tokens
    }
    .into()
}
