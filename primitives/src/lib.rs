// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Low-level types used throughout the Bifrost code.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{parameter_types, PalletId};
use hex_literal::hex;
use parity_scale_codec::MaxEncodedLen;
use scale_info::TypeInfo;
use sp_core::{Decode, Encode, RuntimeDebug, H160};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	FixedU128, MultiSignature, OpaqueExtrinsic, Permill,
};

pub mod currency;
pub use currency::*;
pub mod xcm;
pub use crate::xcm::*;
pub mod mock_xcm;
pub use crate::mock_xcm::*;

pub mod price;
pub use crate::price::*;
pub mod salp;
pub use salp::*;
pub mod traits;
pub use crate::traits::*;
pub mod time_unit;
pub use crate::time_unit::*;

#[cfg(test)]
mod tests;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them.
pub type AccountIndex = u32;

/// An index to an asset
pub type AssetId = u32;

/// Vtoken Mint type
pub type VtokenMintPrice = u128;

/// Balance of an account.
pub type Balance = u128;

/// Price of an asset.
pub type Price = FixedU128;

pub type PriceDetail = (Price, Timestamp);

/// Precision of symbol.
pub type Precision = u32;

/// Type used for expressing timestamp.
pub type Moment = u64;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// A timestamp: milliseconds since the unix epoch.
/// `u64` is enough to represent a duration of half a billion years, when the
/// time scale is milliseconds.
pub type Timestamp = u64;

/// Digest item type.
pub type DigestItem = generic::DigestItem;

/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type.
pub type Block = generic::Block<Header, OpaqueExtrinsic>;

/// Block ID.
pub type BlockId = generic::BlockId<Block>;

/// Balancer pool swap fee.
pub type SwapFee = u128;

/// Balancer pool ID.
pub type PoolId = u32;

/// Balancer pool weight.
pub type PoolWeight = u128;

/// Balancer pool token.
pub type PoolToken = u128;

/// Index of a transaction in the chain. 32-bit should be plenty.
pub type Nonce = u32;

///
pub type BiddingOrderId = u64;

///
pub type EraId = u32;

/// Signed version of Balance
pub type Amount = i128;

/// Parachain Id
pub type ParaId = u32;

/// The measurement type for counting lease periods (generally the same as `BlockNumber`).
pub type LeasePeriod = BlockNumber;

/// Index used for the child trie
pub type TrieIndex = u32;

/// Distribution Id
pub type DistributionId = u32;

/// The fixed point number
pub type Rate = FixedU128;

/// The fixed point number, range from 0 to 1.
pub type Ratio = Permill;

pub type Liquidity = FixedU128;

pub type Shortfall = FixedU128;

pub const SECONDS_PER_YEAR: Timestamp = 365 * 24 * 60 * 60;

pub type DerivativeIndex = u16;

pub type TimeStampedPrice = orml_oracle::TimestampedValue<Price, Moment>;

