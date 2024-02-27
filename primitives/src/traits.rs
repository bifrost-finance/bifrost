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

#![allow(clippy::unnecessary_cast)]

use crate::{
	AssetIds, DerivativeIndex, ExtraFeeInfo, LeasePeriod, ParaId, PoolId, RedeemType, TokenId,
	TokenSymbol, XcmOperationType,
};
use frame_support::pallet_prelude::{DispatchResultWithPostInfo, Weight};
use parity_scale_codec::{Decode, Encode, FullCodec};
use sp_runtime::{
	traits::{
		AccountIdConversion, AtLeast32BitUnsigned, ConstU32, MaybeSerializeDeserialize, Zero,
	},
	BoundedVec, DispatchError, DispatchResult, TokenError, TypeId,
};
use sp_std::{fmt::Debug, vec::Vec};

pub trait TokenInfo {
	fn name(&self) -> Option<&str>;
	fn symbol(&self) -> Option<&str>;
	fn decimals(&self) -> Option<u8>;
}

/// Extension trait for CurrencyId
pub trait CurrencyIdExt {
	type TokenSymbol;
	fn is_vtoken(&self) -> bool;
	fn is_token(&self) -> bool;
	fn is_vstoken(&self) -> bool;
	fn is_vsbond(&self) -> bool;
	fn is_native(&self) -> bool;
	fn is_stable(&self) -> bool;
	fn is_lptoken(&self) -> bool;
	fn is_foreign_asset(&self) -> bool;
	fn into(symbol: Self::TokenSymbol) -> Self;
}

/// Extension traits for assets module
pub trait MultiCurrencyExt<AccountId> {
	/// The currency identifier.
	type CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug;

	/// The balance of an account.
	type Balance: AtLeast32BitUnsigned
		+ FullCodec
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ Default;

	/// Expand the total issuance by currency id
	fn expand_total_issuance(
		currency_id: Self::CurrencyId,
		amount: Self::Balance,
	) -> DispatchResult;

	/// Burn the total issuance by currency id
	fn reduce_total_issuance(
		currency_id: Self::CurrencyId,
		amount: Self::Balance,
	) -> DispatchResult;
}

pub trait BancorHandler<Balance> {
	fn add_token(currency_id: super::CurrencyId, amount: Balance) -> DispatchResult;
}

impl<Balance> BancorHandler<Balance> for () {
	fn add_token(_currency_id: super::CurrencyId, _amount: Balance) -> DispatchResult {
		DispatchResult::from(DispatchError::Token(TokenError::FundsUnavailable))
	}
}

pub trait CheckSubAccount<T: Encode + Decode> {
	fn check_sub_account<S: Decode>(&self, account: &T) -> bool;
}

impl<T, Id> CheckSubAccount<T> for Id
where
	T: Encode + Decode,
	Id: Encode + Decode + TypeId + AccountIdConversion<T> + Eq,
{
	fn check_sub_account<S: Decode>(&self, account: &T) -> bool {
		match Id::try_from_sub_account::<S>(account) {
			Some((id, _)) => id.eq(self),
			None => false,
		}
	}
}

/// The interface to call VtokenMinting module functions.
pub trait VtokenMintingOperator<CurrencyId, Balance, AccountId, TimeUnit> {
	/// Get the currency tokenpool amount.
	fn get_token_pool(currency_id: CurrencyId) -> Balance;

	/// Increase the token amount for the storage "token_pool" in the VtokenMining module.
	fn increase_token_pool(currency_id: CurrencyId, token_amount: Balance) -> DispatchResult;

	/// Decrease the token amount for the storage "token_pool" in the VtokenMining module.
	fn decrease_token_pool(currency_id: CurrencyId, token_amount: Balance) -> DispatchResult;

	/// Update the ongoing era for a CurrencyId.
	fn update_ongoing_time_unit(currency_id: CurrencyId, time_unit: TimeUnit) -> DispatchResult;

