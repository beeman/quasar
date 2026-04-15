//! Emit bump infrastructure directly from `FieldSemantics`.

use {
    super::super::{
        seeds::{render_seed_expr, seeds_to_emit_nodes, SeedEmitNode, SeedRenderContext},
        semantics::FieldSemantics,
    },
    quote::{format_ident, quote},
};

pub(super) fn emit_bump_vars(semantics: &[FieldSemantics]) -> proc_macro2::TokenStream {
    let vars: Vec<proc_macro2::TokenStream> = semantics
        .iter()
        .filter(|sem| sem.pda.is_some())
        .map(|sem| {
            let var = format_ident!("__bumps_{}", sem.core.ident);
            quote! { let mut #var: u8 = 0; }
        })
        .collect();

    if vars.is_empty() {
        quote! {}
    } else {
        quote! { #(#vars)* }
    }
}

pub(super) fn emit_bump_struct(
    semantics: &[FieldSemantics],
    bumps_name: &syn::Ident,
) -> proc_macro2::TokenStream {
    let fields: Vec<proc_macro2::TokenStream> = semantics
        .iter()
        .filter(|sem| sem.pda.is_some())
        .flat_map(|sem| {
            let name = &sem.core.ident;
            let arr_name = format_ident!("__{}_bump", name);
            vec![quote! { pub #name: u8 }, quote! { pub #arr_name: [u8; 1] }]
        })
        .collect();

    if fields.is_empty() {
        quote! { #[derive(Copy, Clone)] pub struct #bumps_name; }
    } else {
        quote! { #[derive(Copy, Clone)] pub struct #bumps_name { #(#fields,)* } }
    }
}

pub(super) fn emit_bump_init(
    semantics: &[FieldSemantics],
    bumps_name: &syn::Ident,
) -> proc_macro2::TokenStream {
    let inits: Vec<proc_macro2::TokenStream> = semantics
        .iter()
        .filter(|sem| sem.pda.is_some())
        .flat_map(|sem| {
            let name = &sem.core.ident;
            let var = format_ident!("__bumps_{}", name);
            let arr_name = format_ident!("__{}_bump", name);
            vec![quote! { #name: #var }, quote! { #arr_name: [#var] }]
        })
        .collect();

    if inits.is_empty() {
        quote! { #bumps_name }
    } else {
        quote! { #bumps_name { #(#inits,)* } }
    }
}

pub(super) fn emit_seed_methods(
    semantics: &[FieldSemantics],
    bumps_name: &syn::Ident,
) -> proc_macro2::TokenStream {
    let methods: Vec<proc_macro2::TokenStream> = semantics
        .iter()
        .filter_map(|sem| {
            let pda = sem.pda.as_ref()?;
            let seeds = seeds_to_emit_nodes(&pda.source, semantics);
            if seeds.iter().any(|s| matches!(s, SeedEmitNode::InstructionArg { .. })) {
                return None;
            }

            let field_ident = &sem.core.ident;
            let method_name = format_ident!("{}_seeds", field_ident);
            let bump_arr_field = format_ident!("__{}_bump", field_ident);

            let mut seed_elements: Vec<proc_macro2::TokenStream> = seeds
                .iter()
                .map(|node| {
                    let bytes = render_seed_expr(node, SeedRenderContext::Method);
                    quote! { quasar_lang::cpi::Seed::from(#bytes) }
                })
                .collect();

            seed_elements
                .push(quote! { quasar_lang::cpi::Seed::from(&bumps.#bump_arr_field as &[u8]) });

            let seed_count = seed_elements.len();
            Some(quote! {
                #[inline(always)]
                pub fn #method_name<'a>(&'a self, bumps: &'a #bumps_name) -> [quasar_lang::cpi::Seed<'a>; #seed_count] {
                    [#(#seed_elements),*]
                }
            })
        })
        .collect();

    quote! { #(#methods)* }
}
