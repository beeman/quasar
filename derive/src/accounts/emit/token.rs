use {super::super::semantics::FieldSemantics, quote::quote};

pub(super) fn emit_non_init_check(sem: &FieldSemantics) -> Option<proc_macro2::TokenStream> {
    let field = &sem.core.ident;

    if let Some(tc) = &sem.token {
        let ty = &sem.core.effective_ty;
        let mint = &tc.mint;
        let auth = &tc.authority;
        let token_program = token_program_expr(sem);
        return Some(quote! {
            {
                let mut __params = <#ty as quasar_lang::account_load::AccountLoad>::Params::default();
                __params.mint = Some(*#mint.to_account_view().address());
                __params.authority = Some(*#auth.to_account_view().address());
                __params.token_program = Some(*#token_program);
                quasar_lang::account_load::AccountLoad::validate(#field, &__params)?;
            }
        });
    }

    if let Some(ac) = &sem.ata {
        let wallet = &ac.authority;
        let mint = &ac.mint;
        let token_program = token_program_expr(sem);
        return Some(quote! {
            quasar_spl::validate_ata(
                #field.to_account_view(),
                #wallet.to_account_view().address(),
                #mint.to_account_view().address(),
                #token_program,
            )?;
        });
    }

    sem.mint.as_ref().map(|mc| {
        let ty = &sem.core.effective_ty;
        let decimals = &mc.decimals;
        let auth = &mc.authority;
        let freeze_expr = mint_freeze_load_expr(&mc.freeze_authority);
        let token_program = token_program_expr(sem);
        quote! {
            {
                let mut __params = <#ty as quasar_lang::account_load::AccountLoad>::Params::default();
                __params.authority = Some(*#auth.to_account_view().address());
                __params.decimals = Some((#decimals) as u8);
                __params.freeze_authority = #freeze_expr;
                __params.token_program = Some(*#token_program);
                quasar_lang::account_load::AccountLoad::validate(#field, &__params)?;
            }
        }
    })
}

pub(super) fn emit_init(
    sem: &FieldSemantics,
    guard: bool,
    payer: &syn::Ident,
    signers_setup: &proc_macro2::TokenStream,
    signers_ref: &proc_macro2::TokenStream,
) -> syn::Result<Option<proc_macro2::TokenStream>> {
    let field = &sem.core.ident;

    if let Some(ac) = &sem.ata {
        let authority = &ac.authority;
        let mint = &ac.mint;
        let ata_program = super::init::require_ident(
            sem.support.associated_token_program.as_ref().cloned(),
            field,
            "#[account(init, associated_token::...)] requires an AssociatedTokenProgram field",
        )?;
        let token_program = super::init::require_ident(
            sem.support.token_program.as_ref().cloned(),
            field,
            "ATA init requires a token program field",
        )?;
        let system_program = super::init::require_ident(
            sem.support.system_program.as_ref().cloned(),
            field,
            "ATA init requires a System program field",
        )?;

        let cpi_body = quote! {
            quasar_spl::init_ata(
                #ata_program, #payer, #field, #authority, #mint,
                #system_program, #token_program, #guard,
            )?;
        };
        let validate = quote! {
            quasar_spl::validate_ata(
                #field.to_account_view(),
                #authority.to_account_view().address(),
                #mint.to_account_view().address(),
                #token_program.address(),
            )?;
        };
        return Ok(Some(super::init::wrap_init_guard(
            field,
            guard,
            cpi_body,
            Some(validate),
        )));
    }

    if let Some(tc) = &sem.token {
        let mint = &tc.mint;
        let authority = &tc.authority;
        let token_program = super::init::require_ident(
            sem.support.token_program.as_ref().cloned(),
            field,
            "Token init requires a token program field",
        )?;
        let cpi_body = quote! {
            #signers_setup
            quasar_spl::init_token_account(
                #payer, #field, #token_program, #mint,
                #authority.address(), #signers_ref, &__shared_rent,
            )?;
        };
        let validate = quote! {
            quasar_spl::validate_token_account(
                #field.to_account_view(),
                #mint.to_account_view().address(),
                #authority.to_account_view().address(),
                #token_program.address(),
            )?;
        };
        return Ok(Some(super::init::wrap_init_guard(
            field,
            guard,
            cpi_body,
            Some(validate),
        )));
    }

    let Some(mc) = &sem.mint else {
        return Ok(None);
    };

    let decimals = &mc.decimals;
    let authority = &mc.authority;
    let token_program = super::init::require_ident(
        sem.support.token_program.as_ref().cloned(),
        field,
        "Mint init requires a token program field",
    )?;
    let freeze_init = mint_freeze_address_expr(&mc.freeze_authority);
    let freeze_validate = mint_freeze_validate_expr(&mc.freeze_authority);
    let cpi_body = quote! {
        #signers_setup
        quasar_spl::init_mint_account(
            #payer, #field, #token_program,
            (#decimals) as u8, #authority.address(), #freeze_init,
            #signers_ref, &__shared_rent,
        )?;
    };
    let validate = quote! {
        quasar_spl::validate_mint(
            #field.to_account_view(),
            #authority.to_account_view().address(),
            (#decimals) as u8,
            #freeze_validate,
            #token_program.address(),
        )?;
    };
    Ok(Some(super::init::wrap_init_guard(
        field,
        guard,
        cpi_body,
        Some(validate),
    )))
}

pub(super) fn token_authority(sem: &FieldSemantics) -> Option<&syn::Ident> {
    sem.token
        .as_ref()
        .map(|tc| &tc.authority)
        .or_else(|| sem.ata.as_ref().map(|ac| &ac.authority))
}

pub(super) fn token_mint(sem: &FieldSemantics) -> Option<&syn::Ident> {
    sem.token
        .as_ref()
        .map(|tc| &tc.mint)
        .or_else(|| sem.ata.as_ref().map(|ac| &ac.mint))
}

pub(super) fn token_program(sem: &FieldSemantics) -> Option<&syn::Ident> {
    sem.support.token_program.as_ref()
}

fn token_program_expr(sem: &FieldSemantics) -> syn::Expr {
    match token_program(sem) {
        Some(token_program) => syn::parse_quote!(#token_program.to_account_view().address()),
        None => syn::parse_quote!(&quasar_spl::SPL_TOKEN_ID),
    }
}

fn mint_freeze_address_expr(freeze_authority: &Option<syn::Ident>) -> proc_macro2::TokenStream {
    match freeze_authority {
        Some(freeze_authority) => quote! { Some(#freeze_authority.address()) },
        None => quote! { None },
    }
}

fn mint_freeze_load_expr(freeze_authority: &Option<syn::Ident>) -> proc_macro2::TokenStream {
    match freeze_authority {
        Some(freeze_authority) => quote! { Some(*#freeze_authority.to_account_view().address()) },
        None => quote! { None },
    }
}

fn mint_freeze_validate_expr(freeze_authority: &Option<syn::Ident>) -> proc_macro2::TokenStream {
    match freeze_authority {
        Some(freeze_authority) => quote! { Some(#freeze_authority.to_account_view().address()) },
        None => quote! { None },
    }
}
