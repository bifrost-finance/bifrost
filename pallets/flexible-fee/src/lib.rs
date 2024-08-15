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

#![cfg_attr(not(feature = "std"), no_std)]

pub use crate::pallet::*;
use bifrost_primitives::{
	currency::WETH,
	traits::{FeeGetter, XcmDestWeightAndFeeHandler},
	xcm_interface::parachains,
	AccountFeeCurrency, CurrencyId, ExtraFeeName, TryConvertFrom, XcmOperationType, BNC,
};
use bifrost_xcm_interface::{polkadot::RelaychainCall, PolkadotXcmCall};
use core::convert::Into;
use cumulus_primitives_core::ParaId;
use frame_support::{
	pallet_prelude::*,
	traits::{
		Currency, ExistenceRequirement, Get, Imbalance, OnUnbalanced, ReservableCurrency,
		WithdrawReasons,
	},
	transactional,
	weights::WeightMeter,
	PalletId,
};
use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;
use pallet_transaction_payment::OnChargeTransaction;
use polkadot_parachain_primitives::primitives::Sibling;
use sp_arithmetic::traits::{CheckedAdd, SaturatedConversion, UniqueSaturatedInto};
use sp_runtime::{
	traits::{AccountIdConversion, DispatchInfoOf, PostDispatchInfoOf, Saturating, Zero},
	transaction_validity::TransactionValidityError,
	BoundedVec,
};
use sp_std::{boxed::Box, vec, vec::Vec};
pub use weights::WeightInfo;
use xcm::{prelude::Unlimited, v4::prelude::*};
use zenlink_protocol::{AssetBalance, AssetId, ExportZenlink};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migrations;
mod mock;
mod tests;
pub mod weights;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type CurrencyIdOf<T> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::CurrencyId;
pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;
pub type PalletBalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
pub type NegativeImbalanceOf<T> =
	<<T as Config>::Currency as Currency<AccountIdOf<T>>>::NegativeImbalance;
pub type PositiveImbalanceOf<T> =
	<<T as Config>::Currency as Currency<AccountIdOf<T>>>::PositiveImbalance;
