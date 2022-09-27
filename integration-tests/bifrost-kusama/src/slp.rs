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
#![cfg(test)]

use bifrost_polkadot_runtime::PolkadotXcm;
use bifrost_slp::{
	primitives::{
		SubstrateLedgerUpdateEntry, SubstrateLedgerUpdateOperation,
		SubstrateValidatorsByDelegatorUpdateEntry, UnlockChunk,
	},
	Delays, Ledger, LedgerUpdateEntry, MinimumsMaximums, SubstrateLedger,
	ValidatorsByDelegatorUpdateEntry, XcmOperation,
};
use cumulus_primitives_core::relay_chain::HashT;
use frame_support::{assert_ok, BoundedVec};
use node_primitives::TimeUnit;
use orml_traits::MultiCurrency;
use pallet_staking::{Nominations, StakingLedger};
use pallet_xcm::QueryStatus;
use xcm::{latest::prelude::*, VersionedMultiAssets, VersionedMultiLocation};
use xcm_emulator::TestExt;

use crate::{kusama_integration_tests::*, kusama_test_net::*};

/// ****************************************************
/// *********  Preparation section  ********************
/// ****************************************************

// parachain 2001 subaccount index 0
pub fn subaccount_0() -> AccountId {
	// 5E78xTBiaN3nAGYtcNnqTJQJqYAkSDGggKqaDfpNsKyPpbcb
	let subaccount_0: AccountId =
		hex_literal::hex!["5a53736d8e96f1c007cf0d630acf5209b20611617af23ce924c8e25328eb5d28"]
			.into();

	subaccount_0
}

pub fn para_account_2001() -> AccountId {
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

		// Set OngoingTimeUnitUpdateInterval as 1/3 Era(1800 blocks per Era, 12 seconds per
		// block)
		assert_ok!(Slp::set_ongoing_time_unit_update_interval(
			Origin::root(),
			RelayCurrencyId::get(),
			Some(600)
		));

		System::set_block_number(600);

		// Initialize ongoing timeunit as 0.
		assert_ok!(Slp::update_ongoing_time_unit(
			Origin::root(),
			RelayCurrencyId::get(),
			TimeUnit::Era(0)
		));

		// Initialize currency delays.
		let delay =
			Delays { unlock_delay: TimeUnit::Era(10), leave_delegators_delay: Default::default() };
		assert_ok!(Slp::set_currency_delays(Origin::root(), RelayCurrencyId::get(), Some(delay)));

		let mins_and_maxs = MinimumsMaximums {
			delegator_bonded_minimum: 100_000_000_000,
			bond_extra_minimum: 0,
			unbond_minimum: 0,
			rebond_minimum: 0,
			unbond_record_maximum: 32,
			validators_back_maximum: 36,
			delegator_active_staking_maximum: 200_000_000_000_000,
			validators_reward_maximum: 0,
			delegation_amount_minimum: 0,
			delegators_maximum: 100,
			validators_maximum: 300,
		};

		// Set minimums and maximums
		assert_ok!(Slp::set_minimums_and_maximums(
			Origin::root(),
			RelayCurrencyId::get(),
			Some(mins_and_maxs)
		));

		// First to setup index-multilocation relationship of subaccount_0
		assert_ok!(Slp::add_delegator(
			Origin::root(),
			RelayCurrencyId::get(),
			0u16,
			Box::new(subaccount_0_location.clone()),
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
			total: dollar::<Runtime>(RelayCurrencyId::get()),
			active: dollar::<Runtime>(RelayCurrencyId::get()),
			unlocking: vec![],
		};
		let ledger = Ledger::Substrate(sb_ledger);

		// Set delegator ledger
		assert_ok!(Slp::set_delegator_ledger(
			Origin::root(),
			RelayCurrencyId::get(),
			Box::new(subaccount_0_location.clone()),
			Box::new(Some(ledger))
		));
	});
}

#[test]
fn register_validators() {
	sp_io::TestExternalities::default().execute_with(|| {
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
			let multi_hash_0 =
				<Runtime as frame_system::Config>::Hashing::hash(&validator_0_location.encode());

			let mins_and_maxs = MinimumsMaximums {
				delegator_bonded_minimum: 100_000_000_000,
				bond_extra_minimum: 0,
				unbond_minimum: 0,
				rebond_minimum: 0,
				unbond_record_maximum: 32,
				validators_back_maximum: 36,
				delegator_active_staking_maximum: 200_000_000_000_000,
				validators_reward_maximum: 0,
				delegation_amount_minimum: 0,
				delegators_maximum: 100,
				validators_maximum: 300,
			};

			// Set minimums and maximums
			assert_ok!(Slp::set_minimums_and_maximums(
				Origin::root(),
				RelayCurrencyId::get(),
				Some(mins_and_maxs)
			));

			// Set delegator ledger
			assert_ok!(Slp::add_validator(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(validator_0_location.clone()),
			));

			let validator_1_32: [u8; 32] = Slp::account_id_to_account_32(validator_1).unwrap();
			let validator_1_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_1_32).unwrap();
			let multi_hash_1 =
				<Runtime as frame_system::Config>::Hashing::hash(&validator_1_location.encode());

			// The storage is reordered by hash. So we need to adjust the push order here.
			valis.push((validator_1_location.clone(), multi_hash_1));
			valis.push((validator_0_location.clone(), multi_hash_0));

			// Set delegator ledger
			assert_ok!(Slp::add_validator(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(validator_1_location),
			));

			assert_eq!(Slp::get_validators(RelayCurrencyId::get()), Some(valis));
		});
	})
}

// Preparation: transfer 1 KSM from Alice in Kusama to Bob in Bifrost.
// Bob has a balance of
#[test]
fn transfer_2_ksm_to_entrance_account_in_bifrost() {
	sp_io::TestExternalities::default().execute_with(|| {
		let para_account_2001 = para_account_2001();

		let entrance_account_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f76746b696e0000000000000000000000000000000000000000"]
				.into();

		// Cross-chain transfer some KSM to Bob account in Bifrost
		KusamaNet::execute_with(|| {
			assert_ok!(kusama_runtime::XcmPallet::reserve_transfer_assets(
				kusama_runtime::Origin::signed(ALICE.into()),
				Box::new(VersionedMultiLocation::V1(X1(Parachain(2001)).into())),
				Box::new(VersionedMultiLocation::V1(
					X1(Junction::AccountId32 { id: entrance_account_32, network: NetworkId::Any })
						.into()
				)),
				Box::new(VersionedMultiAssets::V1(
					(Here, 2 * dollar::<Runtime>(RelayCurrencyId::get())).into()
				)),
				0,
			));

			// predefined 2 dollars + 2 dollar::<Runtime> from cross-chain transfer = 3 dollars
			assert_eq!(
				kusama_runtime::Balances::free_balance(&para_account_2001.clone()),
				4 * dollar::<Runtime>(RelayCurrencyId::get())
			);
		});

		Bifrost::execute_with(|| {
			//  entrance_account get the cross-transferred 2 dollar::<Runtime> KSM minus transaction
			// fee.
			assert_eq!(
				Tokens::free_balance(RelayCurrencyId::get(), &entrance_account_32.into()),
				1999907304000
			);
		});
	})
}