	/// Get the current era of a CurrencyId.
	fn get_ongoing_time_unit(currency_id: CurrencyId) -> Option<TimeUnit>;

	/// Get the the unlocking records of a certain time unit.
	fn get_unlock_records(
		currency_id: CurrencyId,
		time_unit: TimeUnit,
	) -> Option<(Balance, Vec<u32>)>;

	/// Revise the currency indexed unlocking record by some amount.
	fn deduct_unlock_amount(
		currency_id: CurrencyId,
		index: u32,
		deduct_amount: Balance,
	) -> DispatchResult;

	/// Get currency Entrance and Exit accounts.【entrance_account, exit_account】
	fn get_entrance_and_exit_accounts() -> (AccountId, AccountId);

	/// Get the token_unlock_ledger storage info to refund to the due era unlocking users.
	fn get_token_unlock_ledger(
		currency_id: CurrencyId,
		index: u32,
	) -> Option<(AccountId, Balance, TimeUnit, RedeemType<AccountId>)>;
	fn get_astar_parachain_id() -> u32;
	fn get_moonbeam_parachain_id() -> u32;
	fn get_hydradx_parachain_id() -> u32;
	fn get_interlay_parachain_id() -> u32;
	fn get_manta_parachain_id() -> u32;
}

/// Trait for Vtoken-Minting module to check whether accept redeeming or not.
pub trait SlpOperator<CurrencyId> {
	fn all_delegation_requests_occupied(currency_id: CurrencyId) -> bool;
}

/// Trait for Vtoken-Minting module to check whether accept redeeming or not.
pub trait SlpxOperator<Balance> {
	fn get_moonbeam_transfer_to_fee() -> Balance;
}

/// A mapping between CurrencyId and AssetMetadata.
pub trait CurrencyIdMapping<CurrencyId, MultiLocation, AssetMetadata> {
	/// Returns the AssetMetadata associated with a given `AssetIds`.
	fn get_asset_metadata(asset_ids: AssetIds) -> Option<AssetMetadata>;
	/// Returns the AssetMetadata associated with a given `CurrencyId`.
	fn get_currency_metadata(currency_id: CurrencyId) -> Option<AssetMetadata>;
	/// Returns the MultiLocation associated with a given CurrencyId.
	fn get_multi_location(currency_id: CurrencyId) -> Option<MultiLocation>;
	/// Returns the CurrencyId associated with a given MultiLocation.
	fn get_currency_id(multi_location: MultiLocation) -> Option<CurrencyId>;
}

pub trait CurrencyIdConversion<CurrencyId> {
	fn convert_to_token(currency_id: CurrencyId) -> Result<CurrencyId, ()>;
	fn convert_to_vtoken(currency_id: CurrencyId) -> Result<CurrencyId, ()>;
	fn convert_to_vstoken(currency_id: CurrencyId) -> Result<CurrencyId, ()>;
	fn convert_to_vsbond(
		currency_id: CurrencyId,
		index: crate::ParaId,
		first_slot: crate::LeasePeriod,
		last_slot: crate::LeasePeriod,
	) -> Result<CurrencyId, ()>;
}

