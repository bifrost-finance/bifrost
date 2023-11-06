// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
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

use crate::{pallet::Error, AccountIdOf, BalanceOf, Config, CurrencyId};
use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use node_primitives::{currency::KSM, DOT};
use scale_info::TypeInfo;
use sp_runtime::traits::StaticLookup;
use sp_std::{boxed::Box, vec::Vec};
use xcm::{
	opaque::v3::Instruction,
	v3::{prelude::*, Weight as XcmWeight},
	VersionedMultiAssets, VersionedMultiLocation,
};

#[derive(Encode, Decode, RuntimeDebug)]
pub enum SubstrateCall<T: Config> {
	Kusama(KusamaCall<T>),
	Polkadot(PolkadotCall<T>),
}

impl<T: Config> SubstrateCall<T> {
	pub fn get_bond_call(
		currency_id: CurrencyId,
		amount: BalanceOf<T>,
		delegator_account: AccountIdOf<T>,
	) -> Result<Self, Error<T>> {
		match currency_id {
			KSM => Ok(Self::Kusama(KusamaCall::Staking(StakingCall::Bond(
				T::Lookup::unlookup(delegator_account),
				amount,
				RewardDestination::<AccountIdOf<T>>::Staked,
			)))),
			DOT => Ok(Self::Polkadot(PolkadotCall::Staking(StakingCall::Bond(
				T::Lookup::unlookup(delegator_account),
				amount,
				RewardDestination::<AccountIdOf<T>>::Staked,
			)))),
			_ => Err(Error::<T>::NotSupportedCurrencyId),
		}
	}

	pub fn get_bond_extra_call(
		currency_id: CurrencyId,
		amount: BalanceOf<T>,
	) -> Result<Self, Error<T>> {
		match currency_id {
			KSM => Ok(Self::Kusama(KusamaCall::Staking(StakingCall::BondExtra(amount)))),
			DOT => Ok(Self::Polkadot(PolkadotCall::Staking(StakingCall::BondExtra(amount)))),
			_ => Err(Error::NotSupportedCurrencyId),
		}
	}

	pub fn get_unbond_call(
		currency_id: CurrencyId,
		amount: BalanceOf<T>,
	) -> Result<Self, Error<T>> {
		match currency_id {
			KSM => Ok(Self::Kusama(KusamaCall::Staking(StakingCall::Unbond(amount)))),
			DOT => Ok(Self::Polkadot(PolkadotCall::Staking(StakingCall::Unbond(amount)))),
			_ => Err(Error::NotSupportedCurrencyId),
		}
	}

	pub fn get_rebond_call(
		currency_id: CurrencyId,
		amount: BalanceOf<T>,
	) -> Result<Self, Error<T>> {
		match currency_id {
			KSM => Ok(Self::Kusama(KusamaCall::Staking(StakingCall::Rebond(amount)))),
			DOT => Ok(Self::Polkadot(PolkadotCall::Staking(StakingCall::Rebond(amount)))),
			_ => Err(Error::NotSupportedCurrencyId),
		}
	}

	pub fn get_nominate_call(
		currency_id: CurrencyId,
		accounts: Vec<<T::Lookup as StaticLookup>::Source>,
	) -> Result<Self, Error<T>> {
		match currency_id {
			KSM => Ok(Self::Kusama(KusamaCall::Staking(StakingCall::Nominate(accounts)))),
			DOT => Ok(Self::Polkadot(PolkadotCall::Staking(StakingCall::Nominate(accounts)))),
			_ => Err(Error::NotSupportedCurrencyId),
		}
	}

	pub fn get_payout_stakers_call(
		currency_id: CurrencyId,
		validator_account: AccountIdOf<T>,
		payout_era: u32,
	) -> Result<Self, Error<T>> {
		match currency_id {
			KSM => Ok(Self::Kusama(KusamaCall::Staking(StakingCall::PayoutStakers(
				validator_account,
				payout_era,
			)))),
			DOT => Ok(Self::Polkadot(PolkadotCall::Staking(StakingCall::PayoutStakers(
				validator_account,
				payout_era,
			)))),
			_ => Err(Error::NotSupportedCurrencyId),
		}
	}

	pub fn get_withdraw_unbonded_call(
		currency_id: CurrencyId,
		num_slashing_spans: u32,
	) -> Result<Self, Error<T>> {
		match currency_id {
			KSM => Ok(Self::Kusama(KusamaCall::Staking(StakingCall::WithdrawUnbonded(
				num_slashing_spans,
			)))),
			DOT => Ok(Self::Polkadot(PolkadotCall::Staking(StakingCall::WithdrawUnbonded(
				num_slashing_spans,
			)))),
			_ => Err(Error::NotSupportedCurrencyId),
		}
	}

	pub fn get_chill_call(currency_id: CurrencyId) -> Result<Self, Error<T>> {
		match currency_id {
			KSM => Ok(Self::Kusama(KusamaCall::Staking(StakingCall::Chill))),
			DOT => Ok(Self::Polkadot(PolkadotCall::Staking(StakingCall::Chill))),
			_ => Err(Error::NotSupportedCurrencyId),
		}
	}

	pub fn get_reserve_transfer_assets_call(
		currency_id: CurrencyId,
		dest: Box<VersionedMultiLocation>,
		beneficiary: Box<VersionedMultiLocation>,
		assets: Box<VersionedMultiAssets>,
		fee_asset_item: u32,
		weight_limit: WeightLimit,
	) -> Result<Self, Error<T>> {
		match currency_id {
			KSM =>
				Ok(Self::Kusama(KusamaCall::Xcm(Box::new(XcmCall::LimitedReserveTransferAssets(
					dest,
					beneficiary,
					assets,
					fee_asset_item,
					weight_limit,
				))))),
			DOT => Ok(Self::Polkadot(PolkadotCall::Xcm(Box::new(
				XcmCall::LimitedReserveTransferAssets(
					dest,
					beneficiary,
					assets,
					fee_asset_item,
					weight_limit,
				),
			)))),
			_ => Err(Error::NotSupportedCurrencyId),
		}
	}

