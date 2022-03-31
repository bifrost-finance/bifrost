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

use bifrost_slp::{
	primitives::UnlockChunk, Delays, Ledger, MinimumsMaximums, SubstrateLedger, XcmOperation,
};
use frame_support::assert_ok;
use node_primitives::TimeUnit;
use orml_traits::MultiCurrency;
use pallet_staking::{Nominations, StakingLedger};
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
fn register_subaccount_index_0() {
	let subaccount_0 = subaccount_0();

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] = Slp::account_id_to_account_32(subaccount_0).unwrap();

		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		// Initialize ongoing timeunit as 0.
		assert_ok!(Slp::update_ongoing_time_unit(
			Origin::root(),
			RelayCurrencyId::get(),
			TimeUnit::Era(0)
		));

		// Initialize currency delays.
		let delay = Delays { unlock_delay: TimeUnit::Era(10) };
		assert_ok!(Slp::set_currency_delays(Origin::root(), RelayCurrencyId::get(), Some(delay)));

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

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			Origin::root(),
			RelayCurrencyId::get(),
			XcmOperation::Unbond,
			Some((20_000_000_000, 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			Origin::root(),
			RelayCurrencyId::get(),
			XcmOperation::Rebond,
			Some((20_000_000_000, 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			Origin::root(),
			RelayCurrencyId::get(),
			XcmOperation::Delegate,
			Some((20_000_000_000, 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			Origin::root(),
			RelayCurrencyId::get(),
			XcmOperation::Payout,
			Some((20_000_000_000, 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			Origin::root(),
			RelayCurrencyId::get(),
			XcmOperation::Liquidize,
			Some((20_000_000_000, 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			Origin::root(),
			RelayCurrencyId::get(),
			XcmOperation::Chill,
			Some((20_000_000_000, 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			Origin::root(),
			RelayCurrencyId::get(),
			XcmOperation::TransferBack,
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

#[test]
fn register_validators() {
	// GsvVmjr1CBHwQHw84pPHMDxgNY3iBLz6Qn7qS3CH8qPhrHz
	let validator_0: AccountId =
		hex_literal::hex!["be5ddb1579b72e84524fc29e78609e3caf42e85aa118ebfe0b0ad404b5bdd25f"]
			.into();

	// JKspFU6ohf1Grg3Phdzj2pSgWvsYWzSfKghhfzMbdhNBWs5
	let validator_1: AccountId =
		hex_literal::hex!["fe65717dad0447d715f660a0a58411de509b42e6efb8375f562f58a554d5860e"]
			.into();

	Bifrost::execute_with(|| {
		let mut valis = vec![];

		let validator_0_32: [u8; 32] = Slp::account_id_to_account_32(validator_0).unwrap();
		let validator_0_location: MultiLocation =
			Slp::account_32_to_parent_location(validator_0_32).unwrap();
		let multi_hash_0 = Slp::get_hash(&validator_0_location);
		valis.push((validator_0_location.clone(), multi_hash_0));

		// Set delegator ledger
		assert_ok!(Slp::add_validator(
			Origin::root(),
			RelayCurrencyId::get(),
			validator_0_location,
		));

		let validator_1_32: [u8; 32] = Slp::account_id_to_account_32(validator_1).unwrap();
		let validator_1_location: MultiLocation =
			Slp::account_32_to_parent_location(validator_1_32).unwrap();
		let multi_hash_1 = Slp::get_hash(&validator_1_location);
		valis.push((validator_1_location.clone(), multi_hash_1));

		// Set delegator ledger
		assert_ok!(Slp::add_validator(
			Origin::root(),
			RelayCurrencyId::get(),
			validator_1_location,
		));

		assert_eq!(Slp::get_validators(RelayCurrencyId::get()), Some(valis));
	});
}

// Preparation: transfer 1 KSM from Alice in Kusama to Bob in Bifrost.
// Bob has a balance of
#[test]
fn transfer_2_ksm_to_bob_in_bifrost() {
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
fn transfer_2_ksm_to_subaccount_in_kusama() {
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

#[test]
fn locally_bond_subaccount_0_1ksm_in_kusama() {
	transfer_2_ksm_to_subaccount_in_kusama();
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

/// ****************************************************
/// *********  Test section  ********************
/// ****************************************************

#[test]
fn transfer_to_works() {
	register_subaccount_index_0();
	transfer_2_ksm_to_bob_in_bifrost();
	transfer_2_ksm_to_subaccount_in_kusama();
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
		assert_eq!(kusama_runtime::Balances::free_balance(&subaccount_0.clone()), 2999893333340);
	});
}

#[test]
fn bond_works() {
	register_subaccount_index_0();
	transfer_2_ksm_to_subaccount_in_kusama();
	let subaccount_0 = subaccount_0();

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

#[test]
fn unbond_works() {
	// bond 1 ksm for sub-account index 0
	locally_bond_subaccount_0_1ksm_in_kusama();
	register_subaccount_index_0();
	register_delegator_ledger();
	let subaccount_0 = subaccount_0();

	KusamaNet::execute_with(|| {
		kusama_runtime::Staking::trigger_new_era(0, vec![]);
	});

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		// Unbond 0.5 ksm, 0.5 ksm left.
		assert_ok!(Slp::unbond(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location,
			500_000_000_000,
		));
	});

	// Can be uncommented to check if the result is correct.
	// Due to the reason of private fields for struct UnlockChunk,
	// it is not able to construct an instance of UnlockChunk directly.
	// // So the bonded amount should be 2 ksm
	// KusamaNet::execute_with(|| {
	// 	assert_eq!(
	// 		kusama_runtime::Staking::ledger(&subaccount_0),
	// 		Some(StakingLedger {
	// 			stash: subaccount_0.clone(),
	// 			total: dollar(RelayCurrencyId::get()),
	// 			active: 500_000_000_000,
	// 			unlocking: vec![UnlockChunk { value: 500000000000, era: 28 }],
	// 			claimed_rewards: vec![],
	// 		})
	// 	);
	// });
}

#[test]
fn unbond_all_works() {
	// bond 1 ksm for sub-account index 0
	locally_bond_subaccount_0_1ksm_in_kusama();
	register_subaccount_index_0();
	register_delegator_ledger();
	let subaccount_0 = subaccount_0();

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		// Unbond the only bonded 1 ksm.
		assert_ok!(Slp::unbond_all(Origin::root(), RelayCurrencyId::get(), subaccount_0_location,));
	});

	// Can be uncommented to check if the result is correct.
	// Due to the reason of private fields for struct UnlockChunk,
	// it is not able to construct an instance of UnlockChunk directly.
	// KusamaNet::execute_with(|| {
	// 	assert_eq!(
	// 		kusama_runtime::Staking::ledger(&subaccount_0),
	// 		Some(StakingLedger {
	// 			stash: subaccount_0.clone(),
	// 			total: dollar(RelayCurrencyId::get()),
	// 			active: 0,
	// 			unlocking: vec![UnlockChunk { value: 1000000000000, era: 28 }],
	// 			claimed_rewards: vec![],
	// 		})
	// 	);
	// });
}

#[test]
fn rebond_works() {
	// bond 1 ksm for sub-account index 0
	locally_bond_subaccount_0_1ksm_in_kusama();
	register_subaccount_index_0();
	register_delegator_ledger();
	let subaccount_0 = subaccount_0();

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		// Unbond 0.5 ksm, 0.5 ksm left.
		assert_ok!(Slp::unbond(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location.clone(),
			500_000_000_000,
		));

		// Update Bifrost local ledger. This should be done by backend services.
		let chunk = UnlockChunk { value: 500_000_000_000, unlock_time: TimeUnit::Era(8) };
		let sb_ledger = SubstrateLedger {
			account: subaccount_0_location.clone(),
			total: dollar(RelayCurrencyId::get()),
			active: 500_000_000_000,
			unlocking: vec![chunk],
		};
		let ledger = Ledger::Substrate(sb_ledger);

		assert_ok!(Slp::set_delegator_ledger(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location.clone(),
			Some(ledger)
		));

		// rebond 0.5 ksm.
		assert_ok!(Slp::rebond(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location,
			500_000_000_000,
		));
	});

	// So the bonded amount should be 1 ksm
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
fn delegate_works() {
	// bond 1 ksm for sub-account index 0
	register_validators();
	locally_bond_subaccount_0_1ksm_in_kusama();
	register_subaccount_index_0();
	register_delegator_ledger();
	let subaccount_0 = subaccount_0();

	// GsvVmjr1CBHwQHw84pPHMDxgNY3iBLz6Qn7qS3CH8qPhrHz
	let validator_0: AccountId =
		hex_literal::hex!["be5ddb1579b72e84524fc29e78609e3caf42e85aa118ebfe0b0ad404b5bdd25f"]
			.into();

	// JKspFU6ohf1Grg3Phdzj2pSgWvsYWzSfKghhfzMbdhNBWs5
	let validator_1: AccountId =
		hex_literal::hex!["fe65717dad0447d715f660a0a58411de509b42e6efb8375f562f58a554d5860e"]
			.into();

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		let mut targets = vec![];

		let validator_0_32: [u8; 32] = Slp::account_id_to_account_32(validator_0.clone()).unwrap();
		let validator_0_location: MultiLocation =
			Slp::account_32_to_parent_location(validator_0_32).unwrap();
		targets.push(validator_0_location.clone());

		let validator_1_32: [u8; 32] = Slp::account_id_to_account_32(validator_1.clone()).unwrap();
		let validator_1_location: MultiLocation =
			Slp::account_32_to_parent_location(validator_1_32).unwrap();
		targets.push(validator_1_location.clone());

		// delegate
		assert_ok!(Slp::delegate(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location.clone(),
			targets.clone(),
		));

		assert_ok!(Slp::set_validators_by_delegator(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location.clone(),
			targets,
		));
	});

	KusamaNet::execute_with(|| {
		assert_eq!(
			kusama_runtime::Staking::nominators(&subaccount_0),
			Some(Nominations {
				targets: vec![validator_0, validator_1],
				submitted_in: 0,
				suppressed: false
			},)
		);
	});
}

#[test]
fn undelegate_works() {
	delegate_works();

	let subaccount_0 = subaccount_0();

	// GsvVmjr1CBHwQHw84pPHMDxgNY3iBLz6Qn7qS3CH8qPhrHz
	let validator_0: AccountId =
		hex_literal::hex!["be5ddb1579b72e84524fc29e78609e3caf42e85aa118ebfe0b0ad404b5bdd25f"]
			.into();

	// JKspFU6ohf1Grg3Phdzj2pSgWvsYWzSfKghhfzMbdhNBWs5
	let validator_1: AccountId =
		hex_literal::hex!["fe65717dad0447d715f660a0a58411de509b42e6efb8375f562f58a554d5860e"]
			.into();

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		let mut targets = vec![];

		let validator_0_32: [u8; 32] = Slp::account_id_to_account_32(validator_0.clone()).unwrap();
		let validator_0_location: MultiLocation =
			Slp::account_32_to_parent_location(validator_0_32).unwrap();
		targets.push(validator_0_location.clone());

		// Undelegate validator 0. Only validator 1 left.
		assert_ok!(Slp::undelegate(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location,
			targets.clone(),
		));
	});

	KusamaNet::execute_with(|| {
		assert_eq!(
			kusama_runtime::Staking::nominators(&subaccount_0),
			Some(Nominations { targets: vec![validator_1], submitted_in: 0, suppressed: false },)
		);
	});
}

#[test]
fn redelegate_works() {
	undelegate_works();

	let subaccount_0 = subaccount_0();

	// GsvVmjr1CBHwQHw84pPHMDxgNY3iBLz6Qn7qS3CH8qPhrHz
	let validator_0: AccountId =
		hex_literal::hex!["be5ddb1579b72e84524fc29e78609e3caf42e85aa118ebfe0b0ad404b5bdd25f"]
			.into();

	// JKspFU6ohf1Grg3Phdzj2pSgWvsYWzSfKghhfzMbdhNBWs5
	let validator_1: AccountId =
		hex_literal::hex!["fe65717dad0447d715f660a0a58411de509b42e6efb8375f562f58a554d5860e"]
			.into();

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		let mut targets = vec![];

		let validator_0_32: [u8; 32] = Slp::account_id_to_account_32(validator_0.clone()).unwrap();
		let validator_0_location: MultiLocation =
			Slp::account_32_to_parent_location(validator_0_32).unwrap();
		targets.push(validator_0_location.clone());

		let validator_1_32: [u8; 32] = Slp::account_id_to_account_32(validator_1.clone()).unwrap();
		let validator_1_location: MultiLocation =
			Slp::account_32_to_parent_location(validator_1_32).unwrap();
		targets.push(validator_1_location.clone());

		// Redelegate to a set of validator_0 and validator_1.
		assert_ok!(Slp::redelegate(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location,
			targets.clone(),
		));
	});

	KusamaNet::execute_with(|| {
		assert_eq!(
			kusama_runtime::Staking::nominators(&subaccount_0),
			Some(Nominations {
				targets: vec![validator_0, validator_1],
				submitted_in: 0,
				suppressed: false
			},)
		);
	});
}

#[test]
fn payout_works() {
	register_subaccount_index_0();
	transfer_2_ksm_to_subaccount_in_kusama();
	let subaccount_0 = subaccount_0();

	// GsvVmjr1CBHwQHw84pPHMDxgNY3iBLz6Qn7qS3CH8qPhrHz
	let validator_0: AccountId =
		hex_literal::hex!["be5ddb1579b72e84524fc29e78609e3caf42e85aa118ebfe0b0ad404b5bdd25f"]
			.into();

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		let validator_0_32: [u8; 32] = Slp::account_id_to_account_32(validator_0.clone()).unwrap();
		let validator_0_location: MultiLocation =
			Slp::account_32_to_parent_location(validator_0_32).unwrap();

		// Bond 1 ksm for sub-account index 0
		assert_ok!(Slp::payout(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location,
			validator_0_location,
			Some(TimeUnit::Era(27))
		));
	});
}

#[test]
fn liquidize_works() {
	unbond_works();
	let subaccount_0 = subaccount_0();

	KusamaNet::execute_with(|| {
		// Kusama's unbonding period is 27 days = 100_800 blocks
		kusama_runtime::System::set_block_number(101_000);
		for _i in 0..29 {
			kusama_runtime::Staking::trigger_new_era(0, vec![]);
		}

		assert_eq!(
			kusama_runtime::Balances::free_balance(&subaccount_0.clone()),
			2 * dollar(RelayCurrencyId::get())
		);

		// 1ksm is locked for half bonded and half unbonding.
		assert_eq!(
			kusama_runtime::Balances::usable_balance(&subaccount_0.clone()),
			dollar(RelayCurrencyId::get())
		);
	});

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		assert_ok!(Slp::liquidize(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location,
			Some(TimeUnit::SlashingSpan(5))
		));
	});

	KusamaNet::execute_with(|| {
		assert_eq!(
			kusama_runtime::Balances::free_balance(&subaccount_0.clone()),
			2 * dollar(RelayCurrencyId::get())
		);

		// half of 1ksm unlocking has been freed. So the usable balance should be 1.5 ksm
		assert_eq!(
			kusama_runtime::Balances::usable_balance(&subaccount_0.clone()),
			1_500_000_000_000
		);
	});
}

