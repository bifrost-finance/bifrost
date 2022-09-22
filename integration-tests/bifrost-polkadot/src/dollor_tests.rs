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

use crate::polkadot_test_net::{
	register_token2_asset, Bifrost, DECIMAL_10, DECIMAL_18, DOT_TOKEN_ID, GLMR_TOKEN_ID,
};
use bifrost_asset_registry::AssetMetadata;
use bifrost_polkadot_runtime::{AssetRegistry, Runtime};
use bifrost_runtime_common::{cent, dollar, micro, microcent, milli, millicent};
use node_primitives::CurrencyId;
use xcm_emulator::TestExt;

#[test]
fn dollar_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		register_token2_asset();
		Bifrost::execute_with(|| {
			assert_eq!(
				AssetRegistry::currency_metadatas(CurrencyId::Token2(DOT_TOKEN_ID)),
				Some(AssetMetadata {
					name: b"Polkadot DOT".to_vec(),
					symbol: b"DOT".to_vec(),
					decimals: 10u8,
					minimal_balance: 1_000_000,
				})
			);
			assert_eq!(
				AssetRegistry::currency_metadatas(CurrencyId::Token2(GLMR_TOKEN_ID)),
				Some(AssetMetadata {
					name: b"Moonbeam Native Token".to_vec(),
					symbol: b"GLMR".to_vec(),
					decimals: 18u8,
					minimal_balance: 1_000_000_000_000,
				})
			);
			assert_eq!(dollar::<Runtime>(CurrencyId::Token2(DOT_TOKEN_ID)), DECIMAL_10);
			assert_eq!(dollar::<Runtime>(CurrencyId::Token2(GLMR_TOKEN_ID)), DECIMAL_18);
		});
	})
}

#[test]
fn milli_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		register_token2_asset();
		Bifrost::execute_with(|| {
			assert_eq!(milli::<Runtime>(CurrencyId::Token2(DOT_TOKEN_ID)), DECIMAL_10 / 1000);
			assert_eq!(milli::<Runtime>(CurrencyId::Token2(GLMR_TOKEN_ID)), DECIMAL_18 / 1000);
		})
	})
}

#[test]
fn micro_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		register_token2_asset();
		Bifrost::execute_with(|| {
			assert_eq!(micro::<Runtime>(CurrencyId::Token2(DOT_TOKEN_ID)), DECIMAL_10 / 1_000_000);
			assert_eq!(micro::<Runtime>(CurrencyId::Token2(GLMR_TOKEN_ID)), DECIMAL_18 / 1_000_000);
		})
	})
}

#[test]
fn cent_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		register_token2_asset();
		Bifrost::execute_with(|| {
			assert_eq!(cent::<Runtime>(CurrencyId::Token2(DOT_TOKEN_ID)), DECIMAL_10 / 100);
			assert_eq!(cent::<Runtime>(CurrencyId::Token2(GLMR_TOKEN_ID)), DECIMAL_18 / 100);
		})
	})
}

#[test]
fn millicent_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		register_token2_asset();
		Bifrost::execute_with(|| {
			assert_eq!(
				millicent::<Runtime>(CurrencyId::Token2(DOT_TOKEN_ID)),
				DECIMAL_10 / 100_000
			);
			assert_eq!(
				millicent::<Runtime>(CurrencyId::Token2(GLMR_TOKEN_ID)),
				DECIMAL_18 / 100_000
			);
		})
	})
}

#[test]
fn microcent_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		register_token2_asset();
		Bifrost::execute_with(|| {
			assert_eq!(
				microcent::<Runtime>(CurrencyId::Token2(DOT_TOKEN_ID)),
				DECIMAL_10 / 100_000_000
			);
			assert_eq!(
				microcent::<Runtime>(CurrencyId::Token2(GLMR_TOKEN_ID)),
				DECIMAL_18 / 100_000_000
			);
		})
	})
}
