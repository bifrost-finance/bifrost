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

//! Cross-chain transfer tests within Kusama network.

use bifrost_kusama_runtime::{
	Balances, Currencies, NativeCurrencyId, RelayCurrencyId, Runtime, RuntimeOrigin, Slp, Tokens,
	VtokenMinting, XcmDestWeightAndFeeHandler,
};
use bifrost_primitives::{TimeUnit, XcmOperationType as XcmOperation, KSM, VKSM};
use bifrost_slp::{primitives::UnlockChunk, Delays, Ledger, MinimumsMaximums, SubstrateLedger};
use frame_support::{assert_ok, BoundedVec};
use integration_tests_common::{impls::AccountId, BifrostKusama, Kusama};
use orml_traits::MultiCurrency;
use sp_runtime::Permill;
use xcm::{prelude::*, v3::Weight, VersionedMultiAssets, VersionedMultiLocation};
use xcm_emulator::{assert_expected_events, Chain, ParaId, TestExt};

pub const ALICE: [u8; 32] =
	hex_literal::hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"];
pub const BOB: [u8; 32] =
	hex_literal::hex!["8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48"];
pub const KUSAMA_ALICE_STASH_ACCOUNT: [u8; 32] =
	hex_literal::hex!["be5ddb1579b72e84524fc29e78609e3caf42e85aa118ebfe0b0ad404b5bdd25f"];
pub const KUSAMA_BOB_STASH_ACCOUNT: [u8; 32] =
	hex_literal::hex!["fe65717dad0447d715f660a0a58411de509b42e6efb8375f562f58a554d5860e"];
const ENTRANCE_ACCOUNT: [u8; 32] =
	hex_literal::hex!["6d6f646c62662f76746b696e0000000000000000000000000000000000000000"];
const BIFROST_TREASURY_ACCOUNT: [u8; 32] =
	hex_literal::hex!["6d6f646c62662f74727372790000000000000000000000000000000000000000"];
const KSM_DELEGATOR_0_ACCOUNT: [u8; 32] =
	hex_literal::hex!["5a53736d8e96f1c007cf0d630acf5209b20611617af23ce924c8e25328eb5d28"];

const EXIT_ACCOUNT: [u8; 32] =
	hex_literal::hex!["6d6f646c62662f76746f75740000000000000000000000000000000000000000"];

const BIFROST_TREASURY_MULTILOCATION: MultiLocation = MultiLocation {
	parents: 0,
	interior: X1(AccountId32 { network: None, id: BIFROST_TREASURY_ACCOUNT }),
};
const KSM_DELEGATOR_0_MULTILOCATION: MultiLocation = MultiLocation {
	parents: 1,
	interior: X1(AccountId32 { network: None, id: KSM_DELEGATOR_0_ACCOUNT }),
};

const ENTRANCE_ACCOUNT_MULTILOCATION: MultiLocation =
	MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: ENTRANCE_ACCOUNT }) };

const EXIT_ACCOUNT_MULTILOCATION: MultiLocation =
	MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: EXIT_ACCOUNT }) };

const KUSAMA_ALICE_MULTILOCATION: MultiLocation =
	MultiLocation { parents: 1, interior: X1(AccountId32 { network: None, id: ALICE }) };
const KUSAMA_ALICE_STASH_MULTILOCATION: MultiLocation = MultiLocation {
	parents: 1,
	interior: X1(AccountId32 { network: None, id: KUSAMA_ALICE_STASH_ACCOUNT }),
};
const KUSAMA_BOB_STASH_MULTILOCATION: MultiLocation = MultiLocation {
	parents: 1,
	interior: X1(AccountId32 { network: None, id: KUSAMA_BOB_STASH_ACCOUNT }),
};

const KSM_DECIMALS: u128 = 1_000_000_000_000;
const BNC_DECIMALS: u128 = 1_000_000_000_000;

/// ****************************************************
/// *********  Preparation section  ********************
/// ****************************************************

