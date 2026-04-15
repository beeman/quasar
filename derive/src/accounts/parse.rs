use {
    super::{emit, semantics},
    crate::helpers::strip_generics,
    quote::{format_ident, quote},
};

pub(crate) struct ParseParts {
    pub parse_steps: Vec<proc_macro2::TokenStream>,
    pub count_expr: proc_macro2::TokenStream,
    pub typed_seed_asserts: proc_macro2::TokenStream,
    pub parse_body: proc_macro2::TokenStream,
}

pub(crate) fn build_parse_parts(
    semantics: &[semantics::FieldSemantics],
    cx: &emit::EmitCx,
) -> syn::Result<ParseParts> {
    Ok(ParseParts {
        parse_steps: emit_parse_account_steps(semantics),
        count_expr: emit_count_expr(semantics),
        typed_seed_asserts: emit_typed_seed_asserts(semantics),
        parse_body: emit_full_parse_body(semantics, cx)?,
    })
}

fn emit_parse_account_steps(
    semantics: &[semantics::FieldSemantics],
) -> Vec<proc_macro2::TokenStream> {
    if !semantics.iter().any(is_composite) {
        return semantics
            .iter()
            .enumerate()
            .map(|(index, sem)| emit_single_parse_step(sem, &quote! { #index }))
            .collect();
    }

    let mut steps = Vec::new();
    let mut buf_offset_expr = quote! { 0usize };

    for sem in semantics {
        if let Some(inner_ty) = composite_inner_ty(sem) {
            let cur_offset = buf_offset_expr.clone();

            steps.push(quote! {
                {
                    let mut __inner_buf = core::mem::MaybeUninit::<
                        [quasar_lang::__internal::AccountView; <#inner_ty as AccountCount>::COUNT]
                    >::uninit();
                    input = <#inner_ty>::parse_accounts(input, &mut __inner_buf, __program_id)?;
                    let __inner = unsafe { __inner_buf.assume_init() };
                    let mut __j = 0usize;
                    while __j < <#inner_ty as AccountCount>::COUNT {
                        unsafe { core::ptr::write(base.add(#cur_offset + __j), *__inner.as_ptr().add(__j)); }
                        __j += 1;
                    }
                }
            });

            buf_offset_expr = quote! { #cur_offset + <#inner_ty as AccountCount>::COUNT };
        } else {
            let cur_offset = buf_offset_expr.clone();
            steps.push(emit_single_parse_step(sem, &cur_offset));
            buf_offset_expr = quote! { #cur_offset + 1usize };
        }
    }

    steps
}

fn emit_single_parse_step(
    sem: &semantics::FieldSemantics,
    cur_offset: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let field_name = &sem.core.ident;
    let ty = &sem.core.effective_ty;
    let account_index = cur_offset.to_string();

    let is_writable = sem.is_writable();
    let writable_bit: u32 = if is_writable { 0x01 << 16 } else { 0 };
    let writable_mask: u32 = if is_writable { 0xFF << 16 } else { 0 };
    let init_forces_signer =
        matches!(sem.core.shape, semantics::FieldShape::Signer) || sem.client_requires_signer();

    let expected_expr = quote! {{
        const __S: bool = <#ty as quasar_lang::account_load::AccountLoad>::IS_SIGNER || #init_forces_signer;
        const __E: bool = <#ty as quasar_lang::account_load::AccountLoad>::IS_EXECUTABLE;
        0xFFu32 | (__S as u32) << 8 | #writable_bit | (__E as u32) << 24
    }};

    let mask_expr = quote! {{
        const __S: bool = <#ty as quasar_lang::account_load::AccountLoad>::IS_SIGNER || #init_forces_signer;
        const __E: bool = <#ty as quasar_lang::account_load::AccountLoad>::IS_EXECUTABLE;
        0xFFu32 | (if __S { 0xFFu32 << 8 } else { 0u32 }) | #writable_mask | (if __E { 0xFFu32 << 24 } else { 0u32 })
    }};

    if sem.core.optional || sem.core.dup {
        let flag_mask_expr = quote! {{
            const __S: bool = <#ty as quasar_lang::account_load::AccountLoad>::IS_SIGNER || #init_forces_signer;
            const __E: bool = <#ty as quasar_lang::account_load::AccountLoad>::IS_EXECUTABLE;
            (if __S { 0xFFu32 << 8 } else { 0u32 }) | #writable_mask | (if __E { 0xFFu32 << 24 } else { 0u32 })
        }};

        let is_optional = sem.core.optional;
        let is_ref_mut = is_writable;
        let allow_dup = sem.core.dup;

        quote! {
            {
                const __EXPECTED: u32 = #expected_expr;
                const __MASK: u32 = #mask_expr;
                const __FLAG_MASK: u32 = #flag_mask_expr;
                input = unsafe {
                    quasar_lang::__internal::parse_account_dup(
                        input,
                        base,
                        #cur_offset,
                        __program_id,
                        quasar_lang::__internal::ParseFlags {
                            expected: __EXPECTED,
                            mask: __MASK,
                            flag_mask: __FLAG_MASK,
                            is_optional: #is_optional,
                            is_ref_mut: #is_ref_mut,
                            allow_dup: #allow_dup,
                        },
                    )?
                };
                #[cfg(feature = "debug")]
                quasar_lang::prelude::log(concat!(
                    "Account '", stringify!(#field_name),
                    "' (index ", #account_index, "): parsed (dup-aware)"
                ));
            }
        }
    } else {
        quote! {
            {
                const __EXPECTED: u32 = #expected_expr;
                const __MASK: u32 = #mask_expr;
                input = unsafe {
                    quasar_lang::__internal::parse_account(
                        input, base, #cur_offset, __EXPECTED, __MASK,
                    )?
                };
                #[cfg(feature = "debug")]
                quasar_lang::prelude::log(concat!(
                    "Account '", stringify!(#field_name),
                    "' (index ", #account_index, "): validation passed"
                ));
            }
        }
    }
}

fn emit_count_expr(semantics: &[semantics::FieldSemantics]) -> proc_macro2::TokenStream {
    if !semantics.iter().any(is_composite) {
        let n = semantics.len();
        quote! { #n }
    } else {
        let addends: Vec<proc_macro2::TokenStream> = semantics
            .iter()
            .map(|sem| {
                composite_inner_ty(sem)
                    .map(|ty| quote! { <#ty as AccountCount>::COUNT })
                    .unwrap_or_else(|| quote! { 1usize })
            })
            .collect();
        quote! { #(#addends)+* }
    }
}

fn emit_full_parse_body(
    semantics: &[semantics::FieldSemantics],
    cx: &emit::EmitCx,
) -> syn::Result<proc_macro2::TokenStream> {
    let inner_body = emit::emit_parse_body(semantics, cx)?;

    if semantics.iter().any(is_composite) {
        let mut field_lets: Vec<proc_macro2::TokenStream> = Vec::new();
        field_lets.push(quote! { let mut __accounts_rest = accounts; });

        for sem in semantics {
            let field_name = &sem.core.ident;

            if let Some(inner_ty) = composite_inner_ty(sem) {
                let bumps_var = format_ident!("__composite_bumps_{}", field_name);
                field_lets.push(quote! {
                    let (__chunk, __rest) = unsafe {
                        __accounts_rest.split_at_mut_unchecked(<#inner_ty as AccountCount>::COUNT)
                    };
                    __accounts_rest = __rest;
                    let (#field_name, #bumps_var) = unsafe { <#inner_ty as quasar_lang::traits::ParseAccountsUnchecked>::parse_unchecked(
                        __chunk,
                        __program_id
                    ) }?;
                });
            } else {
                field_lets.push(quote! {
                    let (__chunk, __rest) = unsafe { __accounts_rest.split_at_mut_unchecked(1) };
                    __accounts_rest = __rest;
                    let #field_name = unsafe { __chunk.get_unchecked_mut(0) };
                });
            }
        }
        field_lets.push(quote! { let _ = __accounts_rest; });

        Ok(quote! {
            #(#field_lets)*
            #inner_body
        })
    } else {
        let names: Vec<&syn::Ident> = semantics.iter().map(|sem| &sem.core.ident).collect();

        Ok(quote! {
            let [#(#names),*] = accounts else {
                unsafe { core::hint::unreachable_unchecked() }
            };
            #inner_body
        })
    }
}

fn emit_typed_seed_asserts(semantics: &[semantics::FieldSemantics]) -> proc_macro2::TokenStream {
    let asserts: Vec<proc_macro2::TokenStream> = semantics
        .iter()
        .filter_map(|sem| match &sem.pda {
            Some(semantics::PdaConstraint {
                source: semantics::PdaSource::Typed { type_path, args },
                ..
            }) => {
                let arg_count = args.len();
                Some(quote! {
                    let _: [(); <#type_path as quasar_lang::traits::HasSeeds>::SEED_DYNAMIC_COUNT] = [(); #arg_count];
                })
            }
            _ => None,
        })
        .collect();

    quote! { #(#asserts)* }
}

fn is_composite(sem: &semantics::FieldSemantics) -> bool {
    matches!(sem.core.shape, semantics::FieldShape::Composite)
}

fn composite_inner_ty(sem: &semantics::FieldSemantics) -> Option<proc_macro2::TokenStream> {
    is_composite(sem).then(|| strip_generics(&sem.core.effective_ty))
}
