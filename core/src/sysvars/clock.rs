use crate::impl_sysvar_get;
use crate::pod::{PodI64, PodU64};
use {
    crate::sysvars::Sysvar,
    core::mem::{align_of, size_of},
    solana_account_view::{AccountView, Ref},
    solana_address::Address,
    solana_program_error::ProgramError,
};

pub const CLOCK_ID: Address = Address::new_from_array([
    6, 167, 213, 23, 24, 199, 116, 201, 40, 86, 99, 152, 105, 29, 94, 182, 139, 94, 184, 163, 155,
    75, 109, 92, 115, 85, 91, 33, 0, 0, 0, 0,
]);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Clock {
    pub slot: PodU64,
    pub epoch_start_timestamp: PodI64,
    pub epoch: PodU64,
    pub leader_schedule_epoch: PodU64,
    pub unix_timestamp: PodI64,
}

const _ASSERT_STRUCT_LEN: () = assert!(size_of::<Clock>() == 40);
const _ASSERT_STRUCT_ALIGN: () = assert!(align_of::<Clock>() == 1);

impl Clock {
    #[inline]
    pub fn from_account_view(account_view: &AccountView) -> Result<Ref<'_, Clock>, ProgramError> {
        if account_view.address() != &CLOCK_ID {
            return Err(ProgramError::InvalidArgument);
        }
        Ok(Ref::map(account_view.try_borrow()?, |data| unsafe {
            Self::from_bytes_unchecked(data)
        }))
    }

    /// # Safety
    ///
    /// Caller must ensure `bytes.len() >= size_of::<Clock>()` and that the data is
    /// a valid Clock sysvar.
    #[inline(always)]
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> &Self {
        unsafe { &*(bytes.as_ptr() as *const Clock) }
    }
}

impl Sysvar for Clock {
    impl_sysvar_get!(CLOCK_ID, 0);
}
