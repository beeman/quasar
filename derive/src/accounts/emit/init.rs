use {
    super::super::{
        seeds::SeedRenderContext,
        semantics::{FieldSemantics, FieldShape, InitMode},
    },
    quote::{format_ident, quote},
};

pub(super) fn require_ident(
    ident: Option<syn::Ident>,
    field: &syn::Ident,
    message: &str,
) -> syn::Result<syn::Ident> {
    ident.ok_or_else(|| syn::Error::new(field.span(), message))
}

pub(super) fn emit_init_stmts(
    semantics: &[FieldSemantics],
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut stmts = Vec::new();

    for sem in semantics {
        if sem.init.is_some() {
            stmts.push(emit_one_init(sem, semantics)?);
        }
    }

    Ok(stmts)
}

fn emit_one_init(
    sem: &FieldSemantics,
    all_semantics: &[FieldSemantics],
) -> syn::Result<proc_macro2::TokenStream> {
    let field = &sem.core.ident;
    let init = sem.init.as_ref().expect("checked by caller");
    let guard = matches!(init.mode, InitMode::InitIfNeeded);
    let payer = require_ident(
        sem.support.payer.clone(),
        field,
        "init requires a payer field",
    )?;

    let (signers_setup, signers_ref) = emit_signers(field, sem.pda.as_ref(), all_semantics);

    if let Some(token_init) =
        super::token::emit_init(sem, guard, &payer, &signers_setup, &signers_ref)?
    {
        return Ok(token_init);
    }

    let inner_ty = match &sem.core.shape {
        FieldShape::Account { inner_ty } | FieldShape::InterfaceAccount { inner_ty } => inner_ty,
        _ => &sem.core.effective_ty,
    };
    let inner_base = crate::helpers::strip_generics(inner_ty);
    let space_expr = if let Some(space) = &init.space {
        quote! { (#space) as u64 }
    } else {
        quote! { <#inner_base as quasar_lang::traits::Space>::SPACE as u64 }
    };
    let cpi_body = quote! {
        #signers_setup
        quasar_lang::account_init::init_account(
            #payer, #field, #space_expr,
            __program_id, #signers_ref, &__shared_rent,
            <#inner_base as quasar_lang::traits::Discriminator>::DISCRIMINATOR,
        )?;
    };
    let validate = if guard {
        Some(quote! {
            <#inner_base as quasar_lang::traits::CheckOwner>::check_owner(#field.to_account_view())?;
            <#inner_base as quasar_lang::traits::AccountCheck>::check(#field.to_account_view())?;
        })
    } else {
        None
    };
    Ok(wrap_init_guard(field, guard, cpi_body, validate))
}

fn emit_signers(
    field: &syn::Ident,
    pda: Option<&super::super::semantics::PdaConstraint>,
    all_semantics: &[FieldSemantics],
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let Some(pda) = pda else {
        return (quote! {}, quote! { &[] });
    };

    let bump_var = format_ident!("__bumps_{}", field);
    let bindings = super::pda::emit_seed_bindings(
        field,
        pda,
        all_semantics,
        SeedRenderContext::Init,
        "init_seed",
    );
    let seed_lets = bindings.seed_lets;
    let seed_idents = bindings.seed_idents;
    let seed_array_name = format_ident!("__init_seed_refs_{}", field);
    let explicit_bump_name = format_ident!("__init_bump_{}", field);
    let pda_assign = super::pda::emit_pda_bump_assignment(
        field,
        pda,
        &seed_idents,
        &bump_var,
        &quote! { #field.address() },
        &seed_array_name,
        &explicit_bump_name,
        super::pda::PdaBareMode::DeriveExpected,
        false,
    );

    (
        quote! {
            #(#seed_lets)*
            #pda_assign
            let __init_bump_ref: &[u8] = &[#bump_var];
            let __init_signer_seeds = [#(quasar_lang::cpi::Seed::from(#seed_idents),)* quasar_lang::cpi::Seed::from(__init_bump_ref)];
            let __init_signers = [quasar_lang::cpi::Signer::from(&__init_signer_seeds[..])];
        },
        quote! { &__init_signers },
    )
}

pub(super) fn wrap_init_guard(
    field: &syn::Ident,
    idempotent: bool,
    cpi_body: proc_macro2::TokenStream,
    validate_existing: Option<proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    if idempotent {
        let validate = validate_existing.unwrap_or_default();
        quote! {
            {
                if quasar_lang::is_system_program(#field.owner()) {
                    #cpi_body
                } else {
                    #validate
                }
            }
        }
    } else {
        quote! {
            {
                if !quasar_lang::is_system_program(#field.owner()) {
                    return Err(ProgramError::AccountAlreadyInitialized);
                }
                #cpi_body
            }
        }
    }
}