// Preparation: transfer 1 KSM from Alice in Kusama to Bob in Bifrost.
// Bob has a balance of
#[test]
fn transfer_2_ksm_to_subaccount_in_kusama() {
	sp_io::TestExternalities::default().execute_with(|| {
		let subaccount_0 = subaccount_0();

		KusamaNet::execute_with(|| {
			assert_ok!(kusama_runtime::Balances::transfer(
				kusama_runtime::Origin::signed(ALICE.into()),
				MultiAddress::Id(subaccount_0.clone()),
				2 * dollar::<Runtime>(RelayCurrencyId::get())
			));

			assert_eq!(
				kusama_runtime::Balances::free_balance(&subaccount_0.clone()),
				2 * dollar::<Runtime>(RelayCurrencyId::get())
			);
		});
	})
}

#[test]
fn locally_bond_subaccount_0_1ksm_in_kusama() {
	sp_io::TestExternalities::default().execute_with(|| {
		transfer_2_ksm_to_subaccount_in_kusama();
		let subaccount_0 = subaccount_0();

		KusamaNet::execute_with(|| {
			assert_ok!(kusama_runtime::Staking::bond(
				kusama_runtime::Origin::signed(subaccount_0.clone()),
				MultiAddress::Id(subaccount_0.clone()),
				dollar::<Runtime>(RelayCurrencyId::get()),
				pallet_staking::RewardDestination::<AccountId>::Staked,
			));

			assert_eq!(
				kusama_runtime::Staking::ledger(&subaccount_0),
				Some(StakingLedger {
					stash: subaccount_0.clone(),
					total: dollar::<Runtime>(RelayCurrencyId::get()),
					active: dollar::<Runtime>(RelayCurrencyId::get()),
					unlocking: BoundedVec::try_from(vec![]).unwrap(),
					claimed_rewards: vec![],
				})
			);
		});
	})
}

/// ****************************************************
/// *********  Test section  ********************
/// ****************************************************

#[test]
fn transfer_to_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		register_subaccount_index_0();
		transfer_2_ksm_to_entrance_account_in_bifrost();
		transfer_2_ksm_to_subaccount_in_kusama();
		let subaccount_0 = subaccount_0();
		let para_account_2001 = para_account_2001();

		let entrance_account: AccountId =
			hex_literal::hex!["6d6f646c62662f76746b696e0000000000000000000000000000000000000000"]
				.into();

		let entrance_account_32 = Slp::account_id_to_account_32(entrance_account.clone()).unwrap();
		let entrance_account_location: MultiLocation =
			Slp::account_32_to_local_location(entrance_account_32).unwrap();

		Bifrost::execute_with(|| {
			let subaccount_0_32: [u8; 32] =
				Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

			let subaccount_0_location: MultiLocation =
				Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

			// We use transfer_to to transfer some KSM to subaccount_0
			assert_ok!(Slp::transfer_to(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(entrance_account_location),
				Box::new(subaccount_0_location),
				dollar::<Runtime>(RelayCurrencyId::get()),
			));
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Balances::free_balance(&para_account_2001.clone()),
				3 * dollar::<Runtime>(RelayCurrencyId::get())
			);

			// Why not the transferred amount reach the sub-account?
			assert_eq!(
				kusama_runtime::Balances::free_balance(&subaccount_0.clone()),
				2999988476752
			);
		});
	})
}

#[test]
fn bond_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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
				Box::new(subaccount_0_location),
				dollar::<Runtime>(RelayCurrencyId::get()),
				None
			));
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::ledger(&subaccount_0),
				Some(StakingLedger {
					stash: subaccount_0.clone(),
					total: dollar::<Runtime>(RelayCurrencyId::get()),
					active: dollar::<Runtime>(RelayCurrencyId::get()),
					unlocking: BoundedVec::try_from(vec![]).unwrap(),
					claimed_rewards: vec![],
				})
			);

			assert!(kusama_runtime::System::events().iter().any(|r| matches!(
				r.event,
				kusama_runtime::Event::System(frame_system::Event::Remarked { sender: _, hash: _ })
			)));
		});
	})
}

#[test]
fn bond_extra_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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
				Box::new(subaccount_0_location),
				None,
				dollar::<Runtime>(RelayCurrencyId::get()),
			));
		});

		// So the bonded amount should be 2 ksm
		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::ledger(&subaccount_0),
				Some(StakingLedger {
					stash: subaccount_0.clone(),
					total: 2 * dollar::<Runtime>(RelayCurrencyId::get()),
					active: 2 * dollar::<Runtime>(RelayCurrencyId::get()),
					unlocking: BoundedVec::try_from(vec![]).unwrap(),
					claimed_rewards: vec![],
				})
			);
		});
	})
}

#[test]
fn unbond_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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
				Box::new(subaccount_0_location),
				None,
				500_000_000_000,
			));
		});
	})

	// Can be uncommented to check if the result is correct.
	// Due to the reason of private fields for struct UnlockChunk,
	// it is not able to construct an instance of UnlockChunk directly.
	// // So the bonded amount should be 2 ksm
	// KusamaNet::execute_with(|| {
	// 	assert_eq!(
	// 		kusama_runtime::Staking::ledger(&subaccount_0),
	// 		Some(StakingLedger {
	// 			stash: subaccount_0.clone(),
	// 			total: dollar::<Runtime>(RelayCurrencyId::get()),
	// 			active: 500_000_000_000,
	// 			unlocking: vec![UnlockChunk { value: 500000000000, era: 28 }],
	// 			claimed_rewards: vec![],
	// 		})
	// 	);
	// });
}

#[test]
fn unbond_all_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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
			assert_ok!(Slp::unbond_all(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location),
			));
		});
	})

	// Can be uncommented to check if the result is correct.
	// Due to the reason of private fields for struct UnlockChunk,
	// it is not able to construct an instance of UnlockChunk directly.
	// KusamaNet::execute_with(|| {
	// 	assert_eq!(
	// 		kusama_runtime::Staking::ledger(&subaccount_0),
	// 		Some(StakingLedger {
	// 			stash: subaccount_0.clone(),
	// 			total: dollar::<Runtime>(RelayCurrencyId::get()),
	// 			active: 0,
	// 			unlocking: vec![UnlockChunk { value: 1000000000000, era: 28 }],
	// 			claimed_rewards: vec![],
	// 		})
	// 	);
	// });
}

