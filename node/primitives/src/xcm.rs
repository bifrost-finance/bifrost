use codec::{Decode, Encode};
use sp_std::prelude::*;

/// The type used to represent the xcmp transfer direction
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub enum TransferOriginType {
	FromSelf = 0,
	FromRelayChain = 1,
	FromSiblingParaChain = 2,
}

pub struct XcmBaseWeight(u64);

impl From<u64> for XcmBaseWeight {
	fn from(u: u64) -> Self {
		XcmBaseWeight(u)
	}
}

impl From<XcmBaseWeight> for u64 {
	fn from(x: XcmBaseWeight) -> Self {
		x.0.into()
	}
}