pub trait CurrencyIdRegister<CurrencyId> {
	fn check_token_registered(token_symbol: TokenSymbol) -> bool;
	fn check_vtoken_registered(token_symbol: TokenSymbol) -> bool;
	fn check_vstoken_registered(token_symbol: TokenSymbol) -> bool;
	fn check_vsbond_registered(
		token_symbol: TokenSymbol,
		para_id: crate::ParaId,
		first_slot: crate::LeasePeriod,
		last_slot: crate::LeasePeriod,
	) -> bool;
	fn register_vtoken_metadata(token_symbol: TokenSymbol) -> DispatchResult;
	fn register_vstoken_metadata(token_symbol: TokenSymbol) -> DispatchResult;
	fn register_vsbond_metadata(
		token_symbol: TokenSymbol,
		para_id: crate::ParaId,
		first_slot: crate::LeasePeriod,
		last_slot: crate::LeasePeriod,
	) -> DispatchResult;
	fn check_token2_registered(token_id: TokenId) -> bool;
	fn check_vtoken2_registered(token_id: TokenId) -> bool;
	fn check_vstoken2_registered(token_id: TokenId) -> bool;
	fn check_vsbond2_registered(
		token_id: TokenId,
		para_id: crate::ParaId,
		first_slot: crate::LeasePeriod,
		last_slot: crate::LeasePeriod,
	) -> bool;
	fn register_vtoken2_metadata(token_id: TokenId) -> DispatchResult;
	fn register_vstoken2_metadata(token_id: TokenId) -> DispatchResult;
	fn register_vsbond2_metadata(
		token_id: TokenId,
		para_id: crate::ParaId,
		first_slot: crate::LeasePeriod,
		last_slot: crate::LeasePeriod,
	) -> DispatchResult;
	fn register_blp_metadata(pool_id: PoolId, decimals: u8) -> DispatchResult;
}

impl<CurrencyId> CurrencyIdRegister<CurrencyId> for () {
	fn check_token_registered(_token_symbol: TokenSymbol) -> bool {
		false
	}

	fn check_vtoken_registered(_token_symbol: TokenSymbol) -> bool {
		false
	}

	fn check_vstoken_registered(_token_symbol: TokenSymbol) -> bool {
		false
	}

	fn check_vsbond_registered(
		_token_symbol: TokenSymbol,
		_para_id: ParaId,
		_first_slot: LeasePeriod,
		_last_slot: LeasePeriod,
	) -> bool {
		false
	}

	fn register_vtoken_metadata(_token_symbol: TokenSymbol) -> DispatchResult {
		Ok(())
	}

	fn register_vstoken_metadata(_token_symbol: TokenSymbol) -> DispatchResult {
		Ok(())
	}

	fn register_vsbond_metadata(
		_token_symbol: TokenSymbol,
		_para_id: ParaId,
		_first_slot: LeasePeriod,
		_last_slot: LeasePeriod,
	) -> DispatchResult {
		Ok(())
	}

	fn check_token2_registered(_token_id: TokenId) -> bool {
		false
	}

	fn check_vtoken2_registered(_token_id: TokenId) -> bool {
		false
	}

	fn check_vstoken2_registered(_token_id: TokenId) -> bool {
		false
	}

	fn check_vsbond2_registered(
		_token_id: TokenId,
		_para_id: ParaId,
		_first_slot: LeasePeriod,
		_last_slot: LeasePeriod,
	) -> bool {
		false
	}

	fn register_vtoken2_metadata(_token_id: TokenId) -> DispatchResult {
		Ok(())
	}

	fn register_vstoken2_metadata(_token_id: TokenId) -> DispatchResult {
		Ok(())
	}

	fn register_vsbond2_metadata(
		_token_id: TokenId,
		_para_id: ParaId,
		_first_slot: LeasePeriod,
		_last_slot: LeasePeriod,
	) -> DispatchResult {
		Ok(())
	}

	fn register_blp_metadata(_pool_id: PoolId, _decimals: u8) -> DispatchResult {
		Ok(())
	}
}

/// The interface to call farming pallet functions.
pub trait FarmingInfo<Balance, CurrencyId> {
	/// Get the currency token shares.
	fn get_token_shares(pool_id: PoolId, currency_id: CurrencyId) -> Balance;
}