#[test]
fn rebond_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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
				Box::new(subaccount_0_location.clone()),
				None,
				500_000_000_000,
			));

			// Update Bifrost local ledger. This should be done by backend services.
			let chunk = UnlockChunk { value: 500_000_000_000, unlock_time: TimeUnit::Era(8) };
			let sb_ledger = SubstrateLedger {
				account: subaccount_0_location.clone(),
				total: dollar::<Runtime>(RelayCurrencyId::get()),
				active: 500_000_000_000,
				unlocking: vec![chunk],
			};
			let ledger = Ledger::Substrate(sb_ledger);

			assert_ok!(Slp::set_delegator_ledger(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location.clone()),
				Box::new(Some(ledger))
			));

			// rebond 0.5 ksm.
			assert_ok!(Slp::rebond(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location),
				None,
				Some(500_000_000_000),
			));
		});

		// So the bonded amount should be 1 ksm
		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::ledger(&subaccount_0),
				Some(StakingLedger {
					stash: subaccount_0.clone(),
					total: dollar::<Runtime>(RelayCurrencyId::get()),
					active: dollar::<Runtime>(RelayCurrencyId::get()),
					unlocking: BoundedVec::try_from(vec![]).unwrap(),
					claimed_rewards: vec![],
				})
			);
		});
	})
}

#[test]
fn delegate_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		// bond 1 ksm for sub-account index 0
		locally_bond_subaccount_0_1ksm_in_kusama();
		register_subaccount_index_0();
		register_validators();
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

			let validator_0_32: [u8; 32] =
				Slp::account_id_to_account_32(validator_0.clone()).unwrap();
			let validator_0_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_0_32).unwrap();
			targets.push(validator_0_location.clone());

			let validator_1_32: [u8; 32] =
				Slp::account_id_to_account_32(validator_1.clone()).unwrap();
			let validator_1_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_1_32).unwrap();
			targets.push(validator_1_location.clone());

			// delegate
			assert_ok!(Slp::delegate(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location.clone()),
				targets.clone(),
			));

			assert_ok!(Slp::set_validators_by_delegator(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location.clone()),
				targets,
			));
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::nominators(&subaccount_0),
				Some(Nominations {
					targets: BoundedVec::try_from(vec![validator_1, validator_0]).unwrap(),
					submitted_in: 0,
					suppressed: false
				},)
			);
		});
	})
}

#[test]
fn undelegate_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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

			let validator_0_32: [u8; 32] =
				Slp::account_id_to_account_32(validator_0.clone()).unwrap();
			let validator_0_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_0_32).unwrap();
			targets.push(validator_0_location.clone());

			// Undelegate validator 0. Only validator 1 left.
			assert_ok!(Slp::undelegate(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location),
				targets.clone(),
			));
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::nominators(&subaccount_0),
				Some(Nominations {
					targets: BoundedVec::try_from(vec![validator_1]).unwrap(),
					submitted_in: 0,
					suppressed: false
				},)
			);
		});
	})
}

#[test]
fn redelegate_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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

			let validator_0_32: [u8; 32] =
				Slp::account_id_to_account_32(validator_0.clone()).unwrap();
			let validator_0_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_0_32).unwrap();

			let validator_1_32: [u8; 32] =
				Slp::account_id_to_account_32(validator_1.clone()).unwrap();
			let validator_1_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_1_32).unwrap();

			targets.push(validator_1_location.clone());
			targets.push(validator_0_location.clone());

			// Redelegate to a set of validator_0 and validator_1.
			assert_ok!(Slp::redelegate(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location),
				Some(targets.clone()),
			));
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::nominators(&subaccount_0),
				Some(Nominations {
					targets: BoundedVec::try_from(vec![validator_1, validator_0]).unwrap(),
					submitted_in: 0,
					suppressed: false
				},)
			);
		});
	})
}

#[test]
fn payout_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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

			let validator_0_32: [u8; 32] =
				Slp::account_id_to_account_32(validator_0.clone()).unwrap();
			let validator_0_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_0_32).unwrap();

			// Bond 1 ksm for sub-account index 0
			assert_ok!(Slp::payout(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location),
				Box::new(validator_0_location),
				Some(TimeUnit::Era(27))
			));
		});
	})
}

#[test]
fn liquidize_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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
				2 * dollar::<Runtime>(RelayCurrencyId::get())
			);

			// 1ksm is locked for half bonded and half unbonding.
			assert_eq!(
				kusama_runtime::Balances::usable_balance(&subaccount_0.clone()),
				dollar::<Runtime>(RelayCurrencyId::get())
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
				Box::new(subaccount_0_location),
				Some(TimeUnit::SlashingSpan(5)),
				None
			));
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Balances::free_balance(&subaccount_0.clone()),
				2 * dollar::<Runtime>(RelayCurrencyId::get())
			);

			// half of 1ksm unlocking has been freed. So the usable balance should be 1.5 ksm
			assert_eq!(
				kusama_runtime::Balances::usable_balance(&subaccount_0.clone()),
				1_500_000_000_000
			);
		});
	})
}

#[test]
fn chill_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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

			assert_ok!(Slp::chill(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location),
			));
		});

		// check if sub-account index 0 belongs to the group of nominators
		KusamaNet::execute_with(|| {
			assert_eq!(kusama_runtime::Staking::nominators(&subaccount_0.clone()).is_some(), false);
		});
	})
}

#[test]
fn transfer_back_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		bond_works();
		let subaccount_0 = subaccount_0();
		let para_account_2001 = para_account_2001();

		let exit_account: AccountId =
			hex_literal::hex!["6d6f646c62662f76746f75740000000000000000000000000000000000000000"]
				.into();

		let exit_account_32 = Slp::account_id_to_account_32(exit_account.clone()).unwrap();
		let exit_account_location: MultiLocation =
			Slp::account_32_to_local_location(exit_account_32).unwrap();

		KusamaNet::execute_with(|| {
			// 1ksm is locked for half bonded and half unbonding.
			assert_eq!(
				kusama_runtime::Balances::usable_balance(&subaccount_0.clone()),
				dollar::<Runtime>(RelayCurrencyId::get())
			);

			assert_eq!(
				kusama_runtime::Balances::free_balance(&para_account_2001.clone()),
				1999215574218
			);
		});

		Bifrost::execute_with(|| {
			let subaccount_0_32: [u8; 32] =
				Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

			let subaccount_0_location: MultiLocation =
				Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

			assert_eq!(Tokens::free_balance(RelayCurrencyId::get(), &exit_account), 0);

			assert_ok!(Slp::transfer_back(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location),
				Box::new(exit_account_location),
				500_000_000_000
			));
		});

		// Parachain account has been deposited the transferred amount.
		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Balances::usable_balance(&subaccount_0.clone()),
				500_000_000_000
			);
			assert_eq!(
				kusama_runtime::Balances::free_balance(&para_account_2001.clone()),
				2498431148436
			);
		});

		Bifrost::execute_with(|| {
			assert_eq!(Tokens::free_balance(RelayCurrencyId::get(), &exit_account), 499907304000);
		});
	})
}