	pub fn get_call_as_subaccount_from_call(
		self,
		sub_account_index: u16,
	) -> Result<Self, Error<T>> {
		match self {
			SubstrateCall::Kusama(kusama_call) =>
				kusama_call.get_call_as_subaccount_from_call(sub_account_index),
			SubstrateCall::Polkadot(polkadot_call) =>
				polkadot_call.get_call_as_subaccount_from_call(sub_account_index),
		}
	}

	pub fn get_transact_instruct(self, weight: XcmWeight) -> Instruction {
		let encoded_call = match &self {
			SubstrateCall::Kusama(call) => call.encode(),
			SubstrateCall::Polkadot(call) => call.encode(),
		};

		Transact {
			origin_kind: OriginKind::SovereignAccount,
			require_weight_at_most: weight,
			call: encoded_call.into(),
		}
	}
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum KusamaCall<T: Config> {
	#[codec(index = 0)]
	System(SystemCall),
	#[codec(index = 4)]
	Balances(BalancesCall<T>),
	#[codec(index = 6)]
	Staking(StakingCall<T>),
	#[codec(index = 24)]
	Utility(Box<KusamaUtilityCall<Self>>),
	#[codec(index = 99)]
	Xcm(Box<XcmCall>),
}

impl<T: Config> KusamaCall<T> {
	pub fn get_derivative_call(sub_account_index: u16, call: Self) -> Self {
		Self::Utility(Box::new(KusamaUtilityCall::AsDerivative(sub_account_index, Box::new(call))))
	}

	pub fn get_call_as_subaccount_from_call(
		self,
		sub_account_index: u16,
	) -> Result<SubstrateCall<T>, Error<T>> {
		let derivative_call = KusamaCall::<T>::get_derivative_call(sub_account_index, self);

		Ok(SubstrateCall::<T>::Kusama(derivative_call))
	}
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum PolkadotCall<T: Config> {
	#[codec(index = 0)]
	System(SystemCall),
	#[codec(index = 5)]
	Balances(BalancesCall<T>),
	#[codec(index = 7)]
	Staking(StakingCall<T>),
	#[codec(index = 26)]
	Utility(Box<PolkadotUtilityCall<Self>>),
	#[codec(index = 99)]
	Xcm(Box<XcmCall>),
}

impl<T: Config> PolkadotCall<T> {
	pub fn get_derivative_call(sub_account_index: u16, call: Self) -> Self {
		Self::Utility(Box::new(PolkadotUtilityCall::AsDerivative(
			sub_account_index,
			Box::new(call),
		)))
	}

	pub fn get_call_as_subaccount_from_call(
		self,
		sub_account_index: u16,
	) -> Result<SubstrateCall<T>, Error<T>> {
		let derivative_call = PolkadotCall::<T>::get_derivative_call(sub_account_index, self);

		Ok(SubstrateCall::<T>::Polkadot(derivative_call))
	}
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum SystemCall {
	#[codec(index = 7)]
	RemarkWithEvent(Box<Vec<u8>>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum BalancesCall<T: Config> {
	#[codec(index = 3)]
	TransferKeepAlive(<T::Lookup as StaticLookup>::Source, #[codec(compact)] BalanceOf<T>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum KusamaUtilityCall<KusamaCall> {
	#[codec(index = 1)]
	AsDerivative(u16, Box<KusamaCall>),
	#[codec(index = 2)]
	BatchAll(Box<Vec<Box<KusamaCall>>>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum PolkadotUtilityCall<PolkadotCall> {
	#[codec(index = 1)]
	AsDerivative(u16, Box<PolkadotCall>),
	#[codec(index = 2)]
	BatchAll(Box<Vec<Box<PolkadotCall>>>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum StakingCall<T: Config> {
	/// Kusama/Polkadot has the same account Id type as Bifrost.
	#[codec(index = 0)]
	Bond(
		<T::Lookup as StaticLookup>::Source,
		#[codec(compact)] BalanceOf<T>,
		RewardDestination<T::AccountId>,
	),
	#[codec(index = 1)]
	BondExtra(#[codec(compact)] BalanceOf<T>),
	#[codec(index = 2)]
	Unbond(#[codec(compact)] BalanceOf<T>),
	#[codec(index = 3)]
	WithdrawUnbonded(u32),
	#[codec(index = 5)]
	Nominate(Vec<<T::Lookup as StaticLookup>::Source>),
	#[codec(index = 6)]
	Chill,
	#[codec(index = 18)]
	PayoutStakers(T::AccountId, u32),
	#[codec(index = 19)]
	Rebond(#[codec(compact)] BalanceOf<T>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum XcmCall {
	#[codec(index = 8)]
	LimitedReserveTransferAssets(
		Box<VersionedMultiLocation>,
		Box<VersionedMultiLocation>,
		Box<VersionedMultiAssets>,
		u32,
		WeightLimit,
	),
}

/// A destination account for payment.
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum RewardDestination<AccountId> {
	/// Pay into the stash account, increasing the amount at stake accordingly.
	Staked,
	/// Pay into the stash account, not increasing the amount at stake.
	Stash,
	/// Pay into the controller account.
	Controller,
	/// Pay into a specified account.
	Account(AccountId),
	/// Receive no reward.
	None,
}
