use {
    super::super::{
        seeds::{render_seed_expr, seeds_to_emit_nodes, SeedEmitNode, SeedRenderContext},
        semantics::{BumpSyntax, FieldSemantics, PdaConstraint},
    },
    quote::{format_ident, quote},
};

pub(super) struct SeedBindingParts {
    pub seed_idents: Vec<syn::Ident>,
    pub seed_lets: Vec<proc_macro2::TokenStream>,
}

#[derive(Clone, Copy)]
pub(super) enum PdaBareMode {
    KnownAddress,
    DeriveExpected,
}

pub(super) fn emit_seed_bindings(
    field: &syn::Ident,
    pda: &PdaConstraint,
    all_semantics: &[FieldSemantics],
    ctx: SeedRenderContext,
    name_prefix: &str,
) -> SeedBindingParts {
    let seeds = seeds_to_emit_nodes(&pda.source, all_semantics);
    emit_seed_bindings_from_nodes(field, &seeds, ctx, name_prefix)
}

pub(super) fn emit_seed_bindings_from_nodes(
    field: &syn::Ident,
    seeds: &[SeedEmitNode],
    ctx: SeedRenderContext,
    name_prefix: &str,
) -> SeedBindingParts {
    let seed_idents: Vec<syn::Ident> = seeds
        .iter()
        .enumerate()
        .map(|(i, _)| format_ident!("__{}_{}_{}", name_prefix, field, i))
        .collect();

    let seed_lets: Vec<proc_macro2::TokenStream> = seed_idents
        .iter()
        .zip(seeds.iter())
        .map(|(ident, node)| {
            let expr = render_seed_expr(node, ctx);
            quote! { let #ident: &[u8] = #expr; }
        })
        .collect();

    SeedBindingParts {
        seed_idents,
        seed_lets,
    }
}

pub(super) fn emit_pda_bump_assignment(
    field: &syn::Ident,
    pda: &PdaConstraint,
    seed_idents: &[syn::Ident],
    bump_var: &syn::Ident,
    addr_expr: &proc_macro2::TokenStream,
    seed_array_name: &syn::Ident,
    explicit_bump_name: &syn::Ident,
    bare_mode: PdaBareMode,
    log_failure: bool,
) -> proc_macro2::TokenStream {
    match &pda.bump {
        Some(BumpSyntax::Explicit(expr)) => {
            let failure = emit_failure_log(field, log_failure);
            quote! {
                let #explicit_bump_name: u8 = #expr;
                let __bump_ref: &[u8] = &[#explicit_bump_name];
                let #seed_array_name = [#(#seed_idents,)* __bump_ref];
                quasar_lang::pda::verify_program_address(&#seed_array_name, __program_id, #addr_expr)
                    .map_err(|__e| {
                        #failure
                        __e
                    })?;
                #bump_var = #explicit_bump_name;
            }
        }
        Some(BumpSyntax::Bare) | None => {
            let invalid_pda_error = emit_invalid_pda_error_expr(field, log_failure);
            match bare_mode {
                PdaBareMode::KnownAddress => quote! {
                    let #seed_array_name = [#(#seed_idents),*];
                    #bump_var = quasar_lang::pda::find_bump_for_address(
                        &#seed_array_name,
                        __program_id,
                        #addr_expr,
                    ).map_err(|_| { #invalid_pda_error })?;
                },
                PdaBareMode::DeriveExpected => quote! {
                    let #seed_array_name = [#(#seed_idents),*];
                    let (__expected, __derived_bump) =
                        quasar_lang::pda::based_try_find_program_address(&#seed_array_name, __program_id)?;
                    if !quasar_lang::keys_eq(#addr_expr, &__expected) {
                        return Err(#invalid_pda_error);
                    }
                    #bump_var = __derived_bump;
                },
            }
        }
    }
}

fn emit_failure_log(field: &syn::Ident, enabled: bool) -> proc_macro2::TokenStream {
    if enabled {
        quote! {
            #[cfg(feature = "debug")]
            quasar_lang::prelude::log(concat!(
                "Account '", stringify!(#field),
                "': PDA verification failed"
            ));
        }
    } else {
        quote! {}
    }
}

fn emit_invalid_pda_error_expr(field: &syn::Ident, log_failure: bool) -> proc_macro2::TokenStream {
    let log = emit_failure_log(field, log_failure);
    quote! {
        #log
        quasar_lang::prelude::ProgramError::from(QuasarError::InvalidPda)
    }
}