#[test]
fn supplement_fee_reserve_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		let subaccount_0 = subaccount_0();
		delegate_works();
		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Balances::free_balance(&subaccount_0.clone()),
				2 * dollar::<Runtime>(RelayCurrencyId::get())
			);
		});

		Bifrost::execute_with(|| {
			// set fee source
			let alice_location = Slp::account_32_to_local_location(ALICE).unwrap();
			assert_ok!(Slp::set_fee_source(
				Origin::root(),
				RelayCurrencyId::get(),
				Some((alice_location.clone(), dollar::<Runtime>(RelayCurrencyId::get())))
			));

			// We use supplement_fee_reserve to transfer some KSM to subaccount_0
			let subaccount_0_32: [u8; 32] =
				Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

			let subaccount_0_location: MultiLocation =
				Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

			assert_ok!(Slp::supplement_fee_reserve(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location),
			));
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Balances::free_balance(&subaccount_0.clone()),
				2999988476752
			);
		});
	})
}

#[test]
fn confirm_delegator_ledger_query_response_with_bond_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		register_subaccount_index_0();
		transfer_2_ksm_to_subaccount_in_kusama();
		let subaccount_0 = subaccount_0();

		Bifrost::execute_with(|| {
			let subaccount_0_32: [u8; 32] =
				Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

			let subaccount_0_location: MultiLocation =
				Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

			// First call bond function, it will insert a query.
			// Bond 1 ksm for sub-account index 0
			assert_ok!(Slp::bond(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location.clone()),
				dollar::<Runtime>(RelayCurrencyId::get()),
				None
			));

			// Check the existence of the query in pallet_xcm Queries storage.
			assert_eq!(
				PolkadotXcm::query(0),
				Some(QueryStatus::Pending {
					responder: VersionedMultiLocation::V1(MultiLocation {
						parents: 1,
						interior: Here
					}),
					maybe_notify: None,
					timeout: 1600
				})
			);

			// Check the existence of query in the response update queue storage.
			assert_eq!(
				Slp::get_delegator_ledger_update_entry(0),
				Some((
					LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
						currency_id: RelayCurrencyId::get(),
						delegator_id: subaccount_0_location.clone(),
						update_operation: SubstrateLedgerUpdateOperation::Bond,
						amount: dollar::<Runtime>(RelayCurrencyId::get()),
						unlock_time: None
					}),
					1600
				))
			);
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::ledger(&subaccount_0),
				Some(StakingLedger {
					stash: subaccount_0.clone(),
					total: dollar::<Runtime>(RelayCurrencyId::get()),
					active: dollar::<Runtime>(RelayCurrencyId::get()),
					unlocking: BoundedVec::try_from(vec![]).unwrap(),
					claimed_rewards: vec![],
				})
			);
		});

		Bifrost::execute_with(|| {
			let subaccount_0_32: [u8; 32] =
				Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

			let subaccount_0_location: MultiLocation =
				Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

			// Call confirm_delegator_ledger_query_response.
			assert_ok!(Slp::confirm_delegator_ledger_query_response(
				Origin::root(),
				RelayCurrencyId::get(),
				0
			));

			// Check the ledger update.
			assert_eq!(
				Slp::get_delegator_ledger(RelayCurrencyId::get(), subaccount_0_location.clone()),
				Some(Ledger::Substrate(SubstrateLedger {
					account: subaccount_0_location.clone(),
					total: dollar::<Runtime>(RelayCurrencyId::get()),
					active: dollar::<Runtime>(RelayCurrencyId::get()),
					unlocking: vec![]
				}))
			);

			// Check the existence of the query in pallet_xcm Queries storage.
			// If xcm version 3 is introduced. We'll add instruction ReportTransactStatus into the
			// xcm message. And this query will be set to ready status after we received a query
			// response. At that point, this check should be set to equal None.
			assert_eq!(
				PolkadotXcm::query(0),
				Some(QueryStatus::Pending {
					responder: VersionedMultiLocation::V1(MultiLocation {
						parents: 1,
						interior: Here
					}),
					maybe_notify: None,
					timeout: 1600
				})
			);

			// Check the inexistence of query in the response update queue storage.
			assert_eq!(Slp::get_delegator_ledger_update_entry(0), None);
		});
	})
}

#[test]
fn confirm_delegator_ledger_query_response_with_bond_extra_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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
				Box::new(subaccount_0_location.clone()),
				None,
				dollar::<Runtime>(RelayCurrencyId::get()),
			));

			assert_eq!(
				Slp::get_delegator_ledger(RelayCurrencyId::get(), subaccount_0_location.clone()),
				Some(Ledger::Substrate(SubstrateLedger {
					account: subaccount_0_location.clone(),
					total: dollar::<Runtime>(RelayCurrencyId::get()),
					active: dollar::<Runtime>(RelayCurrencyId::get()),
					unlocking: vec![]
				}))
			);

			// Check the existence of the query in pallet_xcm Queries storage.
			assert_eq!(
				PolkadotXcm::query(0),
				Some(QueryStatus::Pending {
					responder: VersionedMultiLocation::V1(MultiLocation {
						parents: 1,
						interior: Here
					}),
					maybe_notify: None,
					timeout: 1600
				})
			);

			// Check the existence of query in the response update queue storage.
			assert_eq!(
				Slp::get_delegator_ledger_update_entry(0),
				Some((
					LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
						currency_id: RelayCurrencyId::get(),
						delegator_id: subaccount_0_location.clone(),
						update_operation: SubstrateLedgerUpdateOperation::Bond,
						amount: dollar::<Runtime>(RelayCurrencyId::get()),
						unlock_time: None
					}),
					1600
				))
			);
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::ledger(&subaccount_0),
				Some(StakingLedger {
					stash: subaccount_0.clone(),
					total: 2 * dollar::<Runtime>(RelayCurrencyId::get()),
					active: 2 * dollar::<Runtime>(RelayCurrencyId::get()),
					unlocking: BoundedVec::try_from(vec![]).unwrap(),
					claimed_rewards: vec![],
				})
			);
		});

		Bifrost::execute_with(|| {
			let subaccount_0_32: [u8; 32] =
				Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

			let subaccount_0_location: MultiLocation =
				Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

			// Call confirm_delegator_ledger_query_response.
			assert_ok!(Slp::confirm_delegator_ledger_query_response(
				Origin::root(),
				RelayCurrencyId::get(),
				0
			));

			// Check the ledger update.
			assert_eq!(
				Slp::get_delegator_ledger(RelayCurrencyId::get(), subaccount_0_location.clone()),
				Some(Ledger::Substrate(SubstrateLedger {
					account: subaccount_0_location.clone(),
					total: 2 * dollar::<Runtime>(RelayCurrencyId::get()),
					active: 2 * dollar::<Runtime>(RelayCurrencyId::get()),
					unlocking: vec![]
				}))
			);

			// Check the existence of the query in pallet_xcm Queries storage.
			// If xcm version 3 is introduced. We'll add instruction ReportTransactStatus into the
			// xcm message. And this query will be set to ready status after we received a query
			// response. At that point, this check should be set to equal None.
			assert_eq!(
				PolkadotXcm::query(0),
				Some(QueryStatus::Pending {
					responder: VersionedMultiLocation::V1(MultiLocation {
						parents: 1,
						interior: Here
					}),
					maybe_notify: None,
					timeout: 1600
				})
			);

			// Check the inexistence of query in the response update queue storage.
			assert_eq!(Slp::get_delegator_ledger_update_entry(0), None);
		});
	})
}