pub trait VtokenMintingInterface<AccountId, CurrencyId, Balance> {
	fn mint(
		exchanger: AccountId,
		token_id: CurrencyId,
		token_amount: Balance,
		remark: BoundedVec<u8, ConstU32<32>>,
		channel_id: Option<u32>,
	) -> Result<Balance, DispatchError>;
	fn redeem(
		exchanger: AccountId,
		vtoken_id: CurrencyId,
		vtoken_amount: Balance,
	) -> DispatchResultWithPostInfo;
	fn slpx_redeem(
		exchanger: AccountId,
		vtoken_id: CurrencyId,
		vtoken_amount: Balance,
		redeem: RedeemType<AccountId>,
	) -> DispatchResultWithPostInfo;
	fn token_to_vtoken(
		token_id: CurrencyId,
		vtoken_id: CurrencyId,
		token_amount: Balance,
	) -> Result<Balance, DispatchError>;
	fn vtoken_to_token(
		token_id: CurrencyId,
		vtoken_id: CurrencyId,
		vtoken_amount: Balance,
	) -> Result<Balance, DispatchError>;
	fn vtoken_id(token_id: CurrencyId) -> Option<CurrencyId>;
	fn token_id(vtoken_id: CurrencyId) -> Option<CurrencyId>;
	fn get_token_pool(currency_id: CurrencyId) -> Balance;
	fn get_minimums_redeem(vtoken_id: CurrencyId) -> Balance;
	fn get_astar_parachain_id() -> u32;
	fn get_moonbeam_parachain_id() -> u32;
	fn get_hydradx_parachain_id() -> u32;
	fn get_interlay_parachain_id() -> u32;
	fn get_manta_parachain_id() -> u32;
}

impl<AccountId, CurrencyId, Balance: Zero> VtokenMintingInterface<AccountId, CurrencyId, Balance>
	for ()
{
	fn mint(
		_exchanger: AccountId,
		_token_id: CurrencyId,
		_token_amount: Balance,
		_remark: BoundedVec<u8, ConstU32<32>>,
		_channel_id: Option<u32>,
	) -> Result<Balance, DispatchError> {
		Ok(Zero::zero())
	}

	fn redeem(
		_exchanger: AccountId,
		_vtoken_id: CurrencyId,
		_vtoken_amount: Balance,
	) -> DispatchResultWithPostInfo {
		Ok(().into())
	}

	fn slpx_redeem(
		_exchanger: AccountId,
		_vtoken_id: CurrencyId,
		_vtoken_amount: Balance,
		_redeem_type: RedeemType<AccountId>,
	) -> DispatchResultWithPostInfo {
		Ok(().into())
	}

	fn token_to_vtoken(
		_token_id: CurrencyId,
		_vtoken_id: CurrencyId,
		_token_amount: Balance,
	) -> Result<Balance, DispatchError> {
		Ok(Zero::zero())
	}

	fn vtoken_to_token(
		_token_id: CurrencyId,
		_vtoken_id: CurrencyId,
		_vtoken_amount: Balance,
	) -> Result<Balance, DispatchError> {
		Ok(Zero::zero())
	}

	fn vtoken_id(_token_id: CurrencyId) -> Option<CurrencyId> {
		None
	}

	fn token_id(_vtoken_id: CurrencyId) -> Option<CurrencyId> {
		None
	}

	fn get_token_pool(_currency_id: CurrencyId) -> Balance {
		Zero::zero()
	}

	fn get_minimums_redeem(_vtoken_id: CurrencyId) -> Balance {
		Zero::zero()
	}

	fn get_astar_parachain_id() -> u32 {
		0
	}
	fn get_moonbeam_parachain_id() -> u32 {
		0
	}
	fn get_hydradx_parachain_id() -> u32 {
		0
	}
	fn get_interlay_parachain_id() -> u32 {
		0
	}
	fn get_manta_parachain_id() -> u32 {
		0
	}
}

pub trait TryConvertFrom<CurrencyId> {
	type Error;
	fn try_convert_from(currency_id: CurrencyId, para_id: u32) -> Result<Self, Self::Error>
	where
		Self: Sized;
}