// Preparation: register sub-account index 0.
fn slp_setup() {
	cross_ksm_to_bifrost(BIFROST_TREASURY_ACCOUNT, 10000 * KSM_DECIMALS);
	// cross_ksm_to_bifrost(ENTRANCE_ACCOUNT, 10000 * KSM_DECIMALS);
	cross_ksm_to_bifrost(ALICE, 10000 * KSM_DECIMALS);
	cross_ksm_to_bifrost(BOB, 10000 * KSM_DECIMALS);

	Kusama::execute_with(|| {
		assert_ok!(kusama_runtime::Balances::force_set_balance(
			kusama_runtime::RuntimeOrigin::root(),
			sp_runtime::MultiAddress::Id(AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
			10000 * KSM_DECIMALS
		));
		kusama_runtime::System::reset_events();
	});

	BifrostKusama::execute_with(|| {
		assert_ok!(Balances::force_set_balance(
			RuntimeOrigin::root(),
			sp_runtime::MultiAddress::Id(AccountId::from(BIFROST_TREASURY_ACCOUNT)),
			10000 * BNC_DECIMALS
		));
		assert_ok!(Tokens::set_balance(
			RuntimeOrigin::root(),
			sp_runtime::MultiAddress::Id(AccountId::from(ALICE)),
			KSM,
			10000 * KSM_DECIMALS,
			0
		));
	});

	BifrostKusama::execute_with(|| {
		// set operate origin to be ALICE for vksm
		assert_ok!(Slp::set_operate_origin(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			Some(AccountId::from(ALICE))
		));
		// Set OngoingTimeUnitUpdateInterval as 1/3 Era(1800 blocks per Era, 12 seconds per
		// block)
		assert_ok!(Slp::set_ongoing_time_unit_update_interval(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			Some(1)
		));
		// Initialize ongoing timeunit as 0.
		assert_ok!(Slp::update_ongoing_time_unit(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			TimeUnit::Era(0)
		));
		assert_ok!(Slp::set_ongoing_time_unit_update_interval(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			Some(600)
		));

		// set fee_source for ksm to be treasury
		assert_ok!(Slp::set_fee_source(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			Some((BIFROST_TREASURY_MULTILOCATION, 1 * KSM_DECIMALS))
		));
		// set fee_source for ksm to be treasury
		assert_ok!(Slp::set_fee_source(
			RuntimeOrigin::root(),
			NativeCurrencyId::get(),
			Some((BIFROST_TREASURY_MULTILOCATION, 1 * BNC_DECIMALS))
		));

		let mins_and_maxs = MinimumsMaximums {
			delegator_bonded_minimum: KSM_DECIMALS / 10,
			bond_extra_minimum: KSM_DECIMALS / 1000,
			unbond_minimum: KSM_DECIMALS / 1000,
			rebond_minimum: KSM_DECIMALS / 1000,
			unbond_record_maximum: 32,
			validators_back_maximum: 24,
			delegator_active_staking_maximum: 80000 * KSM_DECIMALS,
			validators_reward_maximum: 256,
			delegation_amount_minimum: KSM_DECIMALS / 1000,
			delegators_maximum: 100,
			validators_maximum: 300,
		};

		// Set minimums and maximums
		assert_ok!(Slp::set_minimums_and_maximums(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			Some(mins_and_maxs)
		));

		// Initialize currency delays.
		let delay =
			Delays { unlock_delay: TimeUnit::Era(0), leave_delegators_delay: TimeUnit::Era(0) };
		assert_ok!(Slp::set_currency_delays(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			Some(delay)
		));

		assert_ok!(Slp::set_hosting_fees(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			Some((Permill::from_parts(1000), BIFROST_TREASURY_MULTILOCATION))
		));

		assert_ok!(Slp::set_currency_tune_exchange_rate_limit(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			Some((10, Permill::from_parts(1000)))
		));

		// add Alice and Bob to validators
		assert_ok!(Slp::add_validator(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			Box::new(KUSAMA_ALICE_STASH_MULTILOCATION),
		));
		assert_ok!(Slp::add_validator(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			Box::new(KUSAMA_BOB_STASH_MULTILOCATION),
		));

		// Register Operation weight and fee
		assert_ok!(
			<Runtime as bifrost_slp::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
				RelayCurrencyId::get(),
				XcmOperation::TransferTo,
				Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
			)
		);

		assert_ok!(
			<Runtime as bifrost_slp::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
				RelayCurrencyId::get(),
				XcmOperation::Bond,
				Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
			)
		);

		assert_ok!(
			<Runtime as bifrost_slp::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
				RelayCurrencyId::get(),
				XcmOperation::BondExtra,
				Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
			)
		);

		assert_ok!(
			<Runtime as bifrost_slp::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
				RelayCurrencyId::get(),
				XcmOperation::Unbond,
				Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
			)
		);

		assert_ok!(
			<Runtime as bifrost_slp::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
				RelayCurrencyId::get(),
				XcmOperation::Rebond,
				Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
			)
		);

		assert_ok!(
			<Runtime as bifrost_slp::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
				RelayCurrencyId::get(),
				XcmOperation::Delegate,
				Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
			)
		);

		assert_ok!(
			<Runtime as bifrost_slp::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
				RelayCurrencyId::get(),
				XcmOperation::Payout,
				Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
			)
		);

		assert_ok!(
			<Runtime as bifrost_slp::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
				RelayCurrencyId::get(),
				XcmOperation::Liquidize,
				Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
			)
		);

		assert_ok!(
			<Runtime as bifrost_slp::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
				RelayCurrencyId::get(),
				XcmOperation::Chill,
				Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
			)
		);

		assert_ok!(
			<Runtime as bifrost_slp::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
				RelayCurrencyId::get(),
				XcmOperation::TransferBack,
				Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
			)
		);

		// initialize two delegators
		assert_ok!(Slp::initialize_delegator(RuntimeOrigin::root(), RelayCurrencyId::get(), None));

		bifrost_kusama_runtime::System::reset_events();
	});
}