pub type CallOf<T> = <T as frame_system::Config>::RuntimeCall;

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum TargetChain {
	AssetHub,
	RelayChain,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_transaction_payment::Config {
		/// Event
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;
		/// Handler for both NativeCurrency and MultiCurrency
		type MultiCurrency: MultiCurrency<
			Self::AccountId,
			CurrencyId = CurrencyId,
			Balance = PalletBalanceOf<Self>,
		>;
		/// The currency type in which fees will be paid.
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		/// Handler for the unbalanced decrease
		type OnUnbalanced: OnUnbalanced<NegativeImbalanceOf<Self>>;
		/// xcm transfer interface
		type XcmRouter: SendXcm;

		type DexOperator: ExportZenlink<Self::AccountId, AssetId>;
		/// Filter if this transaction needs to be deducted extra fee besides basic transaction fee,
		/// and get the name of the fee
		type ExtraFeeMatcher: FeeGetter<CallOf<Self>>;

		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;

		#[pallet::constant]
		type MaxFeeCurrencyOrderListLen: Get<u32>;

		#[pallet::constant]
		type MinAssetHubExecutionFee: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MinRelaychainExecutionFee: Get<BalanceOf<Self>>;

		/// The currency id of the RelayChain
		#[pallet::constant]
		type RelaychainCurrencyId: Get<CurrencyIdOf<Self>>;

		type ParachainId: Get<ParaId>;

		/// The only origin that can set universal fee currency order list
		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		type XcmWeightAndFeeHandler: XcmDestWeightAndFeeHandler<CurrencyId, PalletBalanceOf<Self>>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(_n: BlockNumberFor<T>, limit: Weight) -> Weight {
			let mut weight = Weight::default();

			if WeightMeter::with_limit(limit)
				.try_consume(T::DbWeight::get().reads_writes(4, 2))
				.is_err()
			{
				return weight;
			}

			weight += T::DbWeight::get().reads_writes(4, 2);

			if Self::handle_fee().is_err() {
				return weight;
			}

			weight += T::DbWeight::get().reads_writes(1, 2);
			weight
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		TransferTo {
			from: T::AccountId,
			target_chain: TargetChain,
			amount: BalanceOf<T>,
		},
		FlexibleFeeExchanged {
			transaction_fee_currency: CurrencyIdOf<T>,
			transaction_fee_amount: PalletBalanceOf<T>,
		}, // token and amount
		FixedRateFeeExchanged(CurrencyIdOf<T>, PalletBalanceOf<T>),
		// [extra_fee_name, currency_id, amount_in, BNC_amount_out]
		ExtraFeeDeducted {
			operation: ExtraFeeName,
			transaction_extra_fee_currency: CurrencyIdOf<T>,
			transaction_extra_fee_amount: PalletBalanceOf<T>,
			transaction_extra_fee_bnc_amount: PalletBalanceOf<T>,
			transaction_extra_fee_receiver: T::AccountId,
		},
	}

	/// The current storage version, we set to 2 our new version(after migrate stroage from vec t
	/// boundedVec).
	#[allow(unused)]
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	/// Universal fee currency order list for all users
	#[pallet::storage]
	#[pallet::getter(fn get_universal_fee_currency_order_list)]
	pub type UniversalFeeCurrencyOrderList<T: Config> =
		StorageValue<_, BoundedVec<CurrencyIdOf<T>, T::MaxFeeCurrencyOrderListLen>, ValueQuery>;

	/// User default fee currency, if set, will be used as the first fee currency, and then use the
	/// universal fee currency order list
	#[pallet::storage]
	#[pallet::getter(fn get_user_default_fee_currency)]
	pub type UserDefaultFeeCurrency<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, CurrencyIdOf<T>, OptionQuery>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		Overflow,
		ConversionError,
		WrongListLength,
		WeightAndFeeNotExist,
		DexFailedToGetAmountInByPath,
		UnweighableMessage,
		XcmExecutionFailed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set user default fee currency
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::set_user_default_fee_currency())]
		pub fn set_user_default_fee_currency(
			origin: OriginFor<T>,
			maybe_fee_currency: Option<CurrencyIdOf<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if let Some(fee_currency) = maybe_fee_currency {
				UserDefaultFeeCurrency::<T>::insert(&who, fee_currency);
			} else {
				UserDefaultFeeCurrency::<T>::remove(&who);
			}

			Ok(())
		}

		/// Set universal fee currency order list
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::set_universal_fee_currency_order_list())]
		pub fn set_universal_fee_currency_order_list(
			origin: OriginFor<T>,
			default_list: BoundedVec<CurrencyIdOf<T>, T::MaxFeeCurrencyOrderListLen>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			ensure!(default_list.len() > 0 as usize, Error::<T>::WrongListLength);

			UniversalFeeCurrencyOrderList::<T>::set(default_list);

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	#[transactional]
	fn handle_fee() -> DispatchResult {
		let fee_receiver = Self::get_fee_receiver(ExtraFeeName::StatemineTransfer);
		let fee_receiver_balance =
			T::MultiCurrency::free_balance(T::RelaychainCurrencyId::get(), &fee_receiver);
		if fee_receiver_balance >= T::MinAssetHubExecutionFee::get() {
			T::MultiCurrency::withdraw(
				T::RelaychainCurrencyId::get(),
				&fee_receiver,
				fee_receiver_balance,
			)?;

			let asset: Asset = Asset {
				id: AssetId(Location::here()),
				fun: Fungible(UniqueSaturatedInto::<u128>::unique_saturated_into(
					fee_receiver_balance,
				)),
			};

			let remote_call =
				RelaychainCall::<BalanceOf<T>, AccountIdOf<T>, BlockNumberFor<T>>::XcmPallet(
					PolkadotXcmCall::LimitedTeleportAssets(
						Box::new(Location::new(0, [Parachain(parachains::Statemine::ID)]).into()),
						Box::new(
							Location::new(
								0,
								[AccountId32 {
									network: None,
									id: Sibling::from(T::ParachainId::get())
										.into_account_truncating(),
								}],
							)
							.into(),
						),
						Box::new(asset.into()),
						0,
						Unlimited,
					),
				)
				.encode()
				.into();

			let (require_weight_at_most, xcm_fee) =
				T::XcmWeightAndFeeHandler::get_operation_weight_and_fee(
					T::RelaychainCurrencyId::get(),
					XcmOperationType::TeleportAssets,
				)
				.ok_or(Error::<T>::WeightAndFeeNotExist)?;

			let fee: Asset = Asset {
				id: AssetId(Location::here()),
				fun: Fungible(UniqueSaturatedInto::<u128>::unique_saturated_into(xcm_fee)),
			};

			let remote_xcm = Xcm(vec![
				WithdrawAsset(fee.clone().into()),
				BuyExecution { fees: fee.clone(), weight_limit: Unlimited },
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most,
					call: remote_call,
				},
				DepositAsset {
					assets: All.into(),
					beneficiary: Location::new(0, [Parachain(T::ParachainId::get().into())]),
				},
			]);
			let (ticket, _) =
				T::XcmRouter::validate(&mut Some(Location::parent()), &mut Some(remote_xcm))
					.map_err(|_| Error::<T>::UnweighableMessage)?;
			T::XcmRouter::deliver(ticket).map_err(|_| Error::<T>::XcmExecutionFailed)?;

			Self::deposit_event(Event::TransferTo {
				from: fee_receiver,
				target_chain: TargetChain::AssetHub,
				amount: fee_receiver_balance,
			});
		}

		let fee_receiver = Self::get_fee_receiver(ExtraFeeName::VoteVtoken);
		let fee_receiver_balance =
			T::MultiCurrency::free_balance(T::RelaychainCurrencyId::get(), &fee_receiver);
		if fee_receiver_balance >= T::MinRelaychainExecutionFee::get() {
			T::MultiCurrency::withdraw(
				T::RelaychainCurrencyId::get(),
				&fee_receiver,
				fee_receiver_balance,
			)?;

			Self::deposit_event(Event::TransferTo {
				from: fee_receiver,
				target_chain: TargetChain::RelayChain,
				amount: fee_receiver_balance,
			});
		}

		Ok(())
	}

	fn get_fee_receiver(extra_fee_name: ExtraFeeName) -> T::AccountId {
		match extra_fee_name {
			ExtraFeeName::SalpContribute |
			ExtraFeeName::VoteVtoken |
			ExtraFeeName::VoteRemoveDelegatorVote => T::PalletId::get().into_sub_account_truncating(0u64),
			ExtraFeeName::StatemineTransfer | ExtraFeeName::EthereumTransfer =>
				T::PalletId::get().into_sub_account_truncating(1u64),
			ExtraFeeName::NoExtraFee => T::TreasuryAccount::get(),
		}
	}

	/// Get user fee charge assets order
	fn inner_get_user_fee_charge_order_list(account_id: &T::AccountId) -> Vec<CurrencyIdOf<T>> {
		let mut order_list: Vec<CurrencyIdOf<T>> = Vec::new();
		// Get user default fee currency
		if let Some(user_default_fee_currency) = UserDefaultFeeCurrency::<T>::get(&account_id) {
			order_list.push(user_default_fee_currency);
		};

		// Get universal fee currency order list
		let mut universal_fee_currency_order_list: Vec<CurrencyIdOf<T>> =
			UniversalFeeCurrencyOrderList::<T>::get().into_iter().collect();

		// Concat user default fee currency and universal fee currency order list
		order_list.append(&mut universal_fee_currency_order_list);

		order_list
	}

	fn find_out_fee_currency_and_amount(
		who: &T::AccountId,
		fee: PalletBalanceOf<T>,
	) -> Result<Option<(CurrencyIdOf<T>, PalletBalanceOf<T>, PalletBalanceOf<T>)>, Error<T>> {
		// get the user defined fee charge order list.
		let user_fee_charge_order_list = Self::inner_get_user_fee_charge_order_list(who);

		// charge the fee by the order of the above order list.
		// first to check whether the user has the asset. If no, pass it. If yes, try to make
		// transaction in the DEX in exchange for BNC
		for currency_id in user_fee_charge_order_list {
			// If it is mainnet currency
			if currency_id == BNC {
				// check native balance if is enough
				if T::MultiCurrency::ensure_can_withdraw(currency_id, who, fee).is_ok() {
					// currency, amount_in, amount_out
					return Ok(Some((currency_id, fee, fee)));
				}
			} else {
				// If it is other assets, go to exchange fee amount.
				let native_asset_id =
					Self::get_currency_asset_id(BNC).map_err(|_| Error::<T>::ConversionError)?;

				let amount_out: AssetBalance = fee.saturated_into();

				let asset_id = Self::get_currency_asset_id(currency_id)
					.map_err(|_| Error::<T>::ConversionError)?;

				let path = vec![asset_id, native_asset_id];
				// see if path exists, if not, continue.
				// query for amount in
				if let Ok(amounts) = T::DexOperator::get_amount_in_by_path(amount_out, &path) {
					// make sure the user has enough free token balance that can be charged.
					let amount_in = PalletBalanceOf::<T>::saturated_from(amounts[0]);
					if T::MultiCurrency::ensure_can_withdraw(currency_id, who, amount_in).is_ok() {
						// currency, amount_in, amount_out
						return Ok(Some((
							currency_id,
							amount_in,
							PalletBalanceOf::<T>::saturated_from(amount_out),
						)));
					}
				}
			}
		}
		Ok(None)
	}

	/// Make sure there are enough BNC to be deducted if the user has assets in other form of tokens
	/// rather than BNC.
	fn ensure_can_charge_fee(
		who: &T::AccountId,
		fee: PalletBalanceOf<T>,
		_reason: WithdrawReasons,
	) -> Result<PalletBalanceOf<T>, DispatchError> {
		let result_option = Self::find_out_fee_currency_and_amount(who, fee)
			.map_err(|_| DispatchError::Other("Fee calculation Error."))?;

		if let Some((currency_id, amount_in, amount_out)) = result_option {
			if currency_id != BNC {
				let native_asset_id = Self::get_currency_asset_id(BNC)?;
				let asset_id = Self::get_currency_asset_id(currency_id)?;
				let path = vec![asset_id, native_asset_id];

				T::DexOperator::inner_swap_assets_for_exact_assets(
					who,
					amount_out.saturated_into(),
					amount_in.saturated_into(),
					&path,
					who,
				)?;

				Self::deposit_event(Event::FlexibleFeeExchanged {
					transaction_fee_currency: currency_id,
					transaction_fee_amount: PalletBalanceOf::<T>::saturated_from(amount_in),
				});
			}
		}

		Ok(fee)
	}

	/// This function is for runtime-api to call
	pub fn cal_fee_token_and_amount(
		who: &T::AccountId,
		fee: PalletBalanceOf<T>,
		utx: &CallOf<T>,
	) -> Result<(CurrencyIdOf<T>, PalletBalanceOf<T>), Error<T>> {
		let total_fee_info = Self::get_extrinsic_and_extra_fee_total(utx, fee)?;
		let (currency_id, amount_in, _amount_out) =
			Self::find_out_fee_currency_and_amount(who, total_fee_info.0)
				.map_err(|_| Error::<T>::DexFailedToGetAmountInByPath)?
				.ok_or(Error::<T>::DexFailedToGetAmountInByPath)?;

		Ok((currency_id, amount_in))
	}

	pub fn get_extrinsic_and_extra_fee_total(
		call: &CallOf<T>,
		fee: PalletBalanceOf<T>,
	) -> Result<(PalletBalanceOf<T>, PalletBalanceOf<T>, PalletBalanceOf<T>, Vec<AssetId>), Error<T>>
	{
		let mut total_fee = fee;

		let native_asset_id = Self::get_currency_asset_id(BNC)?;
		let mut path = vec![native_asset_id, native_asset_id];

		// See if the this RuntimeCall needs to pay extra fee
		let fee_info = T::ExtraFeeMatcher::get_fee_info(call);
		if fee_info.extra_fee_name != ExtraFeeName::NoExtraFee {
			// if the fee_info.extra_fee_name is not NoExtraFee, it means this RuntimeCall needs to
			// pay extra fee
			let operation = match fee_info.extra_fee_name {
				ExtraFeeName::SalpContribute => XcmOperationType::UmpContributeTransact,
				ExtraFeeName::StatemineTransfer => XcmOperationType::StatemineTransfer,
				ExtraFeeName::EthereumTransfer => XcmOperationType::EthereumTransfer,
				ExtraFeeName::VoteVtoken => XcmOperationType::Vote,
				ExtraFeeName::VoteRemoveDelegatorVote => XcmOperationType::RemoveVote,
				ExtraFeeName::NoExtraFee => XcmOperationType::Any,
			};

			let (_, fee_value) = T::XcmWeightAndFeeHandler::get_operation_weight_and_fee(
				fee_info.extra_fee_currency,
				operation,
			)
			.ok_or(Error::<T>::WeightAndFeeNotExist)?;

			let asset_id = Self::get_currency_asset_id(fee_info.extra_fee_currency)?;
			path = vec![native_asset_id, asset_id];

			// get the fee currency value in BNC
			let extra_fee_vec =
				T::DexOperator::get_amount_in_by_path(fee_value.saturated_into(), &path)
					.map_err(|_| Error::<T>::DexFailedToGetAmountInByPath)?;

			let extra_bnc_fee = PalletBalanceOf::<T>::saturated_from(extra_fee_vec[0]);
			total_fee = total_fee.checked_add(&extra_bnc_fee).ok_or(Error::<T>::Overflow)?;

			return Ok((total_fee, extra_bnc_fee, fee_value, path));
		} else {
			return Ok((total_fee, Zero::zero(), Zero::zero(), path));
		}
	}

	fn get_currency_asset_id(currency_id: CurrencyIdOf<T>) -> Result<AssetId, Error<T>> {
		let asset_id: AssetId =
			AssetId::try_convert_from(currency_id, T::ParachainId::get().into())
				.map_err(|_| Error::<T>::ConversionError)?;
		Ok(asset_id)
	}
}

