use crate::*;
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

// pub(crate) type BalanceOf<T> = <T as currency::Config>::Balance;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type BalanceOf<T> =
	<<T as crate::Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

pub type UnsignedFixedPoint<T> = <T as crate::Config>::UnsignedFixedPoint;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum Version {
	/// Initial version.
	V0,
}
