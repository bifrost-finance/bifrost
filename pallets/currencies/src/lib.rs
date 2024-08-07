//! # Currencies Module
//!
//! ## Overview
//!
//! The currencies module provides a mixed currencies system, by configuring a
//! native currency which implements `BasicCurrencyExtended`, and a
//! multi-currency which implements `MultiCurrency`.
//!
//! It also provides an adapter, to adapt `frame_support::traits::Currency`
//! implementations into `BasicCurrencyExtended`.
//!
//! The currencies module provides functionality of both `MultiCurrencyExtended`
//! and `BasicCurrencyExtended`, via unified interfaces, and all calls would be
//! delegated to the underlying multi-currency and base currency system.
//! A native currency ID could be set by `Config::GetNativeCurrencyId`, to
//! identify the native currency.
//!
//! ### Implementations
//!
//! The currencies module provides implementations for following traits.
//!
//! - `MultiCurrency` - Abstraction over a fungible multi-currency system.
//! - `MultiCurrencyExtended` - Extended `MultiCurrency` with additional helper types and methods,
//!   like updating balance
//! by a given signed integer amount.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `transfer` - Transfer some balance to another account, in a given currency.
//! - `transfer_native_currency` - Transfer some balance to another account, in native currency set
//!   in
//! `Config::NativeCurrency`.
//! - `update_balance` - Update balance by signed integer amount, in a given currency, root origin
//!   required.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{
	pallet_prelude::*,
	traits::{
		tokens::{
			fungible, fungibles, DepositConsequence, Fortitude, Precision, Preservation,
			Provenance, WithdrawConsequence,
		},
		Currency as PalletCurrency, ExistenceRequirement, Get, Imbalance,
		LockableCurrency as PalletLockableCurrency,
		NamedReservableCurrency as PalletNamedReservableCurrency,
		ReservableCurrency as PalletReservableCurrency, WithdrawReasons,
	},
};
use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
use orml_traits::{
	arithmetic::{Signed, SimpleArithmetic},
	currency::TransferAll,
	BalanceStatus, BasicCurrency, BasicCurrencyExtended, BasicLockableCurrency,
	BasicReservableCurrency, LockIdentifier, MultiCurrency, MultiCurrencyExtended,
	MultiLockableCurrency, MultiReservableCurrency, NamedBasicReservableCurrency,
	NamedMultiReservableCurrency,
};
use orml_utilities::with_transaction_result;
use parity_scale_codec::Codec;
use sp_runtime::{
	traits::{CheckedSub, MaybeSerializeDeserialize, StaticLookup, Zero},
	DispatchError, DispatchResult,
};
use sp_std::{fmt::Debug, marker, result};

mod mock;
mod tests;
mod weights;

pub use module::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod module {
	use super::*;

	pub(crate) type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;
	pub(crate) type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
		<T as frame_system::Config>::AccountId,
	>>::CurrencyId;
	pub(crate) type AmountOf<T> = <<T as Config>::MultiCurrency as MultiCurrencyExtended<
		<T as frame_system::Config>::AccountId,
	>>::Amount;
	pub(crate) type ReserveIdentifierOf<T> =
		<<T as Config>::MultiCurrency as NamedMultiReservableCurrency<
			<T as frame_system::Config>::AccountId,
		>>::ReserveIdentifier;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type MultiCurrency: TransferAll<Self::AccountId>
			+ MultiCurrencyExtended<Self::AccountId>
			+ MultiLockableCurrency<Self::AccountId>
			+ MultiReservableCurrency<Self::AccountId>
			+ NamedMultiReservableCurrency<Self::AccountId>
			+ fungibles::Inspect<
				Self::AccountId,
				AssetId = CurrencyIdOf<Self>,
				Balance = BalanceOf<Self>,
			> + fungibles::Mutate<
				Self::AccountId,
				AssetId = CurrencyIdOf<Self>,
				Balance = BalanceOf<Self>,
			>;

		type NativeCurrency: BasicCurrencyExtended<
				Self::AccountId,
				Balance = BalanceOf<Self>,
				Amount = AmountOf<Self>,
			> + BasicLockableCurrency<Self::AccountId, Balance = BalanceOf<Self>>
			+ BasicReservableCurrency<Self::AccountId, Balance = BalanceOf<Self>>
			+ NamedBasicReservableCurrency<
				Self::AccountId,
				ReserveIdentifierOf<Self>,
				Balance = BalanceOf<Self>,
			> + fungible::Inspect<Self::AccountId, Balance = BalanceOf<Self>>
			+ fungible::Mutate<Self::AccountId, Balance = BalanceOf<Self>>;

		#[pallet::constant]
		type GetNativeCurrencyId: Get<CurrencyIdOf<Self>>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Unable to convert the Amount type into Balance.
		AmountIntoBalanceFailed,
		/// Balance is too low.
		BalanceTooLow,
		/// Deposit result is not expected
		DepositFailed,
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transfer some balance to another account under `currency_id`.
		///
		/// The dispatch origin for this call must be `Signed` by the
		/// transactor.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::transfer_non_native_currency())]
		pub fn transfer(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			currency_id: CurrencyIdOf<T>,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			<Self as MultiCurrency<T::AccountId>>::transfer(currency_id, &from, &to, amount)
		}

		/// Transfer some native currency to another account.
		///
		/// The dispatch origin for this call must be `Signed` by the
		/// transactor.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::transfer_native_currency())]
		pub fn transfer_native_currency(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			T::NativeCurrency::transfer(&from, &to, amount)
		}

		/// update amount of account `who` under `currency_id`.
		///
		/// The dispatch origin of this call must be _Root_.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::update_balance_non_native_currency())]
		pub fn update_balance(
			origin: OriginFor<T>,
			who: <T::Lookup as StaticLookup>::Source,
			currency_id: CurrencyIdOf<T>,
			amount: AmountOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let dest = T::Lookup::lookup(who)?;
			<Self as MultiCurrencyExtended<T::AccountId>>::update_balance(
				currency_id,
				&dest,
				amount,
			)
		}
	}
}