#[test]
fn confirm_delegator_ledger_query_response_with_unbond_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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
				Box::new(subaccount_0_location.clone()),
				None,
				500_000_000_000,
			));

			assert_eq!(
				Slp::get_delegator_ledger(RelayCurrencyId::get(), subaccount_0_location.clone()),
				Some(Ledger::Substrate(SubstrateLedger {
					account: subaccount_0_location.clone(),
					total: dollar::<Runtime>(RelayCurrencyId::get()),
					active: dollar::<Runtime>(RelayCurrencyId::get()),
					unlocking: vec![]
				}))
			);

			// Check the existence of the query in pallet_xcm Queries storage.
			assert_eq!(
				PolkadotXcm::query(0),
				Some(QueryStatus::Pending {
					responder: VersionedMultiLocation::V1(MultiLocation {
						parents: 1,
						interior: Here
					}),
					maybe_notify: None,
					timeout: 1600
				})
			);

			// Check the existence of query in the response update queue storage.
			assert_eq!(
				Slp::get_delegator_ledger_update_entry(0),
				Some((
					LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
						currency_id: RelayCurrencyId::get(),
						delegator_id: subaccount_0_location.clone(),
						update_operation: SubstrateLedgerUpdateOperation::Unlock,
						amount: 500_000_000_000,
						unlock_time: Some(TimeUnit::Era(10))
					}),
					1600
				))
			);
		});

		Bifrost::execute_with(|| {
			let subaccount_0_32: [u8; 32] =
				Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

			let subaccount_0_location: MultiLocation =
				Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

			// Call confirm_delegator_ledger_query_response.
			assert_ok!(Slp::confirm_delegator_ledger_query_response(
				Origin::root(),
				RelayCurrencyId::get(),
				0
			));

			// Check the ledger update.
			assert_eq!(
				Slp::get_delegator_ledger(RelayCurrencyId::get(), subaccount_0_location.clone()),
				Some(Ledger::Substrate(SubstrateLedger {
					account: subaccount_0_location.clone(),
					total: dollar::<Runtime>(RelayCurrencyId::get()),
					active: 500_000_000_000,
					unlocking: vec![UnlockChunk {
						value: 500000000000,
						unlock_time: TimeUnit::Era(10)
					}]
				}))
			);

			// Check the existence of the query in pallet_xcm Queries storage.
			// If xcm version 3 is introduced. We'll add instruction ReportTransactStatus into the
			// xcm message. And this query will be set to ready status after we received a query
			// response. At that point, this check should be set to equal None.
			assert_eq!(
				PolkadotXcm::query(0),
				Some(QueryStatus::Pending {
					responder: VersionedMultiLocation::V1(MultiLocation {
						parents: 1,
						interior: Here
					}),
					maybe_notify: None,
					timeout: 1600
				})
			);

			// Check the inexistence of query in the response update queue storage.
			assert_eq!(Slp::get_delegator_ledger_update_entry(0), None);
		});
	})
}

#[test]
fn confirm_delegator_ledger_query_response_with_unbond_all_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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
			assert_ok!(Slp::unbond_all(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location.clone()),
			));

			assert_eq!(
				Slp::get_delegator_ledger(RelayCurrencyId::get(), subaccount_0_location.clone()),
				Some(Ledger::Substrate(SubstrateLedger {
					account: subaccount_0_location.clone(),
					total: dollar::<Runtime>(RelayCurrencyId::get()),
					active: dollar::<Runtime>(RelayCurrencyId::get()),
					unlocking: vec![]
				}))
			);

			// Check the existence of the query in pallet_xcm Queries storage.
			assert_eq!(
				PolkadotXcm::query(0),
				Some(QueryStatus::Pending {
					responder: VersionedMultiLocation::V1(MultiLocation {
						parents: 1,
						interior: Here
					}),
					maybe_notify: None,
					timeout: 1600
				})
			);

			// Check the existence of query in the response update queue storage.
			assert_eq!(
				Slp::get_delegator_ledger_update_entry(0),
				Some((
					LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
						currency_id: RelayCurrencyId::get(),
						delegator_id: subaccount_0_location.clone(),
						update_operation: SubstrateLedgerUpdateOperation::Unlock,
						amount: dollar::<Runtime>(RelayCurrencyId::get()),
						unlock_time: Some(TimeUnit::Era(10))
					}),
					1600
				))
			);
		});

		Bifrost::execute_with(|| {
			let subaccount_0_32: [u8; 32] =
				Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

			let subaccount_0_location: MultiLocation =
				Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

			// Call confirm_delegator_ledger_query_response.
			assert_ok!(Slp::confirm_delegator_ledger_query_response(
				Origin::root(),
				RelayCurrencyId::get(),
				0
			));

			// Check the ledger update.
			assert_eq!(
				Slp::get_delegator_ledger(RelayCurrencyId::get(), subaccount_0_location.clone()),
				Some(Ledger::Substrate(SubstrateLedger {
					account: subaccount_0_location.clone(),
					total: dollar::<Runtime>(RelayCurrencyId::get()),
					active: 0,
					unlocking: vec![UnlockChunk {
						value: dollar::<Runtime>(RelayCurrencyId::get()),
						unlock_time: TimeUnit::Era(10)
					}]
				}))
			);

			// Check the existence of the query in pallet_xcm Queries storage.
			// If xcm version 3 is introduced. We'll add instruction ReportTransactStatus into the
			// xcm message. And this query will be set to ready status after we received a query
			// response. At that point, this check should be set to equal None.
			assert_eq!(
				PolkadotXcm::query(0),
				Some(QueryStatus::Pending {
					responder: VersionedMultiLocation::V1(MultiLocation {
						parents: 1,
						interior: Here
					}),
					maybe_notify: None,
					timeout: 1600
				})
			);

			// Check the inexistence of query in the response update queue storage.
			assert_eq!(Slp::get_delegator_ledger_update_entry(0), None);
		});
	})
}

