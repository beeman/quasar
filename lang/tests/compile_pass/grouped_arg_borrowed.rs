#![allow(unexpected_cfgs)]
use quasar_lang::prelude::*;

solana_address::declare_id!("11111111111111111111111111111112");

/// Borrowed instruction arg struct with reference fields.
/// Derives QuasarSerialize → InstructionArgDecode<'a>.
/// Wire format: fixed header (u64) then variable-length str + slice.
#[derive(QuasarSerialize)]
pub struct MintArgs<'a> {
    pub amount: u64,
    #[max(32)]
    pub name: &'a str,
    #[max(8)]
    pub recipients: &'a [Address],
}

#[derive(Accounts)]
pub struct Mint {
    pub authority: Signer,
}

#[program]
mod test_program {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn mint(
        _ctx: Ctx<Mint>,
        _args: MintArgs<'_>,
    ) -> Result<(), ProgramError> {
        Ok(())
    }
}

fn main() {}