fn cross_ksm_to_bifrost(to: [u8; 32], amount: u128) {
	Kusama::execute_with(|| {
		assert_ok!(kusama_runtime::Balances::force_set_balance(
			kusama_runtime::RuntimeOrigin::root(),
			sp_runtime::MultiAddress::Id(AccountId::from(to)),
			amount
		));
		assert_ok!(kusama_runtime::XcmPallet::reserve_transfer_assets(
			kusama_runtime::RuntimeOrigin::signed(to.into()),
			Box::new(VersionedMultiLocation::V3(X1(Parachain(2001)).into())),
			Box::new(VersionedMultiLocation::V3(
				X1(Junction::AccountId32 { id: to, network: None }).into()
			)),
			Box::new(VersionedMultiAssets::V3((Here, amount / 10).into())),
			0,
		));
	});
}

#[test]
fn vtoken_minting() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();
		BifrostKusama::execute_with(|| {
			assert_eq!(Currencies::free_balance(VKSM, &AccountId::from(ALICE)), 0);
			assert_eq!(
				Currencies::free_balance(KSM, &AccountId::from(ALICE)),
				10000 * KSM_DECIMALS
			);
			assert_ok!(VtokenMinting::mint(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				KSM,
				100 * KSM_DECIMALS,
				BoundedVec::default(),
				None
			));
			// alice account should have 99.9 vKSM
			assert_eq!(Currencies::free_balance(VKSM, &AccountId::from(ALICE)), 100 * KSM_DECIMALS);
			assert_eq!(
				Currencies::free_balance(KSM, &AccountId::from(ENTRANCE_ACCOUNT)),
				100 * KSM_DECIMALS
			);
			assert_eq!(Currencies::free_balance(KSM, &AccountId::from(ALICE)), 9900 * KSM_DECIMALS);
		})
	});
}