#[test]
fn confirm_delegator_ledger_query_response_with_rebond_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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
				Box::new(subaccount_0_location.clone()),
				None,
				500_000_000_000,
			));

			// Update Bifrost local ledger. This should be done by backend services.
			let chunk = UnlockChunk { value: 500_000_000_000, unlock_time: TimeUnit::Era(10) };
			let sb_ledger = SubstrateLedger {
				account: subaccount_0_location.clone(),
				total: dollar::<Runtime>(RelayCurrencyId::get()),
				active: 500_000_000_000,
				unlocking: vec![chunk],
			};
			let ledger = Ledger::Substrate(sb_ledger);

			assert_ok!(Slp::set_delegator_ledger(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location.clone()),
				Box::new(Some(ledger))
			));

			// rebond 0.5 ksm.
			assert_ok!(Slp::rebond(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location.clone()),
				None,
				Some(500_000_000_000),
			));

			assert_eq!(
				Slp::get_delegator_ledger(RelayCurrencyId::get(), subaccount_0_location.clone()),
				Some(Ledger::Substrate(SubstrateLedger {
					account: subaccount_0_location.clone(),
					total: dollar::<Runtime>(RelayCurrencyId::get()),
					active: 500_000_000_000,
					unlocking: vec![UnlockChunk {
						value: 500_000_000_000,
						unlock_time: TimeUnit::Era(10)
					}]
				}))
			);

			// Check the existence of the query in pallet_xcm Queries storage.
			assert_eq!(
				PolkadotXcm::query(1),
				Some(QueryStatus::Pending {
					responder: VersionedMultiLocation::V1(MultiLocation {
						parents: 1,
						interior: Here
					}),
					maybe_notify: None,
					timeout: 1600
				})
			);

			// Check the existence of query in the response update queue storage.
			assert_eq!(
				Slp::get_delegator_ledger_update_entry(1),
				Some((
					LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
						currency_id: RelayCurrencyId::get(),
						delegator_id: subaccount_0_location.clone(),
						update_operation: SubstrateLedgerUpdateOperation::Rebond,
						amount: 500_000_000_000,
						unlock_time: None
					}),
					1600
				))
			);
		});

		Bifrost::execute_with(|| {
			let subaccount_0_32: [u8; 32] =
				Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

			let subaccount_0_location: MultiLocation =
				Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

			// Call confirm_delegator_ledger_query_response.
			assert_ok!(Slp::confirm_delegator_ledger_query_response(
				Origin::root(),
				RelayCurrencyId::get(),
				1
			));

			// Check the ledger update.
			assert_eq!(
				Slp::get_delegator_ledger(RelayCurrencyId::get(), subaccount_0_location.clone()),
				Some(Ledger::Substrate(SubstrateLedger {
					account: subaccount_0_location.clone(),
					total: dollar::<Runtime>(RelayCurrencyId::get()),
					active: dollar::<Runtime>(RelayCurrencyId::get()),
					unlocking: vec![]
				}))
			);

			// Check the existence of the query in pallet_xcm Queries storage.
			// If xcm version 3 is introduced. We'll add instruction ReportTransactStatus into the
			// xcm message. And this query will be set to ready status after we received a query
			// response. At that point, this check should be set to equal None.
			assert_eq!(
				PolkadotXcm::query(1),
				Some(QueryStatus::Pending {
					responder: VersionedMultiLocation::V1(MultiLocation {
						parents: 1,
						interior: Here
					}),
					maybe_notify: None,
					timeout: 1600
				})
			);

			// Check the inexistence of query in the response update queue storage.
			assert_eq!(Slp::get_delegator_ledger_update_entry(1), None);
		});
	})
}

#[test]
fn confirm_delegator_ledger_query_response_with_liquidize_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		confirm_delegator_ledger_query_response_with_unbond_works();
		let subaccount_0 = subaccount_0();

		KusamaNet::execute_with(|| {
			// Kusama's unbonding period is 27 days = 100_800 blocks
			kusama_runtime::System::set_block_number(101_000);
			for _i in 0..29 {
				kusama_runtime::Staking::trigger_new_era(0, vec![]);
			}

			assert_eq!(
				kusama_runtime::Balances::free_balance(&subaccount_0.clone()),
				2 * dollar::<Runtime>(RelayCurrencyId::get())
			);

			// 1ksm is locked for half bonded and half unbonding.
			assert_eq!(
				kusama_runtime::Balances::usable_balance(&subaccount_0.clone()),
				dollar::<Runtime>(RelayCurrencyId::get())
			);
		});

		Bifrost::execute_with(|| {
			let subaccount_0_32: [u8; 32] =
				Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

			let subaccount_0_location: MultiLocation =
				Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

			System::set_block_number(1200);

			// set ongoing era to be 11 which is greater than due era 10.
			assert_ok!(Slp::update_ongoing_time_unit(
				Origin::root(),
				RelayCurrencyId::get(),
				TimeUnit::Era(11)
			));

			assert_ok!(Slp::liquidize(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location.clone()),
				Some(TimeUnit::SlashingSpan(5)),
				None
			));

			assert_eq!(
				Slp::get_delegator_ledger(RelayCurrencyId::get(), subaccount_0_location.clone()),
				Some(Ledger::Substrate(SubstrateLedger {
					account: subaccount_0_location.clone(),
					total: dollar::<Runtime>(RelayCurrencyId::get()),
					active: 500_000_000_000,
					unlocking: vec![UnlockChunk {
						value: 500_000_000_000,
						unlock_time: TimeUnit::Era(10)
					}]
				}))
			);

			// Check the existence of the query in pallet_xcm Queries storage.
			assert_eq!(
				PolkadotXcm::query(1),
				Some(QueryStatus::Pending {
					responder: VersionedMultiLocation::V1(MultiLocation {
						parents: 1,
						interior: Here
					}),
					maybe_notify: None,
					timeout: 2200
				})
			);

			// Check the existence of query in the response update queue storage.
			assert_eq!(
				Slp::get_delegator_ledger_update_entry(1),
				Some((
					LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
						currency_id: RelayCurrencyId::get(),
						delegator_id: subaccount_0_location.clone(),
						update_operation: SubstrateLedgerUpdateOperation::Liquidize,
						amount: 0,
						unlock_time: Some(TimeUnit::Era(11))
					}),
					2200
				))
			);
		});

		Bifrost::execute_with(|| {
			let subaccount_0_32: [u8; 32] =
				Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

			let subaccount_0_location: MultiLocation =
				Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

			// Call confirm_delegator_ledger_query_response.
			assert_ok!(Slp::confirm_delegator_ledger_query_response(
				Origin::root(),
				RelayCurrencyId::get(),
				1
			));

			// Check the ledger update.
			assert_eq!(
				Slp::get_delegator_ledger(RelayCurrencyId::get(), subaccount_0_location.clone()),
				Some(Ledger::Substrate(SubstrateLedger {
					account: subaccount_0_location.clone(),
					total: 500_000_000_000,
					active: 500_000_000_000,
					unlocking: vec![]
				}))
			);

			// Check the existence of the query in pallet_xcm Queries storage.
			// If xcm version 3 is introduced. We'll add instruction ReportTransactStatus into the
			// xcm message. And this query will be set to ready status after we received a query
			// response. At that point, this check should be set to equal None.
			assert_eq!(
				PolkadotXcm::query(1),
				Some(QueryStatus::Pending {
					responder: VersionedMultiLocation::V1(MultiLocation {
						parents: 1,
						interior: Here
					}),
					maybe_notify: None,
					timeout: 2200
				})
			);

			// Check the inexistence of query in the response update queue storage.
			assert_eq!(Slp::get_delegator_ledger_update_entry(1), None);
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Balances::free_balance(&subaccount_0.clone()),
				2 * dollar::<Runtime>(RelayCurrencyId::get())
			);

			// half of 1ksm unlocking has been freed. So the usable balance should be 1.5 ksm
			assert_eq!(
				kusama_runtime::Balances::usable_balance(&subaccount_0.clone()),
				1_500_000_000_000
			);
		});
	})
}

