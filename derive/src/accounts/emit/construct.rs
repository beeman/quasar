//! Emit field construction directly from `FieldSemantics`.

use {
    super::super::semantics::{FieldSemantics, FieldShape},
    crate::helpers::strip_generics,
    quote::quote,
};

pub(super) fn field_idents(semantics: &[FieldSemantics]) -> Vec<&syn::Ident> {
    semantics.iter().map(|sem| &sem.core.ident).collect()
}

pub(super) fn emit_construct_steps(semantics: &[FieldSemantics]) -> Vec<proc_macro2::TokenStream> {
    semantics.iter().map(emit_one).collect()
}

fn emit_one(sem: &FieldSemantics) -> proc_macro2::TokenStream {
    let ident = &sem.core.ident;
    let expr = emit_inner_expr(sem);
    if sem.core.optional {
        quote! {
            #ident: if quasar_lang::keys_eq(#ident.address(), __program_id) { None } else { Some(#expr) }
        }
    } else {
        quote! { #ident: #expr }
    }
}

fn emit_inner_expr(sem: &FieldSemantics) -> proc_macro2::TokenStream {
    let ident = &sem.core.ident;
    let ty = &sem.core.effective_ty;
    let skip_checks =
        sem.has_init() && (sem.token.is_some() || sem.ata.is_some() || sem.mint.is_some());

    if matches!(sem.core.shape, FieldShape::Composite) {
        quote! { #ident }
    } else if sem.core.dynamic {
        let inner_ty = match &sem.core.shape {
            FieldShape::Account { inner_ty } => inner_ty,
            _ => &sem.core.effective_ty,
        };
        let base = strip_generics(inner_ty);
        quote! { #base::from_account_view(#ident)? }
    } else if skip_checks {
        quote! {
            unsafe {
                core::ptr::read(
                    <#ty as quasar_lang::account_load::AccountLoad>::from_view_unchecked(#ident)
                )
            }
        }
    } else {
        let field_name_str = ident.to_string();
        quote! {
            <#ty as quasar_lang::account_load::AccountLoad>::load(#ident, #field_name_str)?
        }
    }
}