#[test]
fn transfer_to() {
	slp_setup();

	Kusama::execute_with(|| {
		assert_eq!(
			kusama_runtime::Balances::free_balance(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
			10000 * KSM_DECIMALS
		);
	});

	BifrostKusama::execute_with(|| {
		// Bond 50 ksm for sub-account index 0
		assert_ok!(VtokenMinting::mint(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			KSM,
			100 * KSM_DECIMALS,
			BoundedVec::default(),
			None
		));

		assert_ok!(Slp::transfer_to(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(ENTRANCE_ACCOUNT_MULTILOCATION),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			50 * KSM_DECIMALS,
		));

		assert_eq!(
			Currencies::free_balance(KSM, &AccountId::from(ENTRANCE_ACCOUNT)),
			50 * KSM_DECIMALS
		);
	});

	Kusama::execute_with(|| {
		assert_eq!(
			kusama_runtime::Balances::free_balance(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
			10049999921672574
		);
	});
}

#[test]
fn transfer_back() {
	slp_setup();
	Kusama::execute_with(|| {
		assert_eq!(
			kusama_runtime::Balances::free_balance(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
			10000 * KSM_DECIMALS
		);
	});

	BifrostKusama::execute_with(|| {
		// Bond 50 ksm for sub-account index 0
		assert_eq!(Currencies::free_balance(KSM, &AccountId::from(EXIT_ACCOUNT)), 0);

		assert_ok!(Slp::transfer_back(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			Box::new(EXIT_ACCOUNT_MULTILOCATION),
			50 * KSM_DECIMALS,
			None
		));
	});

	Kusama::execute_with(|| {
		assert_eq!(
			kusama_runtime::Balances::free_balance(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
			9949998630000371
		);
	});

	BifrostKusama::execute_with(|| {
		assert_eq!(Currencies::free_balance(KSM, &AccountId::from(EXIT_ACCOUNT)), 49999919630000);
	});
}

#[test]
fn bond_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();

		BifrostKusama::execute_with(|| {
			// Bond 50 ksm for sub-account index 0
			assert_ok!(Slp::bond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
				None,
				None,
			));
		});

		Kusama::execute_with(|| {
			type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
			assert_expected_events!(
				Kusama,
				vec![
					RuntimeEvent::Staking(pallet_staking::Event::Bonded { stash, amount}) => {
						stash: *stash == AccountId::from(KSM_DELEGATOR_0_ACCOUNT),
						amount:  *amount == 50 * KSM_DECIMALS,
					},
				]
			);
		});

		BifrostKusama::execute_with(|| {
			type RuntimeEvent = <BifrostKusama as Chain>::RuntimeEvent;
			// Bond 50 ksm and auto confirm
			assert_expected_events!(
				BifrostKusama,
				vec![
					// Amount to reserve transfer is transferred to System Parachain's Sovereign account
					RuntimeEvent::Slp(bifrost_slp::Event::DelegatorLedgerQueryResponseConfirmed {..}) => { },
				]
			);
			assert_eq!(
				Slp::get_delegator_ledger(RelayCurrencyId::get(), KSM_DELEGATOR_0_MULTILOCATION),
				Some(Ledger::Substrate(SubstrateLedger {
					account: KSM_DELEGATOR_0_MULTILOCATION,
					total: 50 * KSM_DECIMALS,
					active: 50 * KSM_DECIMALS,
					unlocking: vec![],
				}))
			);
		});
	})
}

#[test]
fn bond_extra_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();

		BifrostKusama::execute_with(|| {
			assert_ok!(Slp::bond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
				None,
				None,
			));
		});
		BifrostKusama::execute_with(|| {
			// Bond_extra 20 ksm for sub-account index 0
			assert_ok!(Slp::bond_extra(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				None,
				20 * KSM_DECIMALS,
				None
			));
		});

		Kusama::execute_with(|| {
			type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
			assert_expected_events!(
				Kusama,
				vec![
					RuntimeEvent::Staking(pallet_staking::Event::Bonded { stash, amount}) => {
						stash: *stash == AccountId::from(KSM_DELEGATOR_0_ACCOUNT),
						amount:  *amount == 20 * KSM_DECIMALS,
					},
				]
			);
		});

		BifrostKusama::execute_with(|| {
			type RuntimeEvent = <BifrostKusama as Chain>::RuntimeEvent;
			// Bond 20 ksm and auto confirm
			assert_expected_events!(
				BifrostKusama,
				vec![
					RuntimeEvent::Slp(bifrost_slp::Event::DelegatorLedgerQueryResponseConfirmed {..}) => { },
				]
			);
			assert_eq!(
				Slp::get_delegator_ledger(RelayCurrencyId::get(), KSM_DELEGATOR_0_MULTILOCATION),
				Some(Ledger::Substrate(SubstrateLedger {
					account: KSM_DELEGATOR_0_MULTILOCATION,
					total: 70 * KSM_DECIMALS,
					active: 70 * KSM_DECIMALS,
					unlocking: vec![],
				}))
			);
		});
	})
}