// Pallet Id
parameter_types! {
	pub const BifrostCrowdloanId: PalletId = PalletId(*b"bf/salp#");
	pub const BifrostEntranceAccount: PalletId = PalletId(*b"bf/vtkin");
	pub const BifrostExitAccount: PalletId = PalletId(*b"bf/vtout");
	pub const BifrostSalpLiteCrowdloanId: PalletId = PalletId(*b"bf/salpl");
	pub const BifrostVsbondAccount: PalletId = PalletId(*b"bf/salpb");
	pub const BuyBackAccount: PalletId = PalletId(*b"bf/bybck");
	pub const BuybackPalletId: PalletId = PalletId(*b"bf/salpc");
	pub const CloudsPalletId: PalletId = PalletId(*b"bf/cloud");
	pub const CommissionPalletId: PalletId = PalletId(*b"bf/comms");
	pub const FarmingBoostPalletId: PalletId = PalletId(*b"bf/fmbst");
	pub const FarmingGaugeRewardIssuerPalletId: PalletId = PalletId(*b"bf/fmgar");
	pub const FarmingKeeperPalletId: PalletId = PalletId(*b"bf/fmkpr");
	pub const FarmingRewardIssuerPalletId: PalletId = PalletId(*b"bf/fmrir");
	pub const FeeSharePalletId: PalletId = PalletId(*b"bf/feesh");
	pub const FlexibleFeePalletId: PalletId = PalletId(*b"bf/flexi");
	pub const IncentivePoolAccount: PalletId = PalletId(*b"bf/inpoo");
	pub const IncentivePalletId: PalletId = PalletId(*b"bf/bbict");
	pub const LendMarketPalletId: PalletId = PalletId(*b"bf/ldmkt");
	pub const LighteningRedeemPalletId: PalletId = PalletId(*b"lighten#");
	pub const LiquidityAccount: PalletId = PalletId(*b"bf/liqdt");
	pub const LiquidityMiningDOTPalletId: PalletId = PalletId(*b"bf/lmdot");
	pub const LiquidityMiningPalletId: PalletId = PalletId(*b"mining##");
	pub const MerkleDirtributorPalletId: PalletId = PalletId(*b"bf/mklds");
	pub const OraclePalletId: PalletId = PalletId(*b"bf/oracl");
	pub const ParachainStakingPalletId: PalletId = PalletId(*b"bf/stake");
	pub const SlpEntrancePalletId: PalletId = PalletId(*b"bf/vtkin");
	pub const SlpExitPalletId: PalletId = PalletId(*b"bf/vtout");
	pub const StableAssetPalletId: PalletId = PalletId(*b"nuts/sta");
	pub const SystemMakerPalletId: PalletId = PalletId(*b"bf/sysmk");
	pub const SystemStakingPalletId: PalletId = PalletId(*b"bf/sysst");
	pub const TreasuryPalletId: PalletId = PalletId(*b"bf/trsry");
	pub const VBNCConvertPalletId: PalletId = PalletId(*b"bf/vbncc");
	pub const VeMintingPalletId: PalletId = PalletId(*b"bf/vemnt");
	// unused after vsbond_auction pallet removed
	pub const VsbondAuctionPalletId: PalletId = PalletId(*b"bf/vsbnd");
	pub const ZenlinkPalletId: PalletId = PalletId(*b"/zenlink");
}

// Account Id
parameter_types! {
	pub BifrostFeeAccount: AccountId = hex!["e4da05f08e89bf6c43260d96f26fffcfc7deae5b465da08669a9d008e64c2c63"].into();
}

// For vtoken-minting
#[derive(
	PartialEq, Eq, Clone, Encode, Decode, MaxEncodedLen, RuntimeDebug, scale_info::TypeInfo,
)]
pub enum RedeemType<AccountId> {
	/// Native chain.
	Native,
	/// Astar chain.
	Astar(AccountId),
	/// Moonbeam chain.
	Moonbeam(H160),
	/// Hydradx chain.
	Hydradx(AccountId),
	/// Interlay chain.
	Interlay(AccountId),
	/// Manta chain.
	Manta(AccountId),
}

impl<AccountId> Default for RedeemType<AccountId> {
	fn default() -> Self {
		Self::Native
	}
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo)]
pub enum XcmOperationType {
	// SALP operations
	UmpContributeTransact,
	// Statemine operations
	StatemineTransfer,
	// SLP operations
	Bond,
	WithdrawUnbonded,
	BondExtra,
	Unbond,
	Rebond,
	Delegate,
	Payout,
	Liquidize,
	TransferBack,
	TransferTo,
	Chill,
	Undelegate,
	CancelLeave,
	XtokensTransferBack,
	ExecuteLeave,
	ConvertAsset,
	// VtokenVoting operations
	Vote,
	RemoveVote,
	Any,
	SupplementaryFee,
	EthereumTransfer,
	TeleportAssets,
}
