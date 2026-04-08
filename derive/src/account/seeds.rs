//! Parse `#[seeds(b"prefix", name: Type, ...)]` on account types.

use {
    quote::quote,
    syn::{
        parse::{Parse, ParseStream},
        Expr, ExprLit, Ident, Lit, Token, Type,
    },
};

/// A single dynamic seed in the #[seeds] definition.
pub struct SeedDef {
    pub name: Ident,
    pub ty: Type,
}

/// Parsed #[seeds] attribute.
pub struct SeedsAttr {
    pub prefix: Vec<u8>,
    pub dynamic_seeds: Vec<SeedDef>,
}

impl Parse for SeedsAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // First element: byte string literal
        let prefix_expr: Expr = input.parse()?;
        let prefix = match &prefix_expr {
            Expr::Lit(ExprLit {
                lit: Lit::ByteStr(b),
                ..
            }) => b.value(),
            _ => {
                return Err(syn::Error::new_spanned(
                    prefix_expr,
                    "#[seeds] first argument must be a byte string literal (e.g., b\"vault\")",
                ))
            }
        };

        let mut dynamic_seeds = Vec::new();
        while !input.is_empty() {
            let _: Token![,] = input.parse()?;
            if input.is_empty() {
                break;
            }
            let name: Ident = input.parse()?;
            let _: Token![:] = input.parse()?;
            let ty: Type = input.parse()?;
            dynamic_seeds.push(SeedDef { name, ty });
        }

        Ok(SeedsAttr {
            prefix,
            dynamic_seeds,
        })
    }
}

/// Extract #[seeds(...)] from attributes, if present.
pub fn parse_seeds_attr(attrs: &[syn::Attribute]) -> Option<syn::Result<SeedsAttr>> {
    attrs
        .iter()
        .find(|a| a.path().is_ident("seeds"))
        .map(|a| a.parse_args::<SeedsAttr>())
}

/// Generate the `HasSeeds` impl for an account type.
pub fn generate_seeds_impl(
    name: &syn::Ident,
    seeds_attr: &SeedsAttr,
    has_lifetime: bool,
) -> proc_macro2::TokenStream {
    let prefix_bytes = &seeds_attr.prefix;
    let dynamic_count = seeds_attr.dynamic_seeds.len();

    if has_lifetime {
        quote! {
            impl HasSeeds for #name<'_> {
                const SEED_PREFIX: &'static [u8] = &[#(#prefix_bytes),*];
                const SEED_DYNAMIC_COUNT: usize = #dynamic_count;
            }
        }
    } else {
        quote! {
            impl HasSeeds for #name {
                const SEED_PREFIX: &'static [u8] = &[#(#prefix_bytes),*];
                const SEED_DYNAMIC_COUNT: usize = #dynamic_count;
            }
        }
    }
}
