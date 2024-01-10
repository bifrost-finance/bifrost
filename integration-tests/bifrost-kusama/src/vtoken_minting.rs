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

use bifrost_asset_registry::AssetIdMaps;
use bifrost_kusama_runtime::{
	Currencies, MultiCurrency, Runtime, SlpEntrancePalletId, VtokenMinting, VtokenMintingOperator,
};
use bifrost_primitives::{CurrencyIdRegister, TimeUnit, TokenSymbol, KSM, VKSM};
use frame_support::{assert_ok, dispatch::RawOrigin};
use integration_tests_common::{BifrostKusama, BifrostKusamaAlice};
use sp_runtime::{traits::AccountIdConversion, BoundedVec, Permill};
use xcm_emulator::TestExt;

#[test]
fn set_unlock_duration_should_work() {
	BifrostKusama::execute_with(|| {
		assert_ok!(VtokenMinting::set_unlock_duration(
			RawOrigin::Root.into(),
			KSM,
			TimeUnit::Era(28),
		));
		assert_eq!(VtokenMinting::unlock_duration(KSM), Some(TimeUnit::Era(28)));
	});
}

#[test]
fn set_minimum_mint_should_work() {
	BifrostKusama::execute_with(|| {
		assert_ok!(VtokenMinting::set_minimum_mint(RawOrigin::Root.into(), KSM, 50_000_000_000,));
		assert_eq!(VtokenMinting::minimum_mint(KSM), 50_000_000_000);
		assert_eq!(AssetIdMaps::<Runtime>::check_vtoken_registered(TokenSymbol::KSM), true);
		assert_eq!(AssetIdMaps::<Runtime>::check_token_registered(TokenSymbol::KSM), true);
	});
}

#[test]
fn set_minimum_redeem_should_work() {
	BifrostKusama::execute_with(|| {
		assert_ok!(VtokenMinting::set_minimum_redeem(RawOrigin::Root.into(), KSM, 10_000,));
		assert_eq!(VtokenMinting::minimum_redeem(KSM), 10_000);
	});
}

#[test]
fn add_support_rebond_token_should_work() {
	BifrostKusama::execute_with(|| {
		assert_eq!(VtokenMinting::token_to_rebond(KSM), None);
		assert_ok!(VtokenMinting::add_support_rebond_token(RawOrigin::Root.into(), KSM,));
		assert_eq!(VtokenMinting::token_to_rebond(KSM), Some(0));
	});
}

#[test]
fn remove_support_rebond_token_should_work() {
	BifrostKusama::execute_with(|| {
		assert_eq!(VtokenMinting::token_to_rebond(KSM), None);
		assert_ok!(VtokenMinting::add_support_rebond_token(RawOrigin::Root.into(), KSM,));
		assert_eq!(VtokenMinting::token_to_rebond(KSM), Some(0));
		assert_ok!(VtokenMinting::remove_support_rebond_token(RawOrigin::Root.into(), KSM,));
		assert_eq!(VtokenMinting::token_to_rebond(KSM), None);
	});
}

#[test]
fn set_fees_should_work() {
	BifrostKusama::execute_with(|| {
		assert_ok!(VtokenMinting::set_fees(
			RawOrigin::Root.into(),
			Permill::from_perthousand(0),
			Permill::from_perthousand(1),
		));
		assert_eq!(
			VtokenMinting::fees(),
			(Permill::from_perthousand(0), Permill::from_perthousand(1))
		);
		println!("{:#?}", Permill::from_perthousand(1));
	});
}

#[test]
fn set_hook_iteration_limit_should_work() {
	BifrostKusama::execute_with(|| {
		assert_ok!(VtokenMinting::set_hook_iteration_limit(RawOrigin::Root.into(), 10));
		assert_eq!(VtokenMinting::hook_iteration_limit(), 10);
	});
}

#[test]
fn set_unlocking_total_should_work() {
	BifrostKusama::execute_with(|| {
		assert_ok!(
			VtokenMinting::set_unlocking_total(RawOrigin::Root.into(), KSM, 10_000_000_000,)
		);
		assert_eq!(VtokenMinting::unlocking_total(KSM), 10_000_000_000);
	});
}

#[test]
fn set_min_time_unit_should_work() {
	BifrostKusama::execute_with(|| {
		assert_ok!(VtokenMinting::set_min_time_unit(
			RawOrigin::Root.into(),
			KSM,
			TimeUnit::Era(4362),
		));
		assert_eq!(VtokenMinting::min_time_unit(KSM), TimeUnit::Era(4362));
	});
}

#[test]
fn mint_should_work() {
	BifrostKusama::execute_with(|| {
		assert_eq!(VtokenMinting::token_pool(KSM), 0);
		assert_eq!(Currencies::total_issuance(VKSM), 0);
		assert_eq!(
			VtokenMinting::fees(),
			(Permill::from_perthousand(0), Permill::from_perthousand(0))
		);

		assert_ok!(VtokenMinting::mint(
			RawOrigin::Signed(BifrostKusamaAlice::get()).into(),
			KSM,
			5_000_000_000_000,
			BoundedVec::default(),
			None
		));

		//check balance
		let entrance_account = SlpEntrancePalletId::get().into_account_truncating();
		assert_eq!(Currencies::free_balance(VKSM, &BifrostKusamaAlice::get()), 5_000_000_000_000);
		assert_eq!(Currencies::free_balance(KSM, &entrance_account), 5_000_000_000_000);
		assert_eq!(VtokenMinting::token_pool(KSM), 5_000_000_000_000);
	});
}

#[test]
fn redeem_should_work() {
	BifrostKusama::execute_with(|| {
		pub const FEE: Permill = Permill::from_percent(2);
		assert_ok!(VtokenMinting::set_fees(RawOrigin::Root.into(), FEE, FEE));
		assert_ok!(VtokenMinting::set_unlock_duration(
			RawOrigin::Root.into(),
			KSM,
			TimeUnit::Era(1)
		));
		assert_ok!(VtokenMinting::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
		assert_ok!(VtokenMinting::set_minimum_redeem(
			RawOrigin::Root.into(),
			KSM,
			2 * 1_000_000_000_000
		));
		assert_ok!(VtokenMinting::mint(
			RawOrigin::Signed(BifrostKusamaAlice::get()).into(),
			KSM,
			5 * 1_000_000_000_000,
			BoundedVec::default(),
			None
		));
		assert_eq!(VtokenMinting::token_pool(KSM), 5 * 1_000_000_000_000 - 5 * 20_000_000_000); // 1000 + 980 - 98 - 196

		assert_ok!(VtokenMinting::redeem(
			RawOrigin::Signed(BifrostKusamaAlice::get()).into(),
			VKSM,
			1 * 1_000_000_000_000
		));
		assert_eq!(
			VtokenMinting::token_pool(KSM),
			5 * 1_000_000_000_000 - 5 * 20_000_000_000 - 1_000_000_000_000 + 20_000_000_000
		);
	});
}