#[test]
fn fail_delegator_ledger_query_response_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		register_subaccount_index_0();
		transfer_2_ksm_to_subaccount_in_kusama();
		let subaccount_0 = subaccount_0();

		Bifrost::execute_with(|| {
			let subaccount_0_32: [u8; 32] =
				Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

			let subaccount_0_location: MultiLocation =
				Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

			// First call bond function, it will insert a query.
			// Bond 1 ksm for sub-account index 0
			assert_ok!(Slp::bond(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location.clone()),
				dollar::<Runtime>(RelayCurrencyId::get()),
				None
			));

			// Check the existence of the query in pallet_xcm Queries storage.
			assert_eq!(
				PolkadotXcm::query(0),
				Some(QueryStatus::Pending {
					responder: VersionedMultiLocation::V1(MultiLocation {
						parents: 1,
						interior: Here
					}),
					maybe_notify: None,
					timeout: 1600
				})
			);

			// Check the existence of query in the response update queue storage.
			assert_eq!(
				Slp::get_delegator_ledger_update_entry(0),
				Some((
					LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
						currency_id: RelayCurrencyId::get(),
						delegator_id: subaccount_0_location.clone(),
						update_operation: SubstrateLedgerUpdateOperation::Bond,
						amount: dollar::<Runtime>(RelayCurrencyId::get()),
						unlock_time: None
					}),
					1600
				))
			);
		});

		Bifrost::execute_with(|| {
			let subaccount_0_32: [u8; 32] =
				Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();

			let subaccount_0_location: MultiLocation =
				Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

			// Call confirm_delegator_ledger_query_response.
			assert_ok!(Slp::fail_delegator_ledger_query_response(
				Origin::root(),
				RelayCurrencyId::get(),
				0
			));

			// Check the ledger update.
			assert_eq!(
				Slp::get_delegator_ledger(RelayCurrencyId::get(), subaccount_0_location.clone()),
				Some(Ledger::Substrate(SubstrateLedger {
					account: subaccount_0_location.clone(),
					total: 0,
					active: 0,
					unlocking: vec![]
				}))
			);

			// Check the existence of the query in pallet_xcm Queries storage.
			// If xcm version 3 is introduced. We'll add instruction ReportTransactStatus into the
			// xcm message. And this query will be set to ready status after we received a query
			// response. At that point, this check should be set to equal None.
			assert_eq!(
				PolkadotXcm::query(0),
				Some(QueryStatus::Pending {
					responder: VersionedMultiLocation::V1(MultiLocation {
						parents: 1,
						interior: Here
					}),
					maybe_notify: None,
					timeout: 1600
				})
			);

			// Check the inexistence of query in the response update queue storage.
			assert_eq!(Slp::get_delegator_ledger_update_entry(0), None);
		});
	})
}

#[test]
fn confirm_validators_by_delegator_query_response_with_delegate_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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
			let mut valis = vec![];

			let validator_0_32: [u8; 32] =
				Slp::account_id_to_account_32(validator_0.clone()).unwrap();
			let validator_0_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_0_32).unwrap();
			targets.push(validator_0_location.clone());
			let multi_hash_0 =
				<Runtime as frame_system::Config>::Hashing::hash(&validator_0_location.encode());

			let validator_1_32: [u8; 32] =
				Slp::account_id_to_account_32(validator_1.clone()).unwrap();
			let validator_1_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_1_32).unwrap();
			targets.push(validator_1_location.clone());
			let multi_hash_1 =
				<Runtime as frame_system::Config>::Hashing::hash(&validator_1_location.encode());

			valis.push((validator_1_location.clone(), multi_hash_1));
			valis.push((validator_0_location.clone(), multi_hash_0));

			// delegate
			assert_ok!(Slp::delegate(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location.clone()),
				targets.clone(),
			));

			// Before data: Delegate nobody.
			assert_eq!(
				Slp::get_validators_by_delegator(
					RelayCurrencyId::get(),
					subaccount_0_location.clone()
				),
				None
			);

			assert_eq!(
				Slp::get_validators_by_delegator_update_entry(0),
				Some((
					ValidatorsByDelegatorUpdateEntry::Substrate(
						SubstrateValidatorsByDelegatorUpdateEntry {
							currency_id: RelayCurrencyId::get(),
							delegator_id: subaccount_0_location.clone(),
							validators: valis.clone(),
						}
					),
					1600
				))
			);

			// confirm call
			assert_ok!(Slp::confirm_validators_by_delegator_query_response(
				Origin::root(),
				RelayCurrencyId::get(),
				0
			));

			// After delegation data.
			assert_eq!(
				Slp::get_validators_by_delegator(
					RelayCurrencyId::get(),
					subaccount_0_location.clone()
				),
				Some(valis)
			);

			assert_eq!(Slp::get_validators_by_delegator_update_entry(0), None);
		});
	})
}

