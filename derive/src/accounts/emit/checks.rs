//! Emit validation checks directly from `FieldSemantics`.

use {
    super::super::semantics::{FieldSemantics, PdaConstraint, UserCheckKind},
    quote::{format_ident, quote},
};

pub(super) fn emit_check_blocks(semantics: &[FieldSemantics]) -> Vec<proc_macro2::TokenStream> {
    semantics
        .iter()
        .map(|sem| emit_one_check_block(sem, semantics))
        .filter(|ts| !ts.is_empty())
        .collect()
}

fn emit_one_check_block(
    sem: &FieldSemantics,
    all_semantics: &[FieldSemantics],
) -> proc_macro2::TokenStream {
    let field_ident = &sem.core.ident;
    let mut stmts = Vec::new();

    for uc in &sem.user_checks {
        match &uc.kind {
            UserCheckKind::HasOne { target } => {
                let err = match &uc.error {
                    Some(e) => quote! { #e.into() },
                    None => quote! { QuasarError::HasOneMismatch.into() },
                };
                let field_name_str = field_ident.to_string();
                let target_str = target.to_string();
                stmts.push(quote! {
                    #[cfg(feature = "debug")]
                    if !quasar_lang::keys_eq(&#field_ident.#target, #target.to_account_view().address()) {
                        quasar_lang::prelude::log(concat!(
                            "has_one mismatch: ", #field_name_str, ".", #target_str,
                            " != ", #target_str, ".address()"
                        ));
                    }
                    quasar_lang::validation::check_address_match(
                        &#field_ident.#target,
                        #target.to_account_view().address(),
                        #err,
                    )?;
                });
            }
            UserCheckKind::Constraint { expr } => {
                let err = match &uc.error {
                    Some(e) => quote! { #e.into() },
                    None => quote! { QuasarError::ConstraintViolation.into() },
                };
                stmts.push(quote! {
                    quasar_lang::validation::check_constraint(#expr, #err)?;
                });
            }
            UserCheckKind::Address { expr } => {
                let err = match &uc.error {
                    Some(e) => quote! { #e.into() },
                    None => quote! { QuasarError::AddressMismatch.into() },
                };
                stmts.push(quote! {
                    quasar_lang::validation::check_address_match(
                        #field_ident.to_account_view().address(),
                        &#expr,
                        #err,
                    )?;
                });
            }
        }
    }

    if !sem.has_init() {
        if let Some(pda) = &sem.pda {
            stmts.push(emit_pda_check(field_ident, pda, all_semantics));
        }
        if let Some(token_check) = super::token::emit_non_init_check(sem) {
            stmts.push(token_check);
        }
    }

    if stmts.is_empty() {
        quote! {}
    } else if sem.core.optional {
        quote! {
            if let Some(ref #field_ident) = #field_ident {
                #(#stmts)*
            }
        }
    } else {
        quote! { #(#stmts)* }
    }
}

fn emit_pda_check(
    field: &syn::Ident,
    pda: &PdaConstraint,
    all_semantics: &[FieldSemantics],
) -> proc_macro2::TokenStream {
    let bump_var = format_ident!("__bumps_{}", field);
    let bindings = super::pda::emit_seed_bindings(
        field,
        pda,
        all_semantics,
        super::super::seeds::SeedRenderContext::Parse,
        "seed",
    );
    let seed_lets = bindings.seed_lets;
    let seed_array_name = format_ident!("__pda_seeds_{}", field);
    let explicit_bump_name = format_ident!("__bump_val_{}", field);
    let addr_access = quote! { #field.to_account_view().address() };
    let bump_assign = super::pda::emit_pda_bump_assignment(
        field,
        pda,
        &bindings.seed_idents,
        &bump_var,
        &addr_access,
        &seed_array_name,
        &explicit_bump_name,
        super::pda::PdaBareMode::KnownAddress,
        true,
    );

    quote! {
        {
            #(#seed_lets)*
            #bump_assign
        }
    }
}
