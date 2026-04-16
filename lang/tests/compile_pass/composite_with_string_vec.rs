#![allow(unexpected_cfgs)]
use quasar_lang::prelude::*;

solana_address::declare_id!("11111111111111111111111111111112");

/// Composite struct containing PodString and PodVec fields.
/// QuasarSerialize generates ZC companion with PodString/PodVec (Zc = Self).
#[derive(Copy, Clone, QuasarSerialize)]
pub struct Metadata {
    pub label: PodString<16>,
    pub values: PodVec<u8, 4>,
    pub version: u32,
}

/// Account with composite field containing aliased String/Vec.
/// Tests the full chain: alias rewriting → ZC mapping → set_inner codegen.
#[account(discriminator = 1, set_inner)]
pub struct Registry {
    pub meta: Metadata,
    pub bump: u8,
}

/// Instruction arg with composite containing PodString/PodVec.
#[derive(Copy, Clone, QuasarSerialize)]
pub struct UpdateArgs {
    pub meta: Metadata,
    pub flag: bool,
}

#[derive(Accounts)]
pub struct Update {
    pub authority: Signer,
}

#[program]
mod test_program {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn update(
        _ctx: Ctx<Update>,
        _args: UpdateArgs,
    ) -> Result<(), ProgramError> {
        Ok(())
    }
}

fn main() {}
