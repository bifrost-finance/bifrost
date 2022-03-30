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

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::RuntimeDebug;
use scale_info::TypeInfo;
use sp_arithmetic::per_things::Percent;

/// Exchange rate of vstoken-conversion
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
pub struct VstokenConversionExchangeRate {
	pub vsbond_convert_to_vsksm: Percent,
	pub vsksm_convert_to_vsbond: Percent,
	pub vsbond_convert_to_vsdot: Percent,
	pub vsdot_convert_to_vsbond: Percent,
}

/// Exchange fee of vstoken-conversion
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
pub struct VstokenConversionExchangeFee<Balance> {
	pub vsksm_exchange_fee: Balance,
	pub vsdot_exchange_fee: Balance,
	pub vsbond_exchange_fee_of_vsksm: Balance,
	pub vsbond_exchange_fee_of_vsdot: Balance,
}