#[test]
fn unbond_works() {
	slp_setup();

	BifrostKusama::execute_with(|| {
		assert_ok!(Slp::bond(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			50 * KSM_DECIMALS,
			None,
			None,
		));
	});

	Kusama::execute_with(|| {});

	BifrostKusama::execute_with(|| {
		assert_ok!(Slp::unbond(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			None,
			20 * KSM_DECIMALS,
			None
		));
	});

	Kusama::execute_with(|| {
		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			Kusama,
			vec![
				RuntimeEvent::Staking(pallet_staking::Event::Unbonded { stash, amount}) => {
					stash: *stash == AccountId::from(KSM_DELEGATOR_0_ACCOUNT),
					amount:  *amount == 20 * KSM_DECIMALS,
				},
			]
		);
	});

	BifrostKusama::execute_with(|| {
		type RuntimeEvent = <BifrostKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			BifrostKusama,
			vec![
				// Amount to reserve transfer is transferred to System Parachain's Sovereign account
				RuntimeEvent::Slp(bifrost_slp::Event::DelegatorLedgerQueryResponseConfirmed {..}) => { },
			]
		);
		assert_eq!(
			Slp::get_delegator_ledger(RelayCurrencyId::get(), KSM_DELEGATOR_0_MULTILOCATION),
			Some(Ledger::Substrate(SubstrateLedger {
				account: KSM_DELEGATOR_0_MULTILOCATION,
				total: 50 * KSM_DECIMALS,
				active: 30 * KSM_DECIMALS,
				unlocking: vec![UnlockChunk {
					value: 20 * KSM_DECIMALS,
					unlock_time: TimeUnit::Era(0)
				}],
			}))
		);
	});
}

#[test]
fn unbond_all_works() {
	slp_setup();

	BifrostKusama::execute_with(|| {
		// Unbond 0.5 ksm, 0.5 ksm left.
		assert_ok!(Slp::bond(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			50 * KSM_DECIMALS,
			None,
			None,
		));
	});

	Kusama::execute_with(|| {});

	BifrostKusama::execute_with(|| {
		assert_ok!(Slp::unbond_all(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			None
		));
	});

	Kusama::execute_with(|| {
		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			Kusama,
			vec![
				RuntimeEvent::Staking(pallet_staking::Event::Unbonded { stash, amount}) => {
					stash: *stash == AccountId::from(KSM_DELEGATOR_0_ACCOUNT),
					amount:  *amount == 20 * KSM_DECIMALS,
				},
			]
		);
	});

	BifrostKusama::execute_with(|| {
		// Bond 70 ksm and auto confirm
		assert_eq!(
			Slp::get_delegator_ledger(RelayCurrencyId::get(), KSM_DELEGATOR_0_MULTILOCATION),
			Some(Ledger::Substrate(SubstrateLedger {
				account: KSM_DELEGATOR_0_MULTILOCATION,
				total: 50 * KSM_DECIMALS,
				active: 0,
				unlocking: vec![UnlockChunk {
					value: 50 * KSM_DECIMALS,
					unlock_time: TimeUnit::Era(0)
				}],
			}))
		);
	});
}

#[test]
fn rebond_works() {
	slp_setup();

	BifrostKusama::execute_with(|| {
		assert_ok!(Slp::bond(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			50 * KSM_DECIMALS,
			None,
			None,
		));
	});

	Kusama::execute_with(|| {
		// TODO: Assert events;
	});

	BifrostKusama::execute_with(|| {
		assert_ok!(Slp::unbond(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			None,
			30 * KSM_DECIMALS,
			None
		));
	});

	Kusama::execute_with(|| {
		// TODO: Assert events;
	});

	BifrostKusama::execute_with(|| {
		// rebond 0.5 ksm.
		assert_ok!(Slp::rebond(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			None,
			Some(20 * KSM_DECIMALS),
			None
		));
	});

	// So the bonded amount should be 1 ksm
	Kusama::execute_with(|| {
		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			Kusama,
			vec![
				RuntimeEvent::Staking(pallet_staking::Event::Bonded { stash, amount}) => {
					stash: *stash == AccountId::from(KSM_DELEGATOR_0_ACCOUNT),
					amount:  *amount == 20 * KSM_DECIMALS,
				},
			]
		);
	});

	BifrostKusama::execute_with(|| {
		// Bond 70 ksm and auto confirm
		assert_eq!(
			Slp::get_delegator_ledger(RelayCurrencyId::get(), KSM_DELEGATOR_0_MULTILOCATION),
			Some(Ledger::Substrate(SubstrateLedger {
				account: KSM_DELEGATOR_0_MULTILOCATION,
				total: 50 * KSM_DECIMALS,
				active: 40 * KSM_DECIMALS,
				unlocking: vec![UnlockChunk {
					value: 10 * KSM_DECIMALS,
					unlock_time: TimeUnit::Era(0)
				}],
			}))
		);
	});
}

