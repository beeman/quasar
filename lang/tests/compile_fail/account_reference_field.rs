#![allow(unexpected_cfgs)]
use quasar_lang::prelude::*;

#[derive(Accounts)]
pub struct BadAccountField<'account> {
    #[account(mut)]
    pub signer: &'account mut Signer,
}

fn main() {}
