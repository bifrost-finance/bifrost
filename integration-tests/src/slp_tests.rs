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

use bifrost_slp::XcmOperation;
use frame_support::assert_ok;
use orml_traits::MultiCurrency;
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
			Some((5_000_000_000, 5_000_000_000)),
		));
	});
}

// Preparation: transfer 1 KSM from Alice in Kusama to Bob in Bifrost.
// Bob has a balance of
#[test]
fn transfer_1_KSM_to_BOB_in_Bifrost() {
	let para_account_2001 = para_account_2001();

	// Cross-chain transfer some KSM to Bob account in Bifrost
	KusamaNet::execute_with(|| {
		assert_ok!(kusama_runtime::XcmPallet::reserve_transfer_assets(
			kusama_runtime::Origin::signed(ALICE.into()),
			Box::new(VersionedMultiLocation::V1(X1(Parachain(2001)).into())),
			Box::new(VersionedMultiLocation::V1(
				X1(Junction::AccountId32 { id: BOB, network: NetworkId::Any }).into()
			)),
			Box::new(VersionedMultiAssets::V1((Here, dollar(RelayCurrencyId::get())).into())),
			0,
		));

		// predefined 2 dollars + 1 dollar from cross-chain transfer = 3 dollars
		assert_eq!(
			kusama_runtime::Balances::free_balance(&para_account_2001.clone()),
			3 * dollar(RelayCurrencyId::get())
		);
	});

	Bifrost::execute_with(|| {
		//  Bob get the cross-transferred 1 dollar KSM.
		assert_eq!(
			Tokens::free_balance(RelayCurrencyId::get(), &AccountId::from(BOB)),
			999936000000
		);
	});
}

/// ****************************************************
/// *********  Test section  ********************
/// ****************************************************

#[test]
fn transfer_to_works() {
	register_subaccount_index_0();
	let subaccount_0 = subaccount_0();

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] = Slp::account_id_to_account_32(subaccount_0).unwrap();

		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		// We use transfer_to to transfer some KSM to subaccount_0
		assert_ok!(Slp::transfer_to(
			Origin::root(),
			RelayCurrencyId::get(),
			AccountId::from(ALICE),
			subaccount_0_location,
			dollar(RelayCurrencyId::get()),
		));
	});

	// // Send some KSM to subaccount_0
	// KusamaNet::execute_with(|| {
	// 	kusama_runtime::Staking::trigger_new_era(0, vec![]);

	// 	// Transfer some KSM into the parachain.
	// 	assert_ok!(kusama_runtime::Balances::transfer(
	// 		kusama_runtime::Origin::signed(ALICE.into()),
	// 		MultiAddress::Id(subaccount_0.clone()),
	// 		1_001_000_000_000_000
	// 	));
	// });

	// Bifrost::execute_with(|| {
	// 	assert_eq!(
	// 		Tokens::free_balance(RelayCurrencyId::get(), &AccountId::from(BOB)),
	// 		999936000000
	// 	);
	// });
}
