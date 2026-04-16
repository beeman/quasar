#![allow(unexpected_cfgs)]
use quasar_lang::prelude::*;

solana_address::declare_id!("11111111111111111111111111111112");

/// Instruction arg struct using PodString and PodVec.
/// String<N>/Vec<T,N> aliases are only valid inside #[account] fields.
#[derive(Copy, Clone, QuasarSerialize)]
pub struct CreateArgs {
    pub amount: u64,
    pub name: PodString<32>,
    pub tags: PodVec<u8, 8>,
}

#[derive(Accounts)]
pub struct Create {
    pub authority: Signer,
}

#[program]
mod test_program {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn create(
        _ctx: Ctx<Create>,
        _args: CreateArgs,
    ) -> Result<(), ProgramError> {
        Ok(())
    }
}

fn main() {}