impl<T: Config> MultiCurrency<T::AccountId> for Pallet<T> {
	type CurrencyId = CurrencyIdOf<T>;
	type Balance = BalanceOf<T>;

	fn minimum_balance(currency_id: Self::CurrencyId) -> Self::Balance {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::minimum_balance()
		} else {
			T::MultiCurrency::minimum_balance(currency_id)
		}
	}

	fn total_issuance(currency_id: Self::CurrencyId) -> Self::Balance {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::total_issuance()
		} else {
			T::MultiCurrency::total_issuance(currency_id)
		}
	}

	fn total_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::total_balance(who)
		} else {
			T::MultiCurrency::total_balance(currency_id, who)
		}
	}

	fn free_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::free_balance(who)
		} else {
			T::MultiCurrency::free_balance(currency_id, who)
		}
	}

	fn ensure_can_withdraw(
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::ensure_can_withdraw(who, amount)
		} else {
			T::MultiCurrency::ensure_can_withdraw(currency_id, who, amount)
		}
	}

	fn transfer(
		currency_id: Self::CurrencyId,
		from: &T::AccountId,
		to: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if amount.is_zero() || from == to {
			return Ok(());
		}
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::transfer(from, to, amount)
		} else {
			T::MultiCurrency::transfer(currency_id, from, to, amount)
		}
	}

	fn deposit(
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::deposit(who, amount)
		} else {
			T::MultiCurrency::deposit(currency_id, who, amount)
		}
	}

	fn withdraw(
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::withdraw(who, amount)
		} else {
			T::MultiCurrency::withdraw(currency_id, who, amount)
		}
	}

	fn can_slash(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> bool {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::can_slash(who, amount)
		} else {
			T::MultiCurrency::can_slash(currency_id, who, amount)
		}
	}

	fn slash(
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> Self::Balance {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::slash(who, amount)
		} else {
			T::MultiCurrency::slash(currency_id, who, amount)
		}
	}
}

impl<T: Config> MultiCurrencyExtended<T::AccountId> for Pallet<T> {
	type Amount = AmountOf<T>;

	fn update_balance(
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		by_amount: Self::Amount,
	) -> DispatchResult {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::update_balance(who, by_amount)
		} else {
			T::MultiCurrency::update_balance(currency_id, who, by_amount)
		}
	}
}

impl<T: Config> MultiLockableCurrency<T::AccountId> for Pallet<T> {
	type Moment = BlockNumberFor<T>;

	fn set_lock(
		lock_id: LockIdentifier,
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::set_lock(lock_id, who, amount)
		} else {
			T::MultiCurrency::set_lock(lock_id, currency_id, who, amount)
		}
	}

	fn extend_lock(
		lock_id: LockIdentifier,
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::extend_lock(lock_id, who, amount)
		} else {
			T::MultiCurrency::extend_lock(lock_id, currency_id, who, amount)
		}
	}

	fn remove_lock(
		lock_id: LockIdentifier,
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
	) -> DispatchResult {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::remove_lock(lock_id, who)
		} else {
			T::MultiCurrency::remove_lock(lock_id, currency_id, who)
		}
	}
}

impl<T: Config> MultiReservableCurrency<T::AccountId> for Pallet<T> {
	fn can_reserve(
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> bool {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::can_reserve(who, value)
		} else {
			T::MultiCurrency::can_reserve(currency_id, who, value)
		}
	}