#[test]
fn delegate_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		// bond 1 ksm for sub-account index 0
		slp_setup();

		Kusama::execute_with(|| {
			use kusama_runtime::System;
			System::reset_events();
		});

		BifrostKusama::execute_with(|| {
			// Unbond 0.5 ksm, 0.5 ksm left.
			assert_ok!(VtokenMinting::mint(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				KSM,
				100 * KSM_DECIMALS,
				BoundedVec::default(),
				None
			));

			assert_ok!(Slp::bond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
				None,
				None,
			));
		});

		Kusama::execute_with(|| {});

		BifrostKusama::execute_with(|| {
			// delegate
			assert_ok!(Slp::delegate(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				vec![KUSAMA_ALICE_STASH_MULTILOCATION, KUSAMA_BOB_STASH_MULTILOCATION],
				None
			));
		});

		Kusama::execute_with(|| {
			Kusama::assert_ump_queue_processed(true, Some(ParaId::new(2001)), None);
		});
	})
}

#[test]
fn undelegate_works() {
	slp_setup();

	BifrostKusama::execute_with(|| {
		assert_ok!(Slp::bond(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			50 * KSM_DECIMALS,
			None,
			None,
		));
	});

	Kusama::execute_with(|| {
		// TODO: Assert events;
	});

	BifrostKusama::execute_with(|| {
		assert_ok!(Slp::delegate(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			vec![KUSAMA_ALICE_STASH_MULTILOCATION, KUSAMA_BOB_STASH_MULTILOCATION],
			None
		));
	});

	Kusama::execute_with(|| {
		// TODO: Assert events;
	});

	BifrostKusama::execute_with(|| {
		// Undelegate validator 0. Only validator 1 left.
		assert_ok!(Slp::undelegate(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			vec![KUSAMA_ALICE_STASH_MULTILOCATION],
			None
		));
	});

	Kusama::execute_with(|| {
		Kusama::assert_ump_queue_processed(true, Some(ParaId::new(2001)), None);
	});
}

#[test]
fn redelegate_works() {
	slp_setup();

	BifrostKusama::execute_with(|| {
		assert_ok!(Slp::bond(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			50 * KSM_DECIMALS,
			None,
			None,
		));
	});

	BifrostKusama::execute_with(|| {
		assert_ok!(Slp::delegate(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			vec![KUSAMA_ALICE_STASH_MULTILOCATION, KUSAMA_BOB_STASH_MULTILOCATION],
			None
		));
	});

	Kusama::execute_with(|| {
		// TODO: Assert events;
	});

	BifrostKusama::execute_with(|| {
		// Undelegate validator 0. Only validator 1 left.
		assert_ok!(Slp::undelegate(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			vec![KUSAMA_ALICE_STASH_MULTILOCATION],
			None
		));
	});

	Kusama::execute_with(|| {
		// TODO: Assert events;
	});

	BifrostKusama::execute_with(|| {
		assert_ok!(Slp::redelegate(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			Some(vec![KUSAMA_ALICE_STASH_MULTILOCATION, KUSAMA_BOB_STASH_MULTILOCATION]),
			None
		));
	});

	Kusama::execute_with(|| {
		Kusama::assert_ump_queue_processed(true, Some(ParaId::new(2001)), None);
	});
}

