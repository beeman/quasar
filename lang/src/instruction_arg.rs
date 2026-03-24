//! Trait for types that can be used as fixed-size instruction arguments.
//!
//! Each type provides an alignment-1 zero-copy companion (`Zc`) and a
//! conversion function (`from_zc`) used by `#[instruction]` codegen.
//! Primitive integers map to their Pod equivalents (e.g. `u64` → `PodU64`),
//! while custom structs derive their companion via
//! `#[derive(QuasarSerialize)]`.

use crate::pod::*;

/// A type that can appear as a fixed-size `#[instruction]` argument.
///
/// The associated `Zc` type must be `#[repr(C)]` with alignment 1 so that
/// the instruction data ZC struct can be read via zero-copy pointer cast
/// from `&[u8]`.
pub trait InstructionArg: Sized {
    /// The alignment-1 companion type for zero-copy deserialization.
    type Zc: Copy;
    /// Reconstruct the native value from its ZC representation.
    fn from_zc(zc: &Self::Zc) -> Self;
}

// --- Identity impls (already alignment 1) ---

impl InstructionArg for u8 {
    type Zc = u8;
    #[inline(always)]
    fn from_zc(zc: &u8) -> u8 {
        *zc
    }
}

impl InstructionArg for i8 {
    type Zc = i8;
    #[inline(always)]
    fn from_zc(zc: &i8) -> i8 {
        *zc
    }
}

impl<const N: usize> InstructionArg for [u8; N] {
    type Zc = [u8; N];
    #[inline(always)]
    fn from_zc(zc: &[u8; N]) -> [u8; N] {
        *zc
    }
}

impl InstructionArg for solana_address::Address {
    type Zc = solana_address::Address;
    #[inline(always)]
    fn from_zc(zc: &solana_address::Address) -> solana_address::Address {
        *zc
    }
}

// --- Pod-mapped impls (native → Pod companion) ---

macro_rules! impl_instruction_arg_pod {
    ($native:ty, $pod:ty) => {
        impl InstructionArg for $native {
            type Zc = $pod;
            #[inline(always)]
            fn from_zc(zc: &$pod) -> $native {
                zc.get()
            }
        }
    };
}

impl_instruction_arg_pod!(u16, PodU16);
impl_instruction_arg_pod!(u32, PodU32);
impl_instruction_arg_pod!(u64, PodU64);
impl_instruction_arg_pod!(u128, PodU128);
impl_instruction_arg_pod!(i16, PodI16);
impl_instruction_arg_pod!(i32, PodI32);
impl_instruction_arg_pod!(i64, PodI64);
impl_instruction_arg_pod!(i128, PodI128);

impl InstructionArg for bool {
    type Zc = PodBool;
    #[inline(always)]
    fn from_zc(zc: &PodBool) -> bool {
        zc.get()
    }
}

// --- Pod types map to themselves ---

macro_rules! impl_instruction_arg_identity {
    ($($t:ty),*) => {$(
        impl InstructionArg for $t {
            type Zc = $t;
            #[inline(always)]
            fn from_zc(zc: &$t) -> $t { *zc }
        }
    )*}
}

impl_instruction_arg_identity!(
    PodU16, PodU32, PodU64, PodU128, PodI16, PodI32, PodI64, PodI128, PodBool
);
