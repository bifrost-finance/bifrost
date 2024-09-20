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

use frame_support::traits::Get;
use orml_xcm_support::UnknownAsset as UnknownAssetT;
use parity_scale_codec::FullCodec;
use sp_runtime::{
	traits::{Convert, MaybeSerializeDeserialize, SaturatedConversion},
	DispatchError,
};
use sp_std::{
	cmp::{Eq, PartialEq},
	fmt::Debug,
	marker::PhantomData,
	prelude::*,
	result,
};
use xcm::{
	v3::{Error as XcmError, Result, Weight},
	v4::Asset,
};
use xcm_builder::TakeRevenue;
use xcm_executor::{
	traits::{ConvertLocation, DropAssets, MatchesFungible, TransactAsset},
	AssetsInHolding,
};

/// Asset transaction errors.
enum Error {
	/// Failed to match fungible.
	FailedToMatchFungible,
	/// `Location` to `AccountId` Conversion failed.
	AccountIdConversionFailed,
	/// `CurrencyId` conversion failed.
	CurrencyIdConversionFailed,
}

impl From<Error> for XcmError {
	fn from(e: Error) -> Self {
		match e {
			Error::FailedToMatchFungible =>
				XcmError::FailedToTransactAsset("FailedToMatchFungible"),
			Error::AccountIdConversionFailed =>
				XcmError::FailedToTransactAsset("AccountIdConversionFailed"),
			Error::CurrencyIdConversionFailed =>
				XcmError::FailedToTransactAsset("CurrencyIdConversionFailed"),
		}
	}
}

/// Deposit errors handler for `TransactAsset` implementations. Default impl for
/// `()` returns an `XcmError::FailedToTransactAsset` error.
pub trait OnDepositFail<CurrencyId, AccountId, Balance> {
	/// Called on deposit errors with a specific `currency_id`.
	fn on_deposit_currency_fail(
		err: DispatchError,
		currency_id: CurrencyId,
		who: &AccountId,
		amount: Balance,
	) -> Result;

	/// Called on unknown asset deposit errors.
	fn on_deposit_unknown_asset_fail(
		err: DispatchError,
		_asset: &Asset,
		_location: &xcm::v4::Location,
	) -> Result {
		Err(XcmError::FailedToTransactAsset(err.into()))
	}

	/// Called on `Location` to `AccountId` conversion errors.
	fn on_account_id_convert_fail(currency_id: CurrencyId, amount: Balance) -> Result;
}

/// `OnDepositFail` impl, will deposit known currencies to an alternative
/// account.
pub struct DepositToAlternative<Alternative, MultiCurrency, CurrencyId, AccountId, Balance>(
	PhantomData<(Alternative, MultiCurrency, CurrencyId, AccountId, Balance)>,
);
impl<
		Alternative: Get<AccountId>,
		MultiCurrency: orml_traits::MultiCurrency<AccountId, CurrencyId = CurrencyId, Balance = Balance>,
		AccountId: sp_std::fmt::Debug + Clone,
		CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug,
		Balance,
	> OnDepositFail<CurrencyId, AccountId, Balance>
	for DepositToAlternative<Alternative, MultiCurrency, CurrencyId, AccountId, Balance>
{
	fn on_deposit_currency_fail(
		_err: DispatchError,
		currency_id: CurrencyId,
		_who: &AccountId,
		amount: Balance,
	) -> Result {
		MultiCurrency::deposit(currency_id, &Alternative::get(), amount)
			.map_err(|e| XcmError::FailedToTransactAsset(e.into()))
	}

	fn on_account_id_convert_fail(currency_id: CurrencyId, amount: Balance) -> Result {
		MultiCurrency::deposit(currency_id, &Alternative::get(), amount)
			.map_err(|e| XcmError::FailedToTransactAsset(e.into()))
	}
}

/// The `TransactAsset` implementation, to handle `Asset` deposit/withdraw.
/// Note that teleport related functions are unimplemented.
///
/// Methods of `DepositFailureHandler` would be called on multi-currency deposit
/// errors.
///
/// If the asset is known, deposit/withdraw will be handled by `MultiCurrency`,
/// else by `UnknownAsset` if unknown.
#[allow(clippy::type_complexity)]
pub struct MultiCurrencyAdapter<
	MultiCurrency,
	UnknownAsset,
	Match,
	AccountId,
	AccountIdConvert,
	CurrencyId,
	CurrencyIdConvert,
	DepositFailureHandler,
