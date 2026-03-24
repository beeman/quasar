//! `#[derive(QuasarSerialize)]` — generates two things for instruction data
//! structs:
//!
//! 1. An `InstructionArg` impl with an alignment-1 ZC companion struct,
//!    enabling zero-copy deserialization inside `#[instruction]` handlers.
//!
//! 2. A `WriteBytes` impl (gated behind `cfg(not(solana))`) for off-chain
//!    instruction data serialization.

use {
    proc_macro::TokenStream,
    quote::{format_ident, quote},
    syn::{parse_macro_input, Data, DeriveInput, Fields},
};

pub(crate) fn derive_write_bytes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    name,
                    "QuasarSerialize can only be derived for structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(
                name,
                "QuasarSerialize can only be derived for structs",
            )
            .to_compile_error()
            .into();
        }
    };

    let zc_name = format_ident!("__{}Zc", name);

    let field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();

    // ZC companion fields: <FieldType as InstructionArg>::Zc
    let zc_field_types: Vec<_> = field_types
        .iter()
        .map(|ty| {
            quote! { <#ty as quasar_lang::instruction_arg::InstructionArg>::Zc }
        })
        .collect();

    // from_zc field reconstructions
    let from_zc_fields: Vec<_> = field_names
        .iter()
        .zip(field_types.iter())
        .map(|(name, ty)| {
            quote! {
                #name: <#ty as quasar_lang::instruction_arg::InstructionArg>::from_zc(&zc.#name)
            }
        })
        .collect();

    // WriteBytes field writes (off-chain only)
    let field_writes: Vec<_> = field_names
        .iter()
        .map(|field_name| {
            quote! {
                quasar_lang::client::WriteBytes::write_bytes(&self.#field_name, buf);
            }
        })
        .collect();

    let expanded = quote! {
        // Alignment-1 ZC companion for zero-copy instruction deserialization.
        #[doc(hidden)]
        #[repr(C)]
        #[derive(Copy, Clone)]
        pub struct #zc_name {
            #(#field_names: #zc_field_types,)*
        }

        impl #impl_generics quasar_lang::instruction_arg::InstructionArg
            for #name #ty_generics #where_clause
        {
            type Zc = #zc_name;
            #[inline(always)]
            fn from_zc(zc: &#zc_name) -> Self {
                Self {
                    #(#from_zc_fields,)*
                }
            }
        }

        // WriteBytes impl (off-chain only)
        #[cfg(not(any(target_os = "solana", target_arch = "bpf")))]
        impl #impl_generics quasar_lang::client::WriteBytes for #name #ty_generics #where_clause {
            #[inline(always)]
            fn write_bytes(&self, buf: &mut ::alloc::vec::Vec<u8>) {
                #(#field_writes)*
            }
        }
    };

    expanded.into()
}
