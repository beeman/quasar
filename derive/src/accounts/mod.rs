//! `#[derive(Accounts)]` — generates account parsing, validation, and PDA
//! derivation from a struct definition. This is the core macro that transforms
//! a declarative accounts struct into the zero-copy parsing pipeline.

mod attrs;
mod descriptors;
pub(crate) mod emit;
mod instruction_args;
mod parse;
pub(crate) mod seeds;
pub(crate) mod semantics;

pub(crate) use instruction_args::InstructionArg;
use {
    instruction_args::{generate_instruction_arg_extraction, parse_struct_instruction_args},
    parse::build_parse_parts,
    proc_macro::TokenStream,
    quote::{format_ident, quote},
    syn::{parse_macro_input, parse_quote, Data, DeriveInput, Fields, GenericParam},
};

pub(crate) fn derive_accounts(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let bumps_name = format_ident!("{}Bumps", name);

    // Currently only custom lifetime parameters are supported, so validate that
    // we don't have any type or const generics.
    if let Some(param) = input
        .generics
        .params
        .iter()
        .find(|param| !matches!(param, GenericParam::Lifetime(_)))
    {
        let message = match param {
            GenericParam::Type(_) => {
                "#[derive(Accounts)] only supports lifetime parameters; type parameters are not \
                 supported"
            }
            GenericParam::Const(_) => {
                "#[derive(Accounts)] only supports lifetime parameters; const parameters are not \
                 supported"
            }
            // Filtered by the `find` predicate above — lifetimes are skipped.
            GenericParam::Lifetime(_) => "",
        };
        return syn::Error::new_spanned(param, message)
            .to_compile_error()
            .into();
    }
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut parse_generics = input.generics.clone();
    // 'input is the default lifetime used for account references in the generated
    // traits, so we need to make sure that it lives longer than any
    // user-defined lifetimes.
    parse_generics.params.push(parse_quote!('input));
    {
        let parse_where = parse_generics.make_where_clause();
        for lifetime in input.generics.lifetimes() {
            let lifetime = &lifetime.lifetime;
            parse_where
                .predicates
                .push(syn::parse_quote!('input: #lifetime));
        }
    }
    // These generics are used for the ParseAccounts impl, which may have different
    // lifetime requirements than the original struct.
    let (parse_impl_generics, _, parse_where_clause) = parse_generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    name,
                    "Accounts can only be derived for structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(name, "Accounts can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };

    let instruction_args = match parse_struct_instruction_args(&input) {
        Ok(args) => args,
        Err(e) => return e.to_compile_error().into(),
    };

    // --- Run the accounts pipeline (syntax → resolve → emit) ---

    let semantics = match semantics::lower_semantics(fields, &instruction_args) {
        Ok(semantics) => semantics,
        Err(e) => return e.to_compile_error().into(),
    };

    let emit_cx = emit::EmitCx {
        bumps_name: bumps_name.clone(),
    };

    let parse_parts = match build_parse_parts(&semantics, &emit_cx) {
        Ok(parts) => parts,
        Err(e) => return e.to_compile_error().into(),
    };
    let parse::ParseParts {
        parse_steps,
        count_expr,
        typed_seed_asserts,
        parse_body,
    } = parse_parts;
    let bumps_struct = emit::emit_bump_struct_def(&semantics, &emit_cx);
    let epilogue_method = match emit::emit_epilogue(&semantics) {
        Ok(ts) => ts,
        Err(e) => return e.to_compile_error().into(),
    };

    // --- Seeds impl ---

    let seeds_methods = emit::emit_seed_methods(&semantics, &emit_cx);
    let seeds_impl = if seeds_methods.is_empty() {
        quote! {}
    } else {
        quote! {
            impl #impl_generics #name #ty_generics #where_clause {
                #seeds_methods
            }
        }
    };

    // --- Client macro ---

    let descriptors = descriptors::describe_accounts(&semantics);
    let client_macro = quasar_client::generate_accounts_macro(name, &descriptors);

    // --- Instruction arg extraction (struct-level #[instruction(...)]) ---

    let ix_arg_extraction = if let Some(ref ix_args) = instruction_args {
        generate_instruction_arg_extraction(ix_args)
    } else {
        quote! {}
    };

    // --- Final output ---

    let exact_len_guard = quote! {
        quasar_lang::traits::check_account_count(accounts.len(), Self::COUNT)?;
    };

    let parse_accounts_impl = quote! {
        impl #parse_impl_generics ParseAccounts<'input> for #name #ty_generics #parse_where_clause {
            type Bumps = #bumps_name;

            #[inline(always)]
            fn parse(accounts: &'input mut [AccountView], program_id: &Address) -> Result<(Self, Self::Bumps), ProgramError> {
                #exact_len_guard
                unsafe {
                    <Self as quasar_lang::traits::ParseAccountsUnchecked>::parse_with_instruction_data_unchecked(
                        accounts,
                        &[],
                        program_id,
                    )
                }
            }

            #[inline(always)]
            fn parse_with_instruction_data(
                accounts: &'input mut [AccountView],
                __ix_data: &[u8],
                __program_id: &Address,
            ) -> Result<(Self, Self::Bumps), ProgramError> {
                #exact_len_guard
                unsafe {
                    <Self as quasar_lang::traits::ParseAccountsUnchecked>::parse_with_instruction_data_unchecked(
                        accounts,
                        __ix_data,
                        __program_id,
                    )
                }
            }

            #epilogue_method
        }

        unsafe impl #parse_impl_generics quasar_lang::traits::ParseAccountsUnchecked<'input>
            for #name #ty_generics
            #parse_where_clause
        {
            #[inline(always)]
            unsafe fn parse_unchecked(
                accounts: &'input mut [AccountView],
                program_id: &Address,
            ) -> Result<(Self, Self::Bumps), ProgramError> {
                <Self as quasar_lang::traits::ParseAccountsUnchecked>::parse_with_instruction_data_unchecked(
                    accounts,
                    &[],
                    program_id,
                )
            }

            #[inline(always)]
            unsafe fn parse_with_instruction_data_unchecked(
                accounts: &'input mut [AccountView],
                __ix_data: &[u8],
                __program_id: &Address,
            ) -> Result<(Self, Self::Bumps), ProgramError> {
                #typed_seed_asserts
                #ix_arg_extraction
                #parse_body
            }
        }
    };

    let expanded = quote! {
        #bumps_struct

        #parse_accounts_impl

        #seeds_impl

        impl #impl_generics AccountCount for #name #ty_generics #where_clause {
            const COUNT: usize = #count_expr;
        }

        impl #impl_generics #name #ty_generics #where_clause {
            #[inline(always)]
            pub unsafe fn parse_accounts(
                mut input: *mut u8,
                buf: &mut core::mem::MaybeUninit<[quasar_lang::__internal::AccountView; #count_expr]>,
                __program_id: &quasar_lang::prelude::Address,
            ) -> Result<*mut u8, ProgramError> {
                let base = buf.as_mut_ptr() as *mut quasar_lang::__internal::AccountView;

                #(#parse_steps)*

                Ok(input)
            }
        }

        #client_macro
    };

    TokenStream::from(expanded)
}