>(
	PhantomData<(
		MultiCurrency,
		UnknownAsset,
		Match,
		AccountId,
		AccountIdConvert,
		CurrencyId,
		CurrencyIdConvert,
		DepositFailureHandler,
	)>,
);

impl<
		MultiCurrency: orml_traits::MultiCurrency<AccountId, CurrencyId = CurrencyId>,
		UnknownAsset: UnknownAssetT,
		Match: MatchesFungible<MultiCurrency::Balance>,
		AccountId: sp_std::fmt::Debug + Clone,
		AccountIdConvert: ConvertLocation<AccountId>,
		CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug,
		CurrencyIdConvert: Convert<Asset, Option<CurrencyId>>,
		DepositFailureHandler: OnDepositFail<CurrencyId, AccountId, MultiCurrency::Balance>,
	> TransactAsset
	for MultiCurrencyAdapter<
		MultiCurrency,
		UnknownAsset,
		Match,
		AccountId,
		AccountIdConvert,
		CurrencyId,
		CurrencyIdConvert,
		DepositFailureHandler,
	>
{
	fn deposit_asset(
		asset: &Asset,
		location: &xcm::v4::Location,
		_context: Option<&xcm::v4::XcmContext>,
	) -> Result {
		match (
			AccountIdConvert::convert_location(location),
			CurrencyIdConvert::convert(asset.clone()),
			Match::matches_fungible(asset),
		) {
			// known asset
			(Some(who), Some(currency_id), Some(amount)) =>
				MultiCurrency::deposit(currency_id, &who, amount).or_else(|err| {
					DepositFailureHandler::on_deposit_currency_fail(err, currency_id, &who, amount)
				}),
			// bad beneficiary
			(None, Some(currency_id), Some(amount)) =>
				DepositFailureHandler::on_account_id_convert_fail(currency_id, amount),
			// unknown asset
			_ => UnknownAsset::deposit(asset, location).or_else(|err| {
				DepositFailureHandler::on_deposit_unknown_asset_fail(err, asset, location)
			}),
		}
	}

	fn withdraw_asset(
		asset: &Asset,
		location: &xcm::v4::Location,
		_maybe_context: Option<&xcm::v4::XcmContext>,
	) -> result::Result<AssetsInHolding, XcmError> {
		UnknownAsset::withdraw(asset, location).or_else(|_| {
			let who = AccountIdConvert::convert_location(location)
				.ok_or(XcmError::from(Error::AccountIdConversionFailed))?;
			let currency_id = CurrencyIdConvert::convert(asset.clone())
				.ok_or_else(|| XcmError::from(Error::CurrencyIdConversionFailed))?;
			let amount: MultiCurrency::Balance = Match::matches_fungible(asset)
				.ok_or_else(|| XcmError::from(Error::FailedToMatchFungible))?
				.saturated_into();
			MultiCurrency::withdraw(currency_id, &who, amount)
				.map_err(|e| XcmError::FailedToTransactAsset(e.into()))
		})?;

		Ok(asset.clone().into())
	}

	fn transfer_asset(
		asset: &Asset,
		from: &xcm::v4::Location,
		to: &xcm::v4::Location,
		_context: &xcm::v4::XcmContext,
	) -> result::Result<AssetsInHolding, XcmError> {
		let from_account = AccountIdConvert::convert_location(from)
			.ok_or(XcmError::from(Error::AccountIdConversionFailed))?;
		let to_account = AccountIdConvert::convert_location(to)
			.ok_or(XcmError::from(Error::AccountIdConversionFailed))?;
		let currency_id = CurrencyIdConvert::convert(asset.clone())
			.ok_or_else(|| XcmError::from(Error::CurrencyIdConversionFailed))?;
		let amount: MultiCurrency::Balance = Match::matches_fungible(asset)
			.ok_or_else(|| XcmError::from(Error::FailedToMatchFungible))?
			.saturated_into();
		MultiCurrency::transfer(currency_id, &from_account, &to_account, amount)
			.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;

		Ok(asset.clone().into())
	}
}

pub struct BifrostDropAssets<T>(PhantomData<T>);
impl<T> DropAssets for BifrostDropAssets<T>
where
	T: TakeRevenue,
{
	fn drop_assets(
		_origin: &xcm::v4::Location,
		assets: AssetsInHolding,
		_context: &xcm::v4::XcmContext,
	) -> Weight {
		let multi_assets: Vec<Asset> = assets.into();
		for asset in multi_assets {
			T::take_revenue(asset);
		}
		// TODO #2492: Put the real weight in there.
		Weight::zero()
	}
}
