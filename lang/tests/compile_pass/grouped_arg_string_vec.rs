#![allow(unexpected_cfgs)]
use quasar_lang::prelude::*;

solana_address::declare_id!("11111111111111111111111111111112");

/// Grouped instruction arg struct with PodString and PodVec fields.
/// Derives QuasarSerialize → InstructionArg: Zc uses PodString/PodVec directly
/// (they are their own Zc). Wire format: fixed PFX+N bytes per field.
#[derive(Copy, Clone, QuasarSerialize)]
pub struct MintArgs {
    pub amount: u64,
    pub name: PodString<32>,
    pub recipients: PodVec<Address, 8>,
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
        _args: MintArgs,
    ) -> Result<(), ProgramError> {
        Ok(())
    }
}

fn main() {}