#[test]
fn chill_works() {
	delegate_works();
	let subaccount_0 = subaccount_0();

	// check if sub-account index 0 belongs to the group of nominators
	KusamaNet::execute_with(|| {
		assert_eq!(kusama_runtime::Staking::nominators(&subaccount_0.clone()).is_some(), true);
	});

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		assert_ok!(Slp::chill(Origin::root(), RelayCurrencyId::get(), subaccount_0_location,));
	});

	// check if sub-account index 0 belongs to the group of nominators
	KusamaNet::execute_with(|| {
		assert_eq!(kusama_runtime::Staking::nominators(&subaccount_0.clone()).is_some(), false);
	});
}

#[test]
fn transfer_back_works() {
	bond_works();
	let subaccount_0 = subaccount_0();
	let para_account_2001 = para_account_2001();

	KusamaNet::execute_with(|| {
		// 1ksm is locked for half bonded and half unbonding.
		assert_eq!(
			kusama_runtime::Balances::usable_balance(&subaccount_0.clone()),
			dollar(RelayCurrencyId::get())
		);

		assert_eq!(
			kusama_runtime::Balances::free_balance(&para_account_2001.clone()),
			1999333333375
		);
	});

	Bifrost::execute_with(|| {
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		assert_eq!(
			Tokens::free_balance(RelayCurrencyId::get(), &AccountId::from(ALICE)),
			10_000_000_000_000
		);

		assert_ok!(Slp::transfer_back(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location,
			AccountId::from(ALICE),
			500_000_000_000
		));
	});

	// Parachain account has been deposited the transferred amount.
	KusamaNet::execute_with(|| {
		assert_eq!(kusama_runtime::Balances::usable_balance(&subaccount_0.clone()), 500000000000);
		assert_eq!(
			kusama_runtime::Balances::free_balance(&para_account_2001.clone()),
			2498666666750
		);
	});

	Bifrost::execute_with(|| {
		assert_eq!(
			Tokens::free_balance(RelayCurrencyId::get(), &AccountId::from(ALICE)),
			10999872000000
		);
	});
}

#[test]
fn supplement_fee_reserve_works() {
	let subaccount_0 = subaccount_0();
	delegate_works();
	KusamaNet::execute_with(|| {
		assert_eq!(
			kusama_runtime::Balances::free_balance(&subaccount_0.clone()),
			2 * dollar(RelayCurrencyId::get())
		);
	});

	Bifrost::execute_with(|| {
		// set fee source
		let alice_location = Slp::account_32_to_local_location(ALICE).unwrap();
		assert_ok!(Slp::set_fee_source(
			Origin::root(),
			RelayCurrencyId::get(),
			Some((alice_location.clone(), dollar(RelayCurrencyId::get())))
		));

		// We use supplement_fee_reserve to transfer some KSM to subaccount_0
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		assert_ok!(Slp::supplement_fee_reserve(
			Origin::root(),
			RelayCurrencyId::get(),
			subaccount_0_location,
		));
	});

	KusamaNet::execute_with(|| {
		assert_eq!(
			kusama_runtime::Balances::free_balance(&subaccount_0.clone()),
			2_999_893_333_340
		);
	});
}