	fn slash_reserved(
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> Self::Balance {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::slash_reserved(who, value)
		} else {
			T::MultiCurrency::slash_reserved(currency_id, who, value)
		}
	}

	fn reserved_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::reserved_balance(who)
		} else {
			T::MultiCurrency::reserved_balance(currency_id, who)
		}
	}

	fn reserve(
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> DispatchResult {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::reserve(who, value)
		} else {
			T::MultiCurrency::reserve(currency_id, who, value)
		}
	}

	fn unreserve(
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> Self::Balance {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::unreserve(who, value)
		} else {
			T::MultiCurrency::unreserve(currency_id, who, value)
		}
	}

	fn repatriate_reserved(
		currency_id: Self::CurrencyId,
		slashed: &T::AccountId,
		beneficiary: &T::AccountId,
		value: Self::Balance,
		status: BalanceStatus,
	) -> result::Result<Self::Balance, DispatchError> {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::repatriate_reserved(slashed, beneficiary, value, status)
		} else {
			T::MultiCurrency::repatriate_reserved(currency_id, slashed, beneficiary, value, status)
		}
	}
}

impl<T: Config> NamedMultiReservableCurrency<T::AccountId> for Pallet<T> {
	type ReserveIdentifier = ReserveIdentifierOf<T>;

	fn slash_reserved_named(
		id: &Self::ReserveIdentifier,
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> Self::Balance {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::slash_reserved_named(id, who, value)
		} else {
			T::MultiCurrency::slash_reserved_named(id, currency_id, who, value)
		}
	}

	fn reserved_balance_named(
		id: &Self::ReserveIdentifier,
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
	) -> Self::Balance {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::reserved_balance_named(id, who)
		} else {
			T::MultiCurrency::reserved_balance_named(id, currency_id, who)
		}
	}

	fn reserve_named(
		id: &Self::ReserveIdentifier,
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> DispatchResult {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::reserve_named(id, who, value)
		} else {
			T::MultiCurrency::reserve_named(id, currency_id, who, value)
		}
	}

	fn unreserve_named(
		id: &Self::ReserveIdentifier,
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> Self::Balance {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::unreserve_named(id, who, value)
		} else {
			T::MultiCurrency::unreserve_named(id, currency_id, who, value)
		}
	}

	fn repatriate_reserved_named(
		id: &Self::ReserveIdentifier,
		currency_id: Self::CurrencyId,
		slashed: &T::AccountId,
		beneficiary: &T::AccountId,
		value: Self::Balance,
		status: BalanceStatus,
	) -> result::Result<Self::Balance, DispatchError> {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::repatriate_reserved_named(id, slashed, beneficiary, value, status)
		} else {
			T::MultiCurrency::repatriate_reserved_named(
				id,
				currency_id,
				slashed,
				beneficiary,
				value,
				status,
			)
		}
	}
}

/// impl fungiles for Pallet<T>
impl<T: Config> fungibles::Inspect<T::AccountId> for Pallet<T> {
	type AssetId = CurrencyIdOf<T>;
	type Balance = BalanceOf<T>;

	fn total_issuance(asset_id: Self::AssetId) -> Self::Balance {
		match asset_id {
			id if id == T::GetNativeCurrencyId::get() =>
				<T::NativeCurrency as fungible::Inspect<_>>::total_issuance(),
			_ => <T::MultiCurrency as fungibles::Inspect<_>>::total_issuance(asset_id),
		}
	}

	fn minimum_balance(asset_id: Self::AssetId) -> Self::Balance {
		match asset_id {
			id if id == T::GetNativeCurrencyId::get() =>
				<T::NativeCurrency as fungible::Inspect<_>>::minimum_balance(),
			_ => <T::MultiCurrency as fungibles::Inspect<_>>::minimum_balance(asset_id),
		}
	}

	fn balance(asset_id: Self::AssetId, who: &T::AccountId) -> Self::Balance {
		match asset_id {
			id if id == T::GetNativeCurrencyId::get() =>
				<T::NativeCurrency as fungible::Inspect<_>>::balance(who),
			_ => <T::MultiCurrency as fungibles::Inspect<_>>::balance(asset_id, who),
		}
	}

	fn total_balance(asset_id: Self::AssetId, who: &T::AccountId) -> Self::Balance {
		match asset_id {
			id if id == T::GetNativeCurrencyId::get() =>
				<T::NativeCurrency as fungible::Inspect<_>>::total_balance(who),
			_ => <T::MultiCurrency as fungibles::Inspect<_>>::total_balance(asset_id, who),
		}
	}

