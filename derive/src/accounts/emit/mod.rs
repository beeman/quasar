//! Emit layer: consumes resolved `FieldSemantics` directly.

mod bumps;
mod checks;
mod construct;
mod init;
mod lifecycle;
mod pda;
mod realloc;
mod token;

use {super::semantics::FieldSemantics, quote::quote};

pub(crate) struct EmitCx {
    pub bumps_name: syn::Ident,
}

pub(crate) fn emit_parse_body(
    semantics: &[FieldSemantics],
    cx: &EmitCx,
) -> syn::Result<proc_macro2::TokenStream> {
    let rent_fetch = emit_rent_fetch(semantics);
    let init_stmts = init::emit_init_stmts(semantics)?;
    let realloc_stmts = realloc::emit_realloc_steps(semantics)?;
    let construct_stmts = construct::emit_construct_steps(semantics);
    let check_stmts = checks::emit_check_blocks(semantics);
    let bump_vars = bumps::emit_bump_vars(semantics);
    let bump_init = bumps::emit_bump_init(semantics, &cx.bumps_name);
    let field_names = construct::field_idents(semantics);

    if !check_stmts.is_empty() {
        Ok(quote! {
            #bump_vars
            #rent_fetch
            #(#init_stmts)*
            #(#realloc_stmts)*
            let result = Self { #(#construct_stmts,)* };
            {
                let Self { #(ref #field_names,)* } = result;
                #(#check_stmts)*
            }
            Ok((result, #bump_init))
        })
    } else {
        Ok(quote! {
            #bump_vars
            #rent_fetch
            #(#init_stmts)*
            #(#realloc_stmts)*
            Ok((Self { #(#construct_stmts,)* }, #bump_init))
        })
    }
}

pub(crate) fn emit_bump_struct_def(
    semantics: &[FieldSemantics],
    cx: &EmitCx,
) -> proc_macro2::TokenStream {
    bumps::emit_bump_struct(semantics, &cx.bumps_name)
}

pub(crate) fn emit_epilogue(semantics: &[FieldSemantics]) -> syn::Result<proc_macro2::TokenStream> {
    lifecycle::emit_lifecycle_steps(semantics)
}

pub(crate) fn emit_seed_methods(
    semantics: &[FieldSemantics],
    cx: &EmitCx,
) -> proc_macro2::TokenStream {
    bumps::emit_seed_methods(semantics, &cx.bumps_name)
}

fn emit_rent_fetch(semantics: &[FieldSemantics]) -> proc_macro2::TokenStream {
    if !semantics.iter().any(FieldSemantics::needs_rent) {
        return quote! {};
    }

    match semantics
        .iter()
        .filter(|sem| sem.needs_rent())
        .find_map(|sem| sem.support.rent_sysvar.as_ref())
    {
        Some(rent_field) => quote! {
            let __shared_rent = unsafe {
                core::clone::Clone::clone(
                    <quasar_lang::sysvars::rent::Rent as quasar_lang::sysvars::Sysvar>::from_bytes_unchecked(
                        #rent_field.borrow_unchecked()
                    )
                )
            };
        },
        None => quote! {
            let __shared_rent = <quasar_lang::sysvars::rent::Rent as quasar_lang::sysvars::Sysvar>::get()?;
        },
    }
}
