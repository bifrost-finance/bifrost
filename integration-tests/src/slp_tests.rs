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

//! Cross-chain transfer tests within Kusama network.

use bifrost_slp::{Ledger, MinimumsMaximums, SubstrateLedger, XcmOperation};
use frame_support::assert_ok;
use orml_traits::MultiCurrency;
use pallet_staking::StakingLedger;
use xcm::{latest::prelude::*, VersionedMultiAssets, VersionedMultiLocation};
use xcm_emulator::TestExt;

use crate::{integration_tests::*, kusama_test_net::*};

/// ****************************************************
/// *********  Preparation section  ********************
/// ****************************************************

// parachain 2001 subaccount index 0
fn subaccount_0() -> AccountId {
	// 5E78xTBiaN3nAGYtcNnqTJQJqYAkSDGggKqaDfpNsKyPpbcb
	let subaccount_0: AccountId =
		hex_literal::hex!["5a53736d8e96f1c007cf0d630acf5209b20611617af23ce924c8e25328eb5d28"]
			.into();

	subaccount_0
}

fn para_account_2001() -> AccountId {
	// 5Ec4AhPV91i9yNuiWuNunPf6AQCYDhFTTA4G5QCbtqYApH9E
	let para_account_2001: AccountId =
		hex_literal::hex!["70617261d1070000000000000000000000000000000000000000000000000000"]
			.into();

	para_account_2001
}

// Preparation: register sub-account index 0.
#[test]
fn register_subaccount_index_0() {
	let subaccount_0 = subaccount_0();

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] = Slp::account_id_to_account_32(subaccount_0).unwrap();

		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		// First to setup index-multilocation relationship of subaccount_0
		assert_ok!(Slp::add_delegator(
			Origin::root(),
			RelayCurrencyId::get(),
			0u16,
			subaccount_0_location.clone(),
		));

		// Register Operation weight and fee
		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			Origin::root(),
			RelayCurrencyId::get(),
			XcmOperation::TransferTo,
			Some((20_000_000_000, 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			Origin::root(),
			RelayCurrencyId::get(),
			XcmOperation::Bond,
			Some((20_000_000_000, 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			Origin::root(),
			RelayCurrencyId::get(),
			XcmOperation::BondExtra,
			Some((20_000_000_000, 10_000_000_000)),
		));

		let mins_and_maxs = MinimumsMaximums {
			delegator_bonded_minimum: 100_000_000_000,
			bond_extra_minimum: 0,
			unbond_minimum: 0,
			rebond_minimum: 0,
			unbond_record_maximum: 32,
			validators_back_maximum: 36,
			delegator_active_staking_maximum: 200_000_000_000_000,
		};

		// Set minimums and maximums
		assert_ok!(Slp::set_minimums_and_maximums(
			Origin::root(),
			RelayCurrencyId::get(),
			Some(mins_and_maxs)
		));
	});
}

fn register_delegator_ledger() {
	let subaccount_0 = subaccount_0();
	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] = Slp::account_id_to_account_32(subaccount_0).unwrap();
		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		let sb_ledger = SubstrateLedger {
			account: subaccount_0_location.clone(),
			total: dollar(RelayCurrencyId::get()),
			active: dollar(RelayCurrencyId::get()),
			unlocking: vec![],
		};
		let ledger = Ledger::Substrate(sb_ledger);

		// Set delegator ledger
		assert_ok!(Slp::set_delegator_ledger(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location.clone(),
			Some(ledger)
		));
	});
}

// Preparation: transfer 1 KSM from Alice in Kusama to Bob in Bifrost.
// Bob has a balance of
#[test]
fn transfer_2_KSM_to_BOB_in_Bifrost() {
	let para_account_2001 = para_account_2001();

	// Cross-chain transfer some KSM to Bob account in Bifrost
	KusamaNet::execute_with(|| {
		assert_ok!(kusama_runtime::XcmPallet::reserve_transfer_assets(
			kusama_runtime::Origin::signed(ALICE.into()),
			Box::new(VersionedMultiLocation::V1(X1(Parachain(2001)).into())),
			Box::new(VersionedMultiLocation::V1(
				X1(Junction::AccountId32 { id: BOB, network: NetworkId::Any }).into()
			)),
			Box::new(VersionedMultiAssets::V1((Here, 2 * dollar(RelayCurrencyId::get())).into())),
			0,
		));

		// predefined 2 dollars + 2 dollar from cross-chain transfer = 3 dollars
		assert_eq!(
			kusama_runtime::Balances::free_balance(&para_account_2001.clone()),
			4 * dollar(RelayCurrencyId::get())
		);
	});

	Bifrost::execute_with(|| {
		//  Bob get the cross-transferred 1 dollar KSM.
		assert_eq!(
			Tokens::free_balance(RelayCurrencyId::get(), &AccountId::from(BOB)),
			1999936000000
		);
	});
}