/// Default implementation for a Currency and an OnUnbalanced handler.
impl<T> OnChargeTransaction<T> for Pallet<T>
where
	T: Config,
	T::Currency: Currency<<T as frame_system::Config>::AccountId>,
	PositiveImbalanceOf<T>: Imbalance<PalletBalanceOf<T>, Opposite = NegativeImbalanceOf<T>>,
	NegativeImbalanceOf<T>: Imbalance<PalletBalanceOf<T>, Opposite = PositiveImbalanceOf<T>>,
{
	type Balance = PalletBalanceOf<T>;
	type LiquidityInfo = Option<NegativeImbalanceOf<T>>;

	/// Withdraw the predicted fee from the transaction origin.
	///
	/// Note: The `fee` already includes the `tip`.
	fn withdraw_fee(
		who: &T::AccountId,
		call: &T::RuntimeCall,
		_info: &DispatchInfoOf<T::RuntimeCall>,
		fee: Self::Balance,
		tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		if fee.is_zero() {
			return Ok(None);
		}

		let withdraw_reason = if tip.is_zero() {
			WithdrawReasons::TRANSACTION_PAYMENT
		} else {
			WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
		};

		// See if the this RuntimeCall needs to pay extra fee
		let fee_info = T::ExtraFeeMatcher::get_fee_info(&call);
		let (total_fee, extra_bnc_fee, fee_value, path) =
			Self::get_extrinsic_and_extra_fee_total(call, fee)
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Custom(55)))?;

		// Make sure there are enough BNC(extrinsic fee + extra fee) to be deducted if the user has
		// assets in other form of tokens rather than BNC.
		Self::ensure_can_charge_fee(who, total_fee, withdraw_reason)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

		if fee_info.extra_fee_name != ExtraFeeName::NoExtraFee {
			// swap BNC for fee_currency
			T::DexOperator::inner_swap_assets_for_exact_assets(
				who,
				fee_value.saturated_into(),
				extra_bnc_fee.saturated_into(),
				&path,
				who,
			)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Custom(44)))?;

			let transaction_extra_fee_receiver = Self::get_fee_receiver(fee_info.extra_fee_name);

			T::MultiCurrency::transfer(
				fee_info.extra_fee_currency,
				who,
				&transaction_extra_fee_receiver,
				fee_value,
			)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

			// deposit extra fee deducted event
			Self::deposit_event(Event::ExtraFeeDeducted {
				operation: fee_info.extra_fee_name,
				transaction_extra_fee_currency: fee_info.extra_fee_currency,
				transaction_extra_fee_amount: fee_value,
				transaction_extra_fee_bnc_amount: extra_bnc_fee,
				transaction_extra_fee_receiver,
			});
		}

		// withdraw normal extrinsic fee
		match T::Currency::withdraw(who, fee, withdraw_reason, ExistenceRequirement::AllowDeath) {
			Ok(imbalance) => Ok(Some(imbalance)),
			Err(_) => Err(InvalidTransaction::Payment.into()),
		}
	}

	/// Hand the fee and the tip over to the `[OnUnbalanced]` implementation.
	/// Since the predicted fee might have been too high, parts of the fee may
	/// be refunded.
	///
	/// Note: The `fee` already includes the `tip`.
	fn correct_and_deposit_fee(
		who: &T::AccountId,
		_dispatch_info: &DispatchInfoOf<T::RuntimeCall>,
		_post_info: &PostDispatchInfoOf<T::RuntimeCall>,
		corrected_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), TransactionValidityError> {
		if let Some(paid) = already_withdrawn {
			// Calculate how much refund we should return
			let refund_amount = paid.peek().saturating_sub(corrected_fee);

			// refund to the the account that paid the fees. If this fails, the
			// account might have dropped below the existential balance. In
			// that case we don't refund anything.
			let refund_imbalance = T::Currency::deposit_into_existing(who, refund_amount)
				.unwrap_or_else(|_| PositiveImbalanceOf::<T>::zero());
			// merge the imbalance caused by paying the fees and refunding parts of it again.
			let adjusted_paid = paid
				.offset(refund_imbalance)
				.same()
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
			// RuntimeCall someone else to handle the imbalance (fee and tip separately)
			let imbalances = adjusted_paid.split(tip);
			T::OnUnbalanced::on_unbalanceds(
				Some(imbalances.0).into_iter().chain(Some(imbalances.1)),
			);
		}
		Ok(())
	}
}

/// Provides account's fee payment asset or default fee asset ( Native asset )
impl<T: Config> AccountFeeCurrency<T::AccountId> for Pallet<T> {
	fn get(who: &T::AccountId) -> CurrencyId {
		Pallet::<T>::get_user_default_fee_currency(who).unwrap_or_else(|| WETH)
	}
}