	fn reducible_balance(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		preservation: Preservation,
		force: Fortitude,
	) -> Self::Balance {
		match asset_id {
			id if id == T::GetNativeCurrencyId::get() => <T::NativeCurrency as fungible::Inspect<
				_,
			>>::reducible_balance(
				who, preservation, force
			),
			_ => <T::MultiCurrency as fungibles::Inspect<_>>::reducible_balance(
				asset_id,
				who,
				preservation,
				force,
			),
		}
	}

	fn can_deposit(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
		provenance: Provenance,
	) -> DepositConsequence {
		match asset_id {
			id if id == T::GetNativeCurrencyId::get() =>
				<T::NativeCurrency as fungible::Inspect<_>>::can_deposit(who, amount, provenance),
			_ => <T::MultiCurrency as fungibles::Inspect<_>>::can_deposit(
				asset_id, who, amount, provenance,
			),
		}
	}

	fn can_withdraw(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> WithdrawConsequence<Self::Balance> {
		match asset_id {
			id if id == T::GetNativeCurrencyId::get() =>
				<T::NativeCurrency as fungible::Inspect<_>>::can_withdraw(who, amount),
			_ => <T::MultiCurrency as fungibles::Inspect<_>>::can_withdraw(asset_id, who, amount),
		}
	}

	fn asset_exists(asset_id: Self::AssetId) -> bool {
		match asset_id {
			id if id == T::GetNativeCurrencyId::get() => true,
			_ => <T::MultiCurrency as fungibles::Inspect<_>>::asset_exists(asset_id),
		}
	}
}

impl<T: Config> fungibles::Unbalanced<T::AccountId> for Pallet<T> {
	fn handle_dust(dust: fungibles::Dust<T::AccountId, Self>) {
		let asset = dust.0;
		if asset == T::GetNativeCurrencyId::get() {
			<T::NativeCurrency as fungible::Unbalanced<T::AccountId>>::handle_dust(fungible::Dust(
				dust.1.into(),
			))
		} else {
			<T::MultiCurrency as fungibles::Unbalanced<T::AccountId>>::handle_dust(fungibles::Dust(
				dust.0.into(),
				dust.1.into(),
			))
		}
	}

	fn write_balance(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> Result<Option<Self::Balance>, DispatchError> {
		match asset_id {
			id if id == T::GetNativeCurrencyId::get() =>
				<T::NativeCurrency as fungible::Unbalanced<_>>::write_balance(who, amount),
			_ =>
				<T::MultiCurrency as fungibles::Unbalanced<_>>::write_balance(asset_id, who, amount),
		}
	}

	fn set_total_issuance(asset_id: Self::AssetId, amount: Self::Balance) {
		match asset_id {
			id if id == T::GetNativeCurrencyId::get() =>
				<T::NativeCurrency as fungible::Unbalanced<_>>::set_total_issuance(amount),
			_ =>
				<T::MultiCurrency as fungibles::Unbalanced<_>>::set_total_issuance(asset_id, amount),
		}
	}
}

impl<T: Config> fungibles::Mutate<T::AccountId> for Pallet<T> {
	fn mint_into(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> Result<Self::Balance, DispatchError> {
		match asset_id {
			id if id == T::GetNativeCurrencyId::get() =>
				<T::NativeCurrency as fungible::Mutate<_>>::mint_into(who, amount),
			_ => <T::MultiCurrency as fungibles::Mutate<_>>::mint_into(asset_id, who, amount),
		}
	}

	fn burn_from(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
		preservation: Preservation,
		precision: Precision,
		fortitude: Fortitude,
	) -> Result<Self::Balance, DispatchError> {
		match asset_id {
			id if id == T::GetNativeCurrencyId::get() =>
				<T::NativeCurrency as fungible::Mutate<_>>::burn_from(
					who,
					amount,
					preservation,
					precision,
					fortitude,
				),
			_ => <T::MultiCurrency as fungibles::Mutate<_>>::burn_from(
				asset_id,
				who,
				amount,
				preservation,
				precision,
				fortitude,
			),
		}
	}

	fn transfer(
		asset_id: Self::AssetId,
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: Self::Balance,
		preservation: Preservation,
	) -> Result<Self::Balance, DispatchError> {
		match asset_id {
			id if id == T::GetNativeCurrencyId::get() => <T::NativeCurrency as fungible::Mutate<
				_,
			>>::transfer(
				source, dest, amount, preservation
			),
			_ => <T::MultiCurrency as fungibles::Mutate<_>>::transfer(
				asset_id,
				source,
				dest,
				amount,
				preservation,
			),
		}
	}
}

pub struct Currency<T, GetCurrencyId>(marker::PhantomData<T>, marker::PhantomData<GetCurrencyId>);

impl<T, GetCurrencyId> BasicCurrency<T::AccountId> for Currency<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyIdOf<T>>,
{
	type Balance = BalanceOf<T>;

