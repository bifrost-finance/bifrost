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

//! A set of constant values used in Bifrost runtime.

/// Money matters.
pub mod currency {
	use crate::Runtime;
	use bifrost_asset_registry::Config;
	use bifrost_runtime_common::{cent, milli};
	use frame_support::weights::{
		constants::{ExtrinsicBaseWeight, WEIGHT_PER_SECOND},
		WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
	};
	use node_primitives::{Balance, CurrencyId, TokenSymbol};
	use smallvec::smallvec;
	pub use sp_runtime::Perbill;

	pub const BNCS: Balance = 1_000_000_000_000;
	pub const DOLLARS: Balance = BNCS;

	pub fn deposit<Runtime: Config>(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * cent::<Runtime>(CurrencyId::Native(TokenSymbol::BNC)) +
			(bytes as Balance) * 6 * cent::<Runtime>(CurrencyId::Native(TokenSymbol::BNC))
	}

	pub struct WeightToFee;
	impl WeightToFeePolynomial for WeightToFee {
		type Balance = Balance;
		fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
			// extrinsic base weight (smallest non-zero weight) is mapped to 1/10 CENT:
			let p = base_tx_fee::<Runtime>();
			let q = Balance::from(ExtrinsicBaseWeight::get().ref_time());
			smallvec![WeightToFeeCoefficient {
				degree: 1,
				negative: false,
				coeff_frac: Perbill::from_rational(p % q, q),
				coeff_integer: p / q,
			}]
		}
	}

	fn base_tx_fee<Runtime: bifrost_asset_registry::Config>() -> Balance {
		milli::<Runtime>(CurrencyId::Native(TokenSymbol::BNC)) / 3
	}

	fn xcm_base_tx_fee<Runtime: bifrost_asset_registry::Config>() -> Balance {
		cent::<Runtime>(CurrencyId::Native(TokenSymbol::BNC)) / 10
	}

	pub fn ksm_per_second<Runtime: bifrost_asset_registry::Config>() -> u128 {
		let base_weight = Balance::from(ExtrinsicBaseWeight::get().ref_time());
		let base_tx_per_second = (WEIGHT_PER_SECOND.ref_time() as u128) / base_weight;
		let fee_per_second = base_tx_per_second * xcm_base_tx_fee::<Runtime>();
		fee_per_second / 100
	}
}

/// Time.
pub mod time {}