#[test]
fn payout_works() {
	slp_setup();
	BifrostKusama::execute_with(|| {
		// Bond 1 ksm for sub-account index 0
		assert_ok!(Slp::payout(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			Box::new(KUSAMA_ALICE_STASH_MULTILOCATION),
			Some(TimeUnit::Era(27)),
			None
		));
	});
}

#[test]
fn liquidize_works() {
	slp_setup();

	BifrostKusama::execute_with(|| {
		assert_ok!(Slp::bond(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			50 * KSM_DECIMALS,
			None,
			None,
		));
	});

	Kusama::execute_with(|| {
		// TODO: Assert events;
	});

	BifrostKusama::execute_with(|| {
		assert_ok!(Slp::delegate(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			vec![KUSAMA_ALICE_STASH_MULTILOCATION, KUSAMA_BOB_STASH_MULTILOCATION],
			None
		));
	});

	Kusama::execute_with(|| {
		// TODO: Assert events;
	});

	BifrostKusama::execute_with(|| {
		assert_ok!(Slp::unbond(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			None,
			20 * KSM_DECIMALS,
			None
		));
	});

	Kusama::execute_with(|| {
		// TODO: Assert events;
	});

	BifrostKusama::execute_with(|| {
		// Bond 70 ksm and auto confirm
		assert_eq!(
			Slp::get_delegator_ledger(RelayCurrencyId::get(), KSM_DELEGATOR_0_MULTILOCATION),
			Some(Ledger::Substrate(SubstrateLedger {
				account: KSM_DELEGATOR_0_MULTILOCATION,
				total: 50 * KSM_DECIMALS,
				active: 30 * KSM_DECIMALS,
				unlocking: vec![UnlockChunk {
					value: 20 * KSM_DECIMALS,
					unlock_time: TimeUnit::Era(0)
				}],
			}))
		);
	});

	BifrostKusama::execute_with(|| {
		assert_ok!(Slp::liquidize(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			RelayCurrencyId::get(),
			Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			Some(TimeUnit::SlashingSpan(5)),
			None,
			None,
			None
		));
	});

	Kusama::execute_with(|| {
		Kusama::assert_ump_queue_processed(true, Some(ParaId::new(2001)), None);
	});

	BifrostKusama::execute_with(|| {
		assert_eq!(
			Slp::get_delegator_ledger(RelayCurrencyId::get(), KSM_DELEGATOR_0_MULTILOCATION),
			Some(Ledger::Substrate(SubstrateLedger {
				account: KSM_DELEGATOR_0_MULTILOCATION,
				total: 30 * KSM_DECIMALS,
				active: 30 * KSM_DECIMALS,
				unlocking: vec![],
			}))
		);
	});
}

#[test]
fn chill_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();

		BifrostKusama::execute_with(|| {
			assert_ok!(Slp::bond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
				None,
				None
			));
		});

		BifrostKusama::execute_with(|| {
			assert_ok!(Slp::delegate(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				vec![KUSAMA_ALICE_STASH_MULTILOCATION, KUSAMA_BOB_STASH_MULTILOCATION],
				None
			));
		});

		// check if sub-account index 0 belongs to the group of nominators
		Kusama::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::ledger(AccountId::from(KSM_DELEGATOR_0_ACCOUNT).into())
					.is_ok(),
				true
			);
		});

		BifrostKusama::execute_with(|| {
			assert_ok!(Slp::chill(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				None
			));
		});

		// check if sub-account index 0 belongs to the group of nominators
		Kusama::execute_with(|| {});
	})
}

#[test]
fn supplement_fee_reserve_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();
		Kusama::execute_with(|| {
			assert_eq!(
				kusama_runtime::Balances::free_balance(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
				10000 * KSM_DECIMALS
			);
		});

		BifrostKusama::execute_with(|| {
			assert_ok!(Slp::supplement_fee_reserve(
				RuntimeOrigin::root(),
				RelayCurrencyId::get(),
				Box::new(KUSAMA_ALICE_MULTILOCATION),
			));
		});

		Kusama::execute_with(|| {
			assert_eq!(
				kusama_runtime::Balances::free_balance(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
				10000 * KSM_DECIMALS
			);
		});
	})
}
