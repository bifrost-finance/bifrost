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

// use super::{Config, RelaychainLease, Weight};
use crate::*;
use frame_support::traits::Get;

pub fn update_relaychain_lease<T: Config>() -> Weight {
	let ksm_lease = KusamaLease::<T>::get();
	RelaychainLease::<T>::set(ksm_lease);
	T::DbWeight::get().reads(1) + T::DbWeight::get().writes(1)
}

/// Exchange rate of vstoken-conversion
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
pub struct DeprecatedVstokenConversionExchangeRate {
	pub vsbond_convert_to_vsksm: Percent,
	pub vsksm_convert_to_vsbond: Percent,
	pub vsbond_convert_to_vsdot: Percent,
	pub vsdot_convert_to_vsbond: Percent,
}

pub fn update_exchange_rate<T: Config>() -> Weight {
	ExchangeRate::<T>::translate::<DeprecatedVstokenConversionExchangeRate, _>(
		|_lease, exchange_rate| {
			let new_entry = VstokenConversionExchangeRate {
				vsbond_convert_to_vstoken: exchange_rate.vsbond_convert_to_vsksm,
				vstoken_convert_to_vsbond: exchange_rate.vsksm_convert_to_vsbond,
			};
			Some(new_entry)
		},
	);

	T::DbWeight::get().reads(1) + T::DbWeight::get().writes(1)
}

/// Exchange fee of vstoken-conversion
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
pub struct DeprecatedVstokenConversionExchangeFee<Balance> {
	pub vsksm_exchange_fee: Balance,
	pub vsdot_exchange_fee: Balance,
	pub vsbond_exchange_fee_of_vsksm: Balance,
	pub vsbond_exchange_fee_of_vsdot: Balance,
}

pub fn update_exchange_fee<T: Config>() -> Weight {
	ExchangeFee::<T>::translate::<DeprecatedVstokenConversionExchangeFee<BalanceOf<T>>, _>(
		|maybe_exchange_fee| {
			if let Some(exchange_fee) = maybe_exchange_fee {
				let new_entry = VstokenConversionExchangeFee::<BalanceOf<T>> {
					vstoken_exchange_fee: exchange_fee.vsksm_exchange_fee,
					vsbond_exchange_fee_of_vstoken: exchange_fee.vsbond_exchange_fee_of_vsksm,
				};
				Some(new_entry)
			} else {
				None
			}
		},
	);

	T::DbWeight::get().reads(1) + T::DbWeight::get().writes(1)
}