	fn minimum_balance() -> Self::Balance {
		<Pallet<T>>::minimum_balance(GetCurrencyId::get())
	}

	fn total_issuance() -> Self::Balance {
		<Pallet<T>>::total_issuance(GetCurrencyId::get())
	}

	fn total_balance(who: &T::AccountId) -> Self::Balance {
		<Pallet<T>>::total_balance(GetCurrencyId::get(), who)
	}

	fn free_balance(who: &T::AccountId) -> Self::Balance {
		<Pallet<T>>::free_balance(GetCurrencyId::get(), who)
	}

	fn ensure_can_withdraw(who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		<Pallet<T>>::ensure_can_withdraw(GetCurrencyId::get(), who, amount)
	}

	fn transfer(from: &T::AccountId, to: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		<Pallet<T> as MultiCurrency<T::AccountId>>::transfer(GetCurrencyId::get(), from, to, amount)
	}

	fn deposit(who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		<Pallet<T>>::deposit(GetCurrencyId::get(), who, amount)
	}

	fn withdraw(who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		<Pallet<T>>::withdraw(GetCurrencyId::get(), who, amount)
	}

	fn can_slash(who: &T::AccountId, amount: Self::Balance) -> bool {
		<Pallet<T>>::can_slash(GetCurrencyId::get(), who, amount)
	}

	fn slash(who: &T::AccountId, amount: Self::Balance) -> Self::Balance {
		<Pallet<T>>::slash(GetCurrencyId::get(), who, amount)
	}
}

impl<T, GetCurrencyId> BasicCurrencyExtended<T::AccountId> for Currency<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyIdOf<T>>,
{
	type Amount = AmountOf<T>;

	fn update_balance(who: &T::AccountId, by_amount: Self::Amount) -> DispatchResult {
		<Pallet<T> as MultiCurrencyExtended<T::AccountId>>::update_balance(
			GetCurrencyId::get(),
			who,
			by_amount,
		)
	}
}

impl<T, GetCurrencyId> BasicLockableCurrency<T::AccountId> for Currency<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyIdOf<T>>,
{
	type Moment = BlockNumberFor<T>;

	fn set_lock(
		lock_id: LockIdentifier,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		<Pallet<T> as MultiLockableCurrency<T::AccountId>>::set_lock(
			lock_id,
			GetCurrencyId::get(),
			who,
			amount,
		)
	}

	fn extend_lock(
		lock_id: LockIdentifier,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		<Pallet<T> as MultiLockableCurrency<T::AccountId>>::extend_lock(
			lock_id,
			GetCurrencyId::get(),
			who,
			amount,
		)
	}

	fn remove_lock(lock_id: LockIdentifier, who: &T::AccountId) -> DispatchResult {
		<Pallet<T> as MultiLockableCurrency<T::AccountId>>::remove_lock(
			lock_id,
			GetCurrencyId::get(),
			who,
		)
	}
}

impl<T, GetCurrencyId> BasicReservableCurrency<T::AccountId> for Currency<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyIdOf<T>>,
{
	fn can_reserve(who: &T::AccountId, value: Self::Balance) -> bool {
		<Pallet<T> as MultiReservableCurrency<T::AccountId>>::can_reserve(
			GetCurrencyId::get(),
			who,
			value,
		)
	}

	fn slash_reserved(who: &T::AccountId, value: Self::Balance) -> Self::Balance {
		<Pallet<T> as MultiReservableCurrency<T::AccountId>>::slash_reserved(
			GetCurrencyId::get(),
			who,
			value,
		)
	}

	fn reserved_balance(who: &T::AccountId) -> Self::Balance {
		<Pallet<T> as MultiReservableCurrency<T::AccountId>>::reserved_balance(
			GetCurrencyId::get(),
			who,
		)
	}

	fn reserve(who: &T::AccountId, value: Self::Balance) -> DispatchResult {
		<Pallet<T> as MultiReservableCurrency<T::AccountId>>::reserve(
			GetCurrencyId::get(),
			who,
			value,
		)
	}

	fn unreserve(who: &T::AccountId, value: Self::Balance) -> Self::Balance {
		<Pallet<T> as MultiReservableCurrency<T::AccountId>>::unreserve(
			GetCurrencyId::get(),
			who,
			value,
		)
	}

