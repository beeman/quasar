//! Emit realloc calls directly from `FieldSemantics`.

use {super::super::semantics::FieldSemantics, quote::quote};

pub(super) fn emit_realloc_steps(
    semantics: &[FieldSemantics],
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    semantics
        .iter()
        .filter_map(|sem| sem.realloc.as_ref().map(|rc| (sem, rc)))
        .map(|(sem, rc)| emit_one_realloc(sem, rc))
        .collect()
}

fn emit_one_realloc(
    sem: &FieldSemantics,
    rc: &super::super::semantics::ReallocConstraint,
) -> syn::Result<proc_macro2::TokenStream> {
    let field = &sem.core.ident;
    let space = &rc.space_expr;
    let payer = sem
        .support
        .realloc_payer
        .clone()
        .ok_or_else(|| syn::Error::new(field.span(), "realloc requires a payer field"))?;

    Ok(quote! {
        {
            let __realloc_space = (#space) as usize;
            quasar_lang::accounts::realloc_account(
                #field, __realloc_space, #payer, Some(&__shared_rent)
            )?;
        }
    })
}
