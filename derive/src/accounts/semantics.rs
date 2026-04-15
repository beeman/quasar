mod core;
mod lower;
mod resolve;
mod rules;

pub(crate) use self::core::*;

pub(super) fn lower_semantics(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    instruction_args: &Option<Vec<crate::accounts::InstructionArg>>,
) -> syn::Result<Vec<FieldSemantics>> {
    self::lower::lower_semantics(fields, instruction_args)
}
