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

use frame_support::assert_ok;
use sp_runtime::{traits::AccountIdConversion, AccountId32};
use xcm_emulator::TestExt;

use crate::{
	kusama_integration_tests::{Origin, RelayCurrencyId, TimeUnit, ALICE, BLOCKS_PER_YEAR},
	kusama_test_net::{Bifrost, KusamaNet},
};
use node_primitives::{AccountId, CurrencyId, TokenSymbol, TryConvertFrom, VtokenMintingOperator};

pub const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
pub const VKSM: CurrencyId = CurrencyId::VToken(TokenSymbol::KSM);

pub fn para_account_2001() -> AccountId {
	// 5Ec4AhPV91i9yNuiWuNunPf6AQCYDhFTTA4G5QCbtqYApH9E
	let para_account_2001: AccountId =
		hex_literal::hex!["70617261d1070000000000000000000000000000000000000000000000000000"]
			.into();

	para_account_2001
}

#[test]
fn kusama_treasury_propose_spend() {
	sp_io::TestExternalities::default().execute_with(|| {
		KusamaNet::execute_with(|| {
			assert_ok!(kusama_runtime::Treasury::propose_spend(
				kusama_runtime::Origin::signed(ALICE.into()),
				50_000_000_000_000_000,
				sp_runtime::MultiAddress::Id(para_account_2001()),
			));
		});
	})
}

#[test]
fn bifrost_treasury_operations() {
	sp_io::TestExternalities::default().execute_with(|| {
		let para_id = 2001u32;
		let treasury_account: AccountId32 =
			bifrost_kusama_runtime::TreasuryPalletId::get().into_account_truncating();
		Bifrost::execute_with(|| {
			let treasury_derivative_account_id =
				bifrost_kusama_runtime::Utility::derivative_account_id(treasury_account.clone(), 0);
			assert_ok!(bifrost_kusama_runtime::Tokens::set_balance(
				bifrost_kusama_runtime::Origin::root(),
				sp_runtime::MultiAddress::Id(treasury_derivative_account_id.clone()),
				RelayCurrencyId::get(),
				50_000_000_000_000_000,
				0,
			));
			assert_ok!(bifrost_kusama_runtime::Tokens::force_transfer(
				bifrost_kusama_runtime::Origin::root(),
				sp_runtime::MultiAddress::Id(treasury_derivative_account_id),
				sp_runtime::MultiAddress::Id(treasury_account.clone()),
				RelayCurrencyId::get(),
				50_000_000_000_000_000,
			));

			assert_ok!(bifrost_kusama_runtime::VtokenMinting::mint(
				bifrost_kusama_runtime::Origin::signed(treasury_account.clone()),
				RelayCurrencyId::get(),
				25_000_000_000_000_000,
			));

			assert_ok!(bifrost_kusama_runtime::ZenlinkProtocol::create_pair(
				bifrost_kusama_runtime::Origin::root(),
				zenlink_protocol::AssetId::try_convert_from(KSM, para_id).unwrap(),
				zenlink_protocol::AssetId::try_convert_from(VKSM, para_id).unwrap(),
			));

			assert_ok!(bifrost_kusama_runtime::ZenlinkProtocol::add_liquidity(
				bifrost_kusama_runtime::Origin::signed(treasury_account.clone()),
				zenlink_protocol::AssetId::try_convert_from(KSM, para_id).unwrap(),
				zenlink_protocol::AssetId::try_convert_from(VKSM, para_id).unwrap(),
				25_000_000_000_000_000,
				25_000_000_000_000_000,
				0,
				0,
				BLOCKS_PER_YEAR,
			));

			let lp_asset_id = bifrost_kusama_runtime::ZenlinkProtocol::lp_asset_id(
				&zenlink_protocol::AssetId::try_convert_from(KSM, para_id).unwrap(),
				&zenlink_protocol::AssetId::try_convert_from(VKSM, para_id).unwrap(),
			);

			let lp = bifrost_kusama_runtime::ZenlinkProtocol::foreign_balance_of(
				lp_asset_id,
				&treasury_account,
			);

			assert_ok!(bifrost_kusama_runtime::ZenlinkProtocol::remove_liquidity(
				bifrost_kusama_runtime::Origin::signed(treasury_account.clone()),
				zenlink_protocol::AssetId::try_convert_from(KSM, para_id).unwrap(),
				zenlink_protocol::AssetId::try_convert_from(VKSM, para_id).unwrap(),
				lp,
				0,
				0,
				sp_runtime::MultiAddress::Id(treasury_account.clone()),
				BLOCKS_PER_YEAR,
			));

			assert_ok!(bifrost_kusama_runtime::VtokenMinting::set_unlock_duration(
				Origin::root(),
				KSM,
				TimeUnit::Era(0)
			));
			assert_ok!(bifrost_kusama_runtime::VtokenMinting::update_ongoing_time_unit(
				KSM,
				TimeUnit::Era(1)
			));

			assert_ok!(bifrost_kusama_runtime::VtokenMinting::redeem(
				bifrost_kusama_runtime::Origin::signed(treasury_account),
				VKSM,
				0,
			));
		});
	})
}
