//! Emit sweep + close directly from `FieldSemantics`.

use {
    super::super::semantics::{FieldSemantics, FieldShape, LifecycleConstraint},
    quote::quote,
};

pub(super) fn emit_lifecycle_steps(
    semantics: &[FieldSemantics],
) -> syn::Result<proc_macro2::TokenStream> {
    let mut sweep_stmts = Vec::new();
    let mut close_stmts = Vec::new();

    for sem in semantics {
        let field = &sem.core.ident;
        for lifecycle in &sem.lifecycle {
            match lifecycle {
                LifecycleConstraint::Sweep { receiver } => {
                    let authority =
                        super::token::token_authority(sem).cloned().ok_or_else(|| {
                            syn::Error::new(field.span(), "sweep requires token::authority")
                        })?;
                    let mint = super::token::token_mint(sem).cloned().ok_or_else(|| {
                        syn::Error::new(field.span(), "sweep requires token::mint")
                    })?;
                    let token_program = super::token::token_program(sem).ok_or_else(|| {
                        syn::Error::new(field.span(), "sweep requires a token program field")
                    })?;
                    sweep_stmts.push(quote! {
                        quasar_spl::sweep_token_account(
                            self.#token_program.to_account_view(),
                            self.#field.to_account_view(),
                            self.#mint.to_account_view(),
                            self.#receiver.to_account_view(),
                            self.#authority.to_account_view(),
                        )?;
                    });
                }
                LifecycleConstraint::Close { destination } => {
                    if let (Some(authority), Some(token_program)) = (
                        super::token::token_authority(sem).cloned(),
                        super::token::token_program(sem),
                    ) {
                        close_stmts.push(quote! {
                            quasar_spl::close_token_account(
                                self.#token_program.to_account_view(),
                                self.#field.to_account_view(),
                                self.#destination.to_account_view(),
                                self.#authority.to_account_view(),
                            )?;
                        });
                    } else {
                        match &sem.core.shape {
                            FieldShape::Account { .. }
                            | FieldShape::InterfaceAccount { .. }
                            | FieldShape::SystemAccount
                            | FieldShape::Other => {
                                close_stmts.push(quote! {
                                    self.#field.close(self.#destination.to_account_view())?;
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    if sweep_stmts.is_empty() && close_stmts.is_empty() {
        return Ok(quote! {});
    }

    Ok(quote! {
        #[inline(always)]
        fn epilogue(&mut self) -> Result<(), ProgramError> {
            #(#sweep_stmts)*
            #(#close_stmts)*
            Ok(())
        }
    })
}