// Preparation: transfer 1 KSM from Alice in Kusama to Bob in Bifrost.
// Bob has a balance of
#[test]
fn transfer_2_KSM_to_subaccount_in_Kusama() {
	let subaccount_0 = subaccount_0();

	KusamaNet::execute_with(|| {
		assert_ok!(kusama_runtime::Balances::transfer(
			kusama_runtime::Origin::signed(ALICE.into()),
			MultiAddress::Id(subaccount_0.clone()),
			2 * dollar(RelayCurrencyId::get())
		));

		assert_eq!(
			kusama_runtime::Balances::free_balance(&subaccount_0.clone()),
			2 * dollar(RelayCurrencyId::get())
		);
	});
}

/// ****************************************************
/// *********  Test section  ********************
/// ****************************************************

#[test]
fn transfer_to_works() {
	register_subaccount_index_0();
	transfer_2_KSM_to_BOB_in_Bifrost();
	transfer_2_KSM_to_subaccount_in_Kusama();
	let subaccount_0 = subaccount_0();
	let para_account_2001 = para_account_2001();

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		// We use transfer_to to transfer some KSM to subaccount_0
		assert_ok!(Slp::transfer_to(
			Origin::root(),
			RelayCurrencyId::get(),
			AccountId::from(BOB),
			subaccount_0_location,
			dollar(RelayCurrencyId::get()),
		));
	});

	KusamaNet::execute_with(|| {
		assert_eq!(
			kusama_runtime::Balances::free_balance(&para_account_2001.clone()),
			3 * dollar(RelayCurrencyId::get())
		);

		// Why not the transferred amount reach the sub-account?
		assert_eq!(
			kusama_runtime::Balances::free_balance(&subaccount_0.clone()),
			3 * dollar(RelayCurrencyId::get())
		);
	});
}

#[test]
fn locally_bond_subaccount_0_1ksm_in_kusama() {
	transfer_2_KSM_to_subaccount_in_Kusama();
	let subaccount_0 = subaccount_0();

	KusamaNet::execute_with(|| {
		assert_ok!(kusama_runtime::Staking::bond(
			kusama_runtime::Origin::signed(subaccount_0.clone()),
			MultiAddress::Id(subaccount_0.clone()),
			dollar(RelayCurrencyId::get()),
			pallet_staking::RewardDestination::<AccountId>::Staked,
		));

		assert_eq!(
			kusama_runtime::Staking::ledger(&subaccount_0),
			Some(StakingLedger {
				stash: subaccount_0.clone(),
				total: dollar(RelayCurrencyId::get()),
				active: dollar(RelayCurrencyId::get()),
				unlocking: vec![],
				claimed_rewards: vec![],
			})
		);
	});
}

#[test]
fn bond_works() {
	register_subaccount_index_0();
	transfer_2_KSM_to_subaccount_in_Kusama();
	let subaccount_0 = subaccount_0();
	let para_account_2001 = para_account_2001();

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		// Bond 1 ksm for sub-account index 0
		assert_ok!(Slp::bond(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location,
			dollar(RelayCurrencyId::get()),
		));
	});

	KusamaNet::execute_with(|| {
		assert_eq!(
			kusama_runtime::Staking::ledger(&subaccount_0),
			Some(StakingLedger {
				stash: subaccount_0.clone(),
				total: dollar(RelayCurrencyId::get()),
				active: dollar(RelayCurrencyId::get()),
				unlocking: vec![],
				claimed_rewards: vec![],
			})
		);
	});
}

#[test]
fn bond_extra_works() {
	// bond 1 ksm for sub-account index 0
	locally_bond_subaccount_0_1ksm_in_kusama();
	register_subaccount_index_0();
	register_delegator_ledger();
	let subaccount_0 = subaccount_0();
	let para_account_2001 = para_account_2001();

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		// Bond_extra 1 ksm for sub-account index 0
		assert_ok!(Slp::bond_extra(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location,
			dollar(RelayCurrencyId::get()),
		));
	});

	// So the bonded amount should be 2 ksm
	KusamaNet::execute_with(|| {
		assert_eq!(
			kusama_runtime::Staking::ledger(&subaccount_0),
			Some(StakingLedger {
				stash: subaccount_0.clone(),
				total: 2 * dollar(RelayCurrencyId::get()),
				active: 2 * dollar(RelayCurrencyId::get()),
				unlocking: vec![],
				claimed_rewards: vec![],
			})
		);
	});
}