	fn repatriate_reserved(
		slashed: &T::AccountId,
		beneficiary: &T::AccountId,
		value: Self::Balance,
		status: BalanceStatus,
	) -> result::Result<Self::Balance, DispatchError> {
		<Pallet<T> as MultiReservableCurrency<T::AccountId>>::repatriate_reserved(
			GetCurrencyId::get(),
			slashed,
			beneficiary,
			value,
			status,
		)
	}
}

impl<T, GetCurrencyId> NamedBasicReservableCurrency<T::AccountId, ReserveIdentifierOf<T>>
	for Currency<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<CurrencyIdOf<T>>,
{
	fn slash_reserved_named(
		id: &ReserveIdentifierOf<T>,
		who: &T::AccountId,
		value: Self::Balance,
	) -> Self::Balance {
		<Pallet<T> as NamedMultiReservableCurrency<T::AccountId>>::slash_reserved_named(
			id,
			GetCurrencyId::get(),
			who,
			value,
		)
	}

	fn reserved_balance_named(id: &ReserveIdentifierOf<T>, who: &T::AccountId) -> Self::Balance {
		<Pallet<T> as NamedMultiReservableCurrency<T::AccountId>>::reserved_balance_named(
			id,
			GetCurrencyId::get(),
			who,
		)
	}

	fn reserve_named(
		id: &ReserveIdentifierOf<T>,
		who: &T::AccountId,
		value: Self::Balance,
	) -> DispatchResult {
		<Pallet<T> as NamedMultiReservableCurrency<T::AccountId>>::reserve_named(
			id,
			GetCurrencyId::get(),
			who,
			value,
		)
	}

	fn unreserve_named(
		id: &ReserveIdentifierOf<T>,
		who: &T::AccountId,
		value: Self::Balance,
	) -> Self::Balance {
		<Pallet<T> as NamedMultiReservableCurrency<T::AccountId>>::unreserve_named(
			id,
			GetCurrencyId::get(),
			who,
			value,
		)
	}

	fn repatriate_reserved_named(
		id: &ReserveIdentifierOf<T>,
		slashed: &T::AccountId,
		beneficiary: &T::AccountId,
		value: Self::Balance,
		status: BalanceStatus,
	) -> result::Result<Self::Balance, DispatchError> {
		<Pallet<T> as NamedMultiReservableCurrency<T::AccountId>>::repatriate_reserved_named(
			id,
			GetCurrencyId::get(),
			slashed,
			beneficiary,
			value,
			status,
		)
	}
}

pub type NativeCurrencyOf<T> = Currency<T, <T as Config>::GetNativeCurrencyId>;

/// Adapt other currency traits implementation to `BasicCurrency`.
pub struct BasicCurrencyAdapter<T, Currency, Amount, Moment>(
	marker::PhantomData<(T, Currency, Amount, Moment)>,
);

type PalletBalanceOf<A, Currency> = <Currency as PalletCurrency<A>>::Balance;

// Adapt `frame_support::traits::Currency`
impl<T, AccountId, Currency, Amount, Moment> BasicCurrency<AccountId>
	for BasicCurrencyAdapter<T, Currency, Amount, Moment>
where
	Currency: PalletCurrency<AccountId>,
	T: Config,
{
	type Balance = PalletBalanceOf<AccountId, Currency>;

	fn minimum_balance() -> Self::Balance {
		Currency::minimum_balance()
	}

	fn total_issuance() -> Self::Balance {
		Currency::total_issuance()
	}

	fn total_balance(who: &AccountId) -> Self::Balance {
		Currency::total_balance(who)
	}

	fn free_balance(who: &AccountId) -> Self::Balance {
		Currency::free_balance(who)
	}

	fn ensure_can_withdraw(who: &AccountId, amount: Self::Balance) -> DispatchResult {
		let new_balance =
			Self::free_balance(who).checked_sub(&amount).ok_or(Error::<T>::BalanceTooLow)?;

		Currency::ensure_can_withdraw(who, amount, WithdrawReasons::all(), new_balance)
	}

	fn transfer(from: &AccountId, to: &AccountId, amount: Self::Balance) -> DispatchResult {
		Currency::transfer(from, to, amount, ExistenceRequirement::AllowDeath)
	}

	fn deposit(who: &AccountId, amount: Self::Balance) -> DispatchResult {
		if !amount.is_zero() {
			let deposit_result = Currency::deposit_creating(who, amount);
			let actual_deposit = deposit_result.peek();
			ensure!(actual_deposit == amount, Error::<T>::DepositFailed);
		}
		Ok(())
	}

	fn withdraw(who: &AccountId, amount: Self::Balance) -> DispatchResult {
		Currency::withdraw(who, amount, WithdrawReasons::all(), ExistenceRequirement::AllowDeath)
			.map(|_| ())
	}

	fn can_slash(who: &AccountId, amount: Self::Balance) -> bool {
		Currency::can_slash(who, amount)
	}

	fn slash(who: &AccountId, amount: Self::Balance) -> Self::Balance {
		let (_, gap) = Currency::slash(who, amount);
		gap
	}
}

// Adapt `frame_support::traits::Currency`
impl<T, AccountId, Currency, Amount, Moment> BasicCurrencyExtended<AccountId>
	for BasicCurrencyAdapter<T, Currency, Amount, Moment>
where
	Amount: Signed
		+ TryInto<PalletBalanceOf<AccountId, Currency>>
		+ TryFrom<PalletBalanceOf<AccountId, Currency>>
		+ SimpleArithmetic
		+ Codec
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ Default
		+ MaxEncodedLen,
	Currency: PalletCurrency<AccountId>,
	T: Config,
{
	type Amount = Amount;

	fn update_balance(who: &AccountId, by_amount: Self::Amount) -> DispatchResult {
		let by_balance =
			by_amount.abs().try_into().map_err(|_| Error::<T>::AmountIntoBalanceFailed)?;
		if by_amount.is_positive() {
			Self::deposit(who, by_balance)
		} else {
			Self::withdraw(who, by_balance)
		}
	}
}

// Adapt `frame_support::traits::LockableCurrency`
impl<T, AccountId, Currency, Amount, Moment> BasicLockableCurrency<AccountId>
	for BasicCurrencyAdapter<T, Currency, Amount, Moment>
where
	Currency: PalletLockableCurrency<AccountId>,
	T: Config,
{
	type Moment = Moment;

	fn set_lock(lock_id: LockIdentifier, who: &AccountId, amount: Self::Balance) -> DispatchResult {
		Currency::set_lock(lock_id, who, amount, WithdrawReasons::all());
		Ok(())
	}

	fn extend_lock(
		lock_id: LockIdentifier,
		who: &AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		Currency::extend_lock(lock_id, who, amount, WithdrawReasons::all());
		Ok(())
	}

	fn remove_lock(lock_id: LockIdentifier, who: &AccountId) -> DispatchResult {
		Currency::remove_lock(lock_id, who);
		Ok(())
	}
}

// Adapt `frame_support::traits::ReservableCurrency`
impl<T, AccountId, Currency, Amount, Moment> BasicReservableCurrency<AccountId>
	for BasicCurrencyAdapter<T, Currency, Amount, Moment>
where
	Currency: PalletReservableCurrency<AccountId>,
	T: Config,
{
	fn can_reserve(who: &AccountId, value: Self::Balance) -> bool {
		Currency::can_reserve(who, value)
	}

	fn slash_reserved(who: &AccountId, value: Self::Balance) -> Self::Balance {
		let (_, gap) = Currency::slash_reserved(who, value);
		gap
	}

	fn reserved_balance(who: &AccountId) -> Self::Balance {
		Currency::reserved_balance(who)
	}

	fn reserve(who: &AccountId, value: Self::Balance) -> DispatchResult {
		Currency::reserve(who, value)
	}

	fn unreserve(who: &AccountId, value: Self::Balance) -> Self::Balance {
		Currency::unreserve(who, value)
	}

	fn repatriate_reserved(
		slashed: &AccountId,
		beneficiary: &AccountId,
		value: Self::Balance,
		status: BalanceStatus,
	) -> result::Result<Self::Balance, DispatchError> {
		Currency::repatriate_reserved(slashed, beneficiary, value, status)
	}
}

// Adapt `frame_support::traits::NamedReservableCurrency`
impl<T, AccountId, Currency, Amount, Moment, ReserveIdentifier>
	NamedBasicReservableCurrency<AccountId, ReserveIdentifier>
	for BasicCurrencyAdapter<T, Currency, Amount, Moment>
where
	Currency: PalletNamedReservableCurrency<AccountId, ReserveIdentifier = ReserveIdentifier>,
	T: Config,
{
	fn slash_reserved_named(
		id: &ReserveIdentifier,
		who: &AccountId,
		value: Self::Balance,
	) -> Self::Balance {
		let (_, gap) = Currency::slash_reserved_named(id, who, value);
		gap
	}

	fn reserved_balance_named(id: &ReserveIdentifier, who: &AccountId) -> Self::Balance {
		Currency::reserved_balance_named(id, who)
	}

	fn reserve_named(
		id: &ReserveIdentifier,
		who: &AccountId,
		value: Self::Balance,
	) -> DispatchResult {
		Currency::reserve_named(id, who, value)
	}

	fn unreserve_named(
		id: &ReserveIdentifier,
		who: &AccountId,
		value: Self::Balance,
	) -> Self::Balance {
		Currency::unreserve_named(id, who, value)
	}

	fn repatriate_reserved_named(
		id: &ReserveIdentifier,
		slashed: &AccountId,
		beneficiary: &AccountId,
		value: Self::Balance,
		status: BalanceStatus,
	) -> result::Result<Self::Balance, DispatchError> {
		Currency::repatriate_reserved_named(id, slashed, beneficiary, value, status)
	}
}

/// impl fungile for Currency<T, GetCurrencyId>
type FungibleBalanceOf<A, Currency> = <Currency as fungible::Inspect<A>>::Balance;
impl<T, Currency, Amount, Moment> fungible::Inspect<T::AccountId>
	for BasicCurrencyAdapter<T, Currency, Amount, Moment>
where
	Currency: fungible::Inspect<T::AccountId>,
	T: Config,
{
	type Balance = FungibleBalanceOf<T::AccountId, Currency>;

	fn total_issuance() -> Self::Balance {
		<Currency as fungible::Inspect<_>>::total_issuance()
	}
	fn minimum_balance() -> Self::Balance {
		<Currency as fungible::Inspect<_>>::minimum_balance()
	}
	fn balance(who: &T::AccountId) -> Self::Balance {
		<Currency as fungible::Inspect<_>>::balance(who)
	}
	fn total_balance(who: &T::AccountId) -> Self::Balance {
		<Currency as fungible::Inspect<_>>::total_balance(who)
	}
	fn reducible_balance(
		who: &T::AccountId,
		preservation: Preservation,
		force: Fortitude,
	) -> Self::Balance {
		<Currency as fungible::Inspect<_>>::reducible_balance(who, preservation, force)
	}
	fn can_deposit(
		who: &T::AccountId,
		amount: Self::Balance,
		provenance: Provenance,
	) -> DepositConsequence {
		<Currency as fungible::Inspect<_>>::can_deposit(who, amount, provenance)
	}
	fn can_withdraw(
		who: &T::AccountId,
		amount: Self::Balance,
	) -> WithdrawConsequence<Self::Balance> {
		<Currency as fungible::Inspect<_>>::can_withdraw(who, amount)
	}
}

impl<T, Currency, Amount, Moment> fungible::Unbalanced<T::AccountId>
	for BasicCurrencyAdapter<T, Currency, Amount, Moment>
where
	Currency: fungible::Unbalanced<T::AccountId>,
	T: Config,
{
	fn handle_dust(_dust: fungible::Dust<T::AccountId, Self>) {
		// https://github.com/paritytech/substrate/blob/569aae5341ea0c1d10426fa1ec13a36c0b64393b/frame/support/src/traits/tokens/fungibles/regular.rs#L124
		// Note: currently the field of Dust type is private and there is no constructor for it, so
		// we can't construct a Dust value and pass it.
		// `BasicCurrencyAdapter` overwrites these functions which can be called as user-level
		// operation of fungible traits when calling these functions, it will not actually reach
		// `Unbalanced::handle_dust`.
	}

	fn write_balance(
		who: &T::AccountId,
		amount: Self::Balance,
	) -> Result<Option<Self::Balance>, DispatchError> {
		<Currency as fungible::Unbalanced<_>>::write_balance(who, amount)
	}

	fn set_total_issuance(amount: Self::Balance) {
		<Currency as fungible::Unbalanced<_>>::set_total_issuance(amount)
	}
}

impl<T, Currency, Amount, Moment> fungible::Mutate<T::AccountId>
	for BasicCurrencyAdapter<T, Currency, Amount, Moment>
where
	Currency: fungible::Mutate<T::AccountId>,
	T: Config,
{
	fn mint_into(
		who: &T::AccountId,
		amount: Self::Balance,
	) -> Result<Self::Balance, DispatchError> {
		<Currency as fungible::Mutate<_>>::mint_into(who, amount)
	}

	fn burn_from(
		who: &T::AccountId,
		amount: Self::Balance,
		preservation: Preservation,
		precision: Precision,
		fortitude: Fortitude,
	) -> Result<Self::Balance, DispatchError> {
		<Currency as fungible::Mutate<_>>::burn_from(
			who,
			amount,
			preservation,
			precision,
			fortitude,
		)
	}

	fn transfer(
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: Self::Balance,
		preservation: Preservation,
	) -> Result<Self::Balance, DispatchError> {
		<Currency as fungible::Mutate<_>>::transfer(source, dest, amount, preservation)
	}
}

impl<T: Config> TransferAll<T::AccountId> for Pallet<T> {
	fn transfer_all(source: &T::AccountId, dest: &T::AccountId) -> DispatchResult {
		with_transaction_result(|| {
			// transfer non-native free to dest
			T::MultiCurrency::transfer_all(source, dest)?;

			// transfer all free to dest
			T::NativeCurrency::transfer(source, dest, T::NativeCurrency::free_balance(source))
		})
	}
}