pub trait XcmDestWeightAndFeeHandler<CurrencyId, Balance>
where
	Balance: AtLeast32BitUnsigned,
{
	fn get_operation_weight_and_fee(
		token: CurrencyId,
		operation: XcmOperationType,
	) -> Option<(Weight, Balance)>;

	fn set_xcm_dest_weight_and_fee(
		currency_id: CurrencyId,
		operation: XcmOperationType,
		weight_and_fee: Option<(Weight, Balance)>,
	) -> DispatchResult;
}

impl<CurrencyId, Balance> XcmDestWeightAndFeeHandler<CurrencyId, Balance> for ()
where
	Balance: AtLeast32BitUnsigned,
{
	fn get_operation_weight_and_fee(
		_token: CurrencyId,
		_operation: XcmOperationType,
	) -> Option<(Weight, Balance)> {
		Some((Zero::zero(), Zero::zero()))
	}

	fn set_xcm_dest_weight_and_fee(
		_currency_id: CurrencyId,
		_operation: XcmOperationType,
		_weight_and_fee: Option<(Weight, Balance)>,
	) -> DispatchResult {
		Ok(())
	}
}

pub trait FeeGetter<RuntimeCall> {
	fn get_fee_info(call: &RuntimeCall) -> ExtraFeeInfo;
}

pub trait DerivativeAccountHandler<CurrencyId, Balance> {
	fn check_derivative_index_exists(token: CurrencyId, derivative_index: DerivativeIndex) -> bool;

	fn get_multilocation(
		token: CurrencyId,
		derivative_index: DerivativeIndex,
	) -> Option<xcm::v3::MultiLocation>;

	fn get_stake_info(
		token: CurrencyId,
		derivative_index: DerivativeIndex,
	) -> Option<(Balance, Balance)>;

	#[cfg(feature = "runtime-benchmarks")]
	fn init_minimums_and_maximums(token: CurrencyId);

	#[cfg(feature = "runtime-benchmarks")]
	fn new_delegator_ledger(token: CurrencyId, who: xcm::v3::MultiLocation);

	#[cfg(feature = "runtime-benchmarks")]
	fn add_delegator(token: CurrencyId, index: DerivativeIndex, who: xcm::v3::MultiLocation);
}

pub trait VTokenSupplyProvider<CurrencyId, Balance> {
	fn get_vtoken_supply(vtoken: CurrencyId) -> Option<Balance>;

	fn get_token_supply(token: CurrencyId) -> Option<Balance>;
}

// traits for pallet channel-commission
pub trait VTokenMintRedeemProvider<CurrencyId, Balance> {
	// record the mint amount of vtoken
	fn record_mint_amount(
		channel_id: Option<u32>,
		vtoken: CurrencyId,
		amount: Balance,
	) -> Result<(), DispatchError>;
	// record the redeem amount of vtoken
	fn record_redeem_amount(vtoken: CurrencyId, amount: Balance) -> Result<(), DispatchError>;
}

impl<CurrencyId, Balance> VTokenMintRedeemProvider<CurrencyId, Balance> for () {
	fn record_mint_amount(
		_channel_id: Option<u32>,
		_vtoken: CurrencyId,
		_amount: Balance,
	) -> Result<(), DispatchError> {
		Ok(())
	}

	fn record_redeem_amount(_vtoken: CurrencyId, _amount: Balance) -> Result<(), DispatchError> {
		Ok(())
	}
}

pub trait SlpHostingFeeProvider<CurrencyId, Balance, AccountId> {
	// record the hosting fee
	fn record_hosting_fee(
		commission_token: CurrencyId,
		amount: Balance,
	) -> Result<(), DispatchError>;
}

impl<CurrencyId, Balance, AccountId> SlpHostingFeeProvider<CurrencyId, Balance, AccountId> for () {
	fn record_hosting_fee(
		_commission_token: CurrencyId,
		_amount: Balance,
	) -> Result<(), DispatchError> {
		Ok(())
	}
}