#[test]
fn confirm_validators_by_delegator_query_response_with_undelegate_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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
			let mut valis_1 = vec![];
			let mut valis_2 = vec![];

			let validator_0_32: [u8; 32] =
				Slp::account_id_to_account_32(validator_0.clone()).unwrap();
			let validator_0_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_0_32).unwrap();
			targets.push(validator_0_location.clone());
			let multi_hash_0 =
				<Runtime as frame_system::Config>::Hashing::hash(&validator_0_location.encode());

			let validator_1_32: [u8; 32] =
				Slp::account_id_to_account_32(validator_1.clone()).unwrap();
			let validator_1_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_1_32).unwrap();
			let multi_hash_1 =
				<Runtime as frame_system::Config>::Hashing::hash(&validator_1_location.encode());

			valis_1.push((validator_1_location.clone(), multi_hash_1.clone()));

			valis_2.push((validator_1_location.clone(), multi_hash_1));
			valis_2.push((validator_0_location.clone(), multi_hash_0));

			// Undelegate validator 0. Only validator 1 left.
			assert_ok!(Slp::undelegate(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location.clone()),
				targets.clone(),
			));

			// Before data: Delegate 2 validators.
			assert_eq!(
				Slp::get_validators_by_delegator(
					RelayCurrencyId::get(),
					subaccount_0_location.clone()
				),
				Some(valis_2)
			);

			assert_eq!(
				Slp::get_validators_by_delegator_update_entry(1),
				Some((
					ValidatorsByDelegatorUpdateEntry::Substrate(
						SubstrateValidatorsByDelegatorUpdateEntry {
							currency_id: RelayCurrencyId::get(),
							delegator_id: subaccount_0_location.clone(),
							validators: valis_1.clone(),
						}
					),
					1600
				))
			);

			// confirm call
			assert_ok!(Slp::confirm_validators_by_delegator_query_response(
				Origin::root(),
				RelayCurrencyId::get(),
				1,
			));

			// After delegation data: delegate only 1 validator.
			assert_eq!(
				Slp::get_validators_by_delegator(
					RelayCurrencyId::get(),
					subaccount_0_location.clone()
				),
				Some(valis_1)
			);

			assert_eq!(Slp::get_validators_by_delegator_update_entry(1), None);
		});
	})
}

#[test]
fn confirm_validators_by_delegator_query_response_with_redelegate_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		confirm_validators_by_delegator_query_response_with_undelegate_works();

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
			let mut valis_1 = vec![];
			let mut valis_2 = vec![];

			let validator_0_32: [u8; 32] =
				Slp::account_id_to_account_32(validator_0.clone()).unwrap();
			let validator_0_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_0_32).unwrap();
			targets.push(validator_0_location.clone());
			let multi_hash_0 =
				<Runtime as frame_system::Config>::Hashing::hash(&validator_0_location.encode());

			let validator_1_32: [u8; 32] =
				Slp::account_id_to_account_32(validator_1.clone()).unwrap();
			let validator_1_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_1_32).unwrap();
			targets.push(validator_1_location.clone());
			let multi_hash_1 =
				<Runtime as frame_system::Config>::Hashing::hash(&validator_1_location.encode());

			valis_1.push((validator_1_location.clone(), multi_hash_1.clone()));
			valis_2.push((validator_1_location.clone(), multi_hash_1));
			valis_2.push((validator_0_location.clone(), multi_hash_0));

			// Redelegate to a set of validator_0 and validator_1.
			assert_ok!(Slp::redelegate(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location.clone()),
				Some(targets.clone()),
			));

			// Before data: Delegate only 1 validator.
			assert_eq!(
				Slp::get_validators_by_delegator(
					RelayCurrencyId::get(),
					subaccount_0_location.clone()
				),
				Some(valis_1)
			);

			assert_eq!(
				Slp::get_validators_by_delegator_update_entry(2),
				Some((
					ValidatorsByDelegatorUpdateEntry::Substrate(
						SubstrateValidatorsByDelegatorUpdateEntry {
							currency_id: RelayCurrencyId::get(),
							delegator_id: subaccount_0_location.clone(),
							validators: valis_2.clone(),
						}
					),
					1600
				))
			);

			// confirm call
			assert_ok!(Slp::confirm_validators_by_delegator_query_response(
				Origin::root(),
				RelayCurrencyId::get(),
				2,
			));

			// After delegation data: delegate 2 validators.
			assert_eq!(
				Slp::get_validators_by_delegator(
					RelayCurrencyId::get(),
					subaccount_0_location.clone()
				),
				Some(valis_2)
			);

			assert_eq!(Slp::get_validators_by_delegator_update_entry(2), None);
		});
	})
}

#[test]
fn fail_validators_by_delegator_query_response_works() {
	sp_io::TestExternalities::default().execute_with(|| {
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
			let mut valis = vec![];

			let validator_0_32: [u8; 32] =
				Slp::account_id_to_account_32(validator_0.clone()).unwrap();
			let validator_0_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_0_32).unwrap();
			targets.push(validator_0_location.clone());
			let multi_hash_0 =
				<Runtime as frame_system::Config>::Hashing::hash(&validator_0_location.encode());

			let validator_1_32: [u8; 32] =
				Slp::account_id_to_account_32(validator_1.clone()).unwrap();
			let validator_1_location: MultiLocation =
				Slp::account_32_to_parent_location(validator_1_32).unwrap();
			targets.push(validator_1_location.clone());
			let multi_hash_1 =
				<Runtime as frame_system::Config>::Hashing::hash(&validator_1_location.encode());

			valis.push((validator_1_location.clone(), multi_hash_1));
			valis.push((validator_0_location.clone(), multi_hash_0));

			// delegate
			assert_ok!(Slp::delegate(
				Origin::root(),
				RelayCurrencyId::get(),
				Box::new(subaccount_0_location.clone()),
				targets.clone(),
			));

			// check before data: delegate nobody.
			assert_eq!(
				Slp::get_validators_by_delegator(
					RelayCurrencyId::get(),
					subaccount_0_location.clone()
				),
				None
			);

			assert_eq!(
				Slp::get_validators_by_delegator_update_entry(0),
				Some((
					ValidatorsByDelegatorUpdateEntry::Substrate(
						SubstrateValidatorsByDelegatorUpdateEntry {
							currency_id: RelayCurrencyId::get(),
							delegator_id: subaccount_0_location.clone(),
							validators: valis,
						}
					),
					1600
				))
			);

			// call fail function
			assert_ok!(Slp::fail_validators_by_delegator_query_response(
				Origin::root(),
				RelayCurrencyId::get(),
				0,
			));

			// check after data
			assert_eq!(
				Slp::get_validators_by_delegator(
					RelayCurrencyId::get(),
					subaccount_0_location.clone()
				),
				None
			);

			assert_eq!(Slp::get_validators_by_delegator_update_entry(0), None);
		});
	})
}
