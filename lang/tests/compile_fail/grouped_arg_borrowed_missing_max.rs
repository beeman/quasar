use quasar_lang::prelude::*;

solana_address::declare_id!("11111111111111111111111111111112");

// Reference field without #[max(N)] — should produce a clear error.
#[derive(QuasarSerialize)]
pub struct MintArgs<'a> {
    pub amount: u64,
    pub name: &'a str,
}

fn main() {}
