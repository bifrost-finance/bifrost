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
	use bifrost_runtime_common::{cent, dollar, milli};
	use frame_support::weights::{
		constants::{ExtrinsicBaseWeight, WEIGHT_PER_SECOND},
		WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
	};
	use node_primitives::{Balance, CurrencyId, TokenSymbol};
	use smallvec::smallvec;
	pub use sp_runtime::Perbill;

	pub fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * cent(CurrencyId::Native(TokenSymbol::ASG)) +
			(bytes as Balance) * 6 * cent(CurrencyId::Native(TokenSymbol::ASG))
	}

	pub struct KsmWeightToFee;
	impl WeightToFeePolynomial for KsmWeightToFee {
		type Balance = Balance;
		fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
			let p = ksm_base_tx_fee();
			let q = 10 * Balance::from(ExtrinsicBaseWeight::get());
			smallvec![WeightToFeeCoefficient {
				degree: 1,
				negative: false,
				coeff_frac: Perbill::from_rational(p % q, q),
				coeff_integer: p / q,
			}]
		}
	}

	pub struct WeightToFee;
	impl WeightToFeePolynomial for WeightToFee {
		type Balance = Balance;
		fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
			// extrinsic base weight (smallest non-zero weight) is mapped to 1/10 CENT:
			let p = base_tx_fee();
			let q = Balance::from(ExtrinsicBaseWeight::get());
			smallvec![WeightToFeeCoefficient {
				degree: 1,
				negative: false,
				coeff_frac: Perbill::from_rational(p % q, q),
				coeff_integer: p / q,
			}]
		}
	}

	fn ksm_base_tx_fee() -> Balance {
		dollar(CurrencyId::Token(TokenSymbol::KSM)) / 10_000
	}

	fn base_tx_fee() -> Balance {
		milli(CurrencyId::Native(TokenSymbol::ASG)) / 3
	}

	pub fn ksm_per_second() -> u128 {
		let base_weight = Balance::from(ExtrinsicBaseWeight::get());
		let base_tx_per_second = (WEIGHT_PER_SECOND as u128) / base_weight;
		let fee_per_second = base_tx_per_second * base_tx_fee();
		fee_per_second / 100
	}
}

/// Time.
pub mod time {}
