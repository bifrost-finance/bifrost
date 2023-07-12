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

/*

fail_validators_by_delegator_query_response
confirm_validators_by_delegator_query_response
fail_delegator_ledger_query_response
confirm_delegator_ledger_query_response

remove_supplement_fee_account_from_whitelist
add_supplement_fee_account_to_whitelist
supplement_fee_reserve

set_ongoing_time_unit_update_interval
update_ongoing_time_unit

set_currency_tune_exchange_rate_limit
set_hosting_fees
set_currency_delays
set_minimums_and_maximums

set_delegator_ledger
set_validators_by_delegator
set_fee_source
set_operate_origin
set_xcm_dest_weight_and_fee

remove_validator
add_validator

initialize_delegator
remove_delegator
add_delegator

charge_host_fee_and_tune_vtoken_exchange_rate
refund_currency_due_unbond

decrease_token_pool
increase_token_pool

transfer_to
transfer_back

chill
liquidize
payout

redelegate
undelegate
delegate

rebond
unbond_all
unbond
bond_extra
bond


bond/unbond/bond_extra/unbond_all/rebond/chill/liquidize  confirm_delegator_ledger_query_response
delegate/undelegate/redelegate  confirm_validators_by_delegator_query_response

*/

#![cfg(test)]
use bifrost_kusama_runtime::{NativeCurrencyId, VtokenMinting};
use bifrost_slp::{
	primitives::UnlockChunk, Delays, Ledger, MinimumsMaximums, SubstrateLedger, XcmOperation,
};
use frame_support::{assert_ok, BoundedVec};
use node_primitives::TimeUnit;
use orml_traits::MultiCurrency;
use pallet_staking::{Nominations, StakingLedger};
use sp_runtime::Permill;
use xcm::{prelude::*, v3::Weight, VersionedMultiAssets, VersionedMultiLocation};
use xcm_emulator::TestExt;

use crate::{kusama_integration_tests::*, kusama_test_net::*};

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

/// ****************************************************
/// *********  Preparation section  ********************
/// ****************************************************

// Preparation: register sub-account index 0.
fn slp_setup() {
	cross_ksm_to_bifrost(BIFROST_TREASURY_ACCOUNT, 10000 * KSM_DECIMALS);
	// cross_ksm_to_bifrost(ENTRANCE_ACCOUNT, 10000 * KSM_DECIMALS);
	cross_ksm_to_bifrost(ALICE, 10000 * KSM_DECIMALS);
	cross_ksm_to_bifrost(BOB, 10000 * KSM_DECIMALS);

	KusamaNet::execute_with(|| {
		assert_ok!(kusama_runtime::Balances::force_set_balance(
			kusama_runtime::RuntimeOrigin::root(),
			sp_runtime::MultiAddress::Id(AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
			10000 * KSM_DECIMALS
		));
	});

	Bifrost::execute_with(|| {
		assert_ok!(Balances::force_set_balance(
			RuntimeOrigin::root(),
			sp_runtime::MultiAddress::Id(AccountId::from(BIFROST_TREASURY_ACCOUNT)),
			10000 * BNC_DECIMALS
		));
	});

	vksm_vtoken_minting_setup();

	Bifrost::execute_with(|| {
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
		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			XcmOperation::TransferTo,
			Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			XcmOperation::Bond,
			Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			XcmOperation::BondExtra,
			Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			XcmOperation::Unbond,
			Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			XcmOperation::Rebond,
			Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			XcmOperation::Delegate,
			Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			XcmOperation::Payout,
			Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			XcmOperation::Liquidize,
			Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			XcmOperation::Chill,
			Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
		));

		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			XcmOperation::TransferBack,
			Some((Weight::from_parts(10000000000, 1000000), 10_000_000_000)),
		));

		// initialize two delegators
		assert_ok!(Slp::initialize_delegator(RuntimeOrigin::root(), RelayCurrencyId::get(), None));
	});
}

fn vksm_vtoken_minting_setup() {
	Bifrost::execute_with(|| {
		// Set the vtoken-minting mint and redeem fee rate to 0.1% with origin root. This is for all
		// tokens, not just vksm.
		assert_ok!(VtokenMinting::set_fees(
			RuntimeOrigin::root(),
			Permill::from_parts(1000),
			Permill::from_parts(1000),
		));
		// set the number of fast-redeem user unlocking records to be 10 per block. This is for all
		// tokens, not just vksm.
		assert_ok!(VtokenMinting::set_hook_iteration_limit(RuntimeOrigin::root(), 10));
		// set vksm unlock duration to be 28 eras
		assert_ok!(VtokenMinting::set_unlock_duration(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			TimeUnit::Era(28)
		));
		// set vksm minimum mint amount to be 1 KSM
		assert_ok!(VtokenMinting::set_minimum_mint(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			1 * KSM_DECIMALS
		));
		// set vksm minimum redeem amount to be 0.1 KSM
		assert_ok!(VtokenMinting::set_minimum_redeem(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			KSM_DECIMALS / 10
		));
		// add vksm to be a supported rebond token
		assert_ok!(VtokenMinting::add_support_rebond_token(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
		));
		// set vksm starting fast-redeem timeunit to be era 0
		assert_ok!(VtokenMinting::set_min_time_unit(
			RuntimeOrigin::root(),
			RelayCurrencyId::get(),
			TimeUnit::Era(0)
		));
	})
}

fn cross_ksm_to_bifrost(to: [u8; 32], amount: u128) {
	KusamaNet::execute_with(|| {
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
			Box::new(VersionedMultiAssets::V3((Here, amount).into())),
			0,
		));
	});
}

#[test]
fn vtoken_minting() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();
		Bifrost::execute_with(|| {
			println!(
				"{:?}",
				Currencies::free_balance(
					CurrencyId::VToken(TokenSymbol::KSM),
					&AccountId::from(ALICE)
				)
			);
			println!(
				"{:?}",
				Currencies::free_balance(
					CurrencyId::Token(TokenSymbol::KSM),
					&AccountId::from(ALICE)
				)
			);
			assert_ok!(VtokenMinting::mint(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				CurrencyId::Token(TokenSymbol::KSM),
				100 * KSM_DECIMALS
			));
			// alice account should have 99.9 vKSM
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::VToken(TokenSymbol::KSM),
					&AccountId::from(ALICE)
				),
				99900000000000
			);
			// TODO : entrance_account should have 100 KSM
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::Token(TokenSymbol::KSM),
					&AccountId::from(ENTRANCE_ACCOUNT)
				),
				99900000000000
			)
		})
	});
}

#[test]
fn transfer_to() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Balances::free_balance(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
				10000 * KSM_DECIMALS
			);
		});

		Bifrost::execute_with(|| {
			// Bond 50 ksm for sub-account index 0
			assert_ok!(VtokenMinting::mint(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				CurrencyId::Token(TokenSymbol::KSM),
				100 * KSM_DECIMALS
			));

			assert_ok!(Slp::transfer_to(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(ENTRANCE_ACCOUNT_MULTILOCATION),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
			));

			assert_eq!(
				Currencies::free_balance(
					CurrencyId::Token(TokenSymbol::KSM),
					&AccountId::from(ENTRANCE_ACCOUNT)
				),
				49900000000000
			);
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Balances::free_balance(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
				10049999909994200
			);
		});
	})
}

#[test]
fn transfer_back() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();

		KusamaNet::execute_with(|| {
			use kusama_runtime::System;
			System::reset_events();
			assert_eq!(
				kusama_runtime::Balances::free_balance(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
				10000 * KSM_DECIMALS
			);
		});

		Bifrost::execute_with(|| {
			// Bond 50 ksm for sub-account index 0
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::Token(TokenSymbol::KSM),
					&AccountId::from(EXIT_ACCOUNT)
				),
				0
			);

			assert_ok!(Slp::transfer_back(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				Box::new(EXIT_ACCOUNT_MULTILOCATION),
				50 * KSM_DECIMALS,
			));
		});

		Bifrost::execute_with(|| {
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::Token(TokenSymbol::KSM),
					&AccountId::from(EXIT_ACCOUNT)
				),
				49999929608000
			);
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Balances::free_balance(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
				9950 * KSM_DECIMALS
			);
		});
	})
}

#[test]
fn bond_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();

		Bifrost::execute_with(|| {
			// Bond 50 ksm for sub-account index 0
			assert_ok!(Slp::bond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
				None
			));
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::ledger(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
				Some(StakingLedger {
					stash: AccountId::from(KSM_DELEGATOR_0_ACCOUNT),
					total: 50 * KSM_DECIMALS,
					active: 50 * KSM_DECIMALS,
					unlocking: BoundedVec::try_from(vec![]).unwrap(),
					claimed_rewards: BoundedVec::try_from(vec![]).unwrap(),
				})
			);
		});

		Bifrost::execute_with(|| {
			// Bond 50 ksm and auto confirm
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

		KusamaNet::execute_with(|| {
			use kusama_runtime::System;
			System::reset_events();
		});

		Bifrost::execute_with(|| {
			assert_ok!(Slp::bond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
				None
			));
		});
		Bifrost::execute_with(|| {
			// Bond_extra 20 ksm for sub-account index 0
			assert_ok!(Slp::bond_extra(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				None,
				20 * KSM_DECIMALS,
			));
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::ledger(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
				Some(StakingLedger {
					stash: AccountId::from(KSM_DELEGATOR_0_ACCOUNT),
					total: 70 * KSM_DECIMALS,
					active: 70 * KSM_DECIMALS,
					unlocking: BoundedVec::try_from(vec![]).unwrap(),
					claimed_rewards: BoundedVec::try_from(vec![]).unwrap(),
				})
			);
		});

		Bifrost::execute_with(|| {
			// Bond 70 ksm and auto confirm
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
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();

		Bifrost::execute_with(|| {
			assert_ok!(Slp::bond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
				None
			));
		});

		Bifrost::execute_with(|| {
			assert_ok!(Slp::unbond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				None,
				20 * KSM_DECIMALS,
			));
		});
		// KusamaNet::execute_with(|| {
		// 	use kusama_runtime::System;
		// 	assert_eq!(
		// 		kusama_runtime::Staking::ledger(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
		// 		Some(StakingLedger {
		// 			stash: AccountId::from(KSM_DELEGATOR_0_ACCOUNT),
		// 			total: 50 * KSM_DECIMALS,
		// 			active: 30 * KSM_DECIMALS,
		// 			unlocking: _,
		// 			claimed_rewards: BoundedVec::try_from(vec![]).unwrap(),
		// 		})
		// 	);
		// });

		Bifrost::execute_with(|| {
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
	})
}

#[test]
fn unbond_all_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();

		Bifrost::execute_with(|| {
			// Unbond 0.5 ksm, 0.5 ksm left.
			assert_ok!(Slp::bond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
				None
			));
		});

		Bifrost::execute_with(|| {
			assert_ok!(Slp::unbond_all(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			));
		});

		// KusamaNet::execute_with(|| {
		// 	use kusama_runtime::System;
		// 	println!("{:?}", System::events());
		// 	assert_eq!(
		// 		kusama_runtime::Staking::ledger(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
		// 		Some(StakingLedger {
		// 			stash: AccountId::from(KSM_DELEGATOR_0_ACCOUNT),
		// 			total: 50 * KSM_DECIMALS,
		// 			active: 0,
		// 			unlocking: BoundedVec::try_from(vec![]).unwrap(),
		// 			claimed_rewards: BoundedVec::try_from(vec![]).unwrap(),
		// 		})
		// 	);
		// });

		Bifrost::execute_with(|| {
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
	})
}

#[test]
fn rebond_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();

		Bifrost::execute_with(|| {
			assert_ok!(Slp::bond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
				None
			));
		});

		Bifrost::execute_with(|| {
			assert_ok!(Slp::unbond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				None,
				30 * KSM_DECIMALS
			));
		});

		Bifrost::execute_with(|| {
			// rebond 0.5 ksm.
			assert_ok!(Slp::rebond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				None,
				Some(20 * KSM_DECIMALS),
			));
		});

		// So the bonded amount should be 1 ksm
		// KusamaNet::execute_with(|| {
		// 	assert_eq!(
		// 		kusama_runtime::Staking::ledger(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
		// 		Some(StakingLedger {
		// 			stash: AccountId::from(KSM_DELEGATOR_0_ACCOUNT),
		// 			total: 50 * KSM_DECIMALS,
		// 			active: 40 * KSM_DECIMALS,
		// 			unlocking: BoundedVec::try_from(vec![]).unwrap(),
		// 			claimed_rewards: BoundedVec::try_from(vec![]).unwrap(),
		// 		})
		// 	);
		// });
		Bifrost::execute_with(|| {
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
	})
}

#[test]
fn delegate_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		// bond 1 ksm for sub-account index 0
		slp_setup();

		KusamaNet::execute_with(|| {
			use kusama_runtime::System;
			System::reset_events();
		});

		Bifrost::execute_with(|| {
			// Unbond 0.5 ksm, 0.5 ksm left.
			assert_ok!(VtokenMinting::mint(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				CurrencyId::Token(TokenSymbol::KSM),
				100 * KSM_DECIMALS
			));

			assert_ok!(Slp::bond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
				None
			));
		});

		KusamaNet::execute_with(|| {
			use kusama_runtime::System;
			println!("{:?}", System::events());
			System::reset_events();
		});

		Bifrost::execute_with(|| {
			// delegate
			assert_ok!(Slp::delegate(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				vec![KUSAMA_ALICE_STASH_MULTILOCATION, KUSAMA_BOB_STASH_MULTILOCATION],
			));
		});

		KusamaNet::execute_with(|| {
			use kusama_runtime::System;
			println!("{:?}", System::events());
			assert_eq!(
				kusama_runtime::Staking::nominators(AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
				Some(Nominations {
					targets: BoundedVec::try_from(vec![
						KUSAMA_ALICE_STASH_ACCOUNT.into(),
						KUSAMA_BOB_STASH_ACCOUNT.into(),
					])
					.unwrap(),
					submitted_in: 0,
					suppressed: false
				})
			);
		});
	})
}

#[test]
fn undelegate_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();

		Bifrost::execute_with(|| {
			assert_ok!(Slp::bond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
				None
			));
		});

		Bifrost::execute_with(|| {
			assert_ok!(Slp::delegate(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				vec![KUSAMA_ALICE_STASH_MULTILOCATION, KUSAMA_BOB_STASH_MULTILOCATION],
			));
		});

		Bifrost::execute_with(|| {
			// Undelegate validator 0. Only validator 1 left.
			assert_ok!(Slp::undelegate(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				vec![KUSAMA_ALICE_STASH_MULTILOCATION],
			));
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::nominators(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
				Some(Nominations {
					targets: BoundedVec::try_from(vec![KUSAMA_BOB_STASH_ACCOUNT.into()]).unwrap(),
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
		slp_setup();

		Bifrost::execute_with(|| {
			assert_ok!(Slp::bond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
				None
			));
		});

		Bifrost::execute_with(|| {
			assert_ok!(Slp::delegate(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				vec![KUSAMA_ALICE_STASH_MULTILOCATION, KUSAMA_BOB_STASH_MULTILOCATION],
			));
		});

		Bifrost::execute_with(|| {
			// Undelegate validator 0. Only validator 1 left.
			assert_ok!(Slp::undelegate(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				vec![KUSAMA_ALICE_STASH_MULTILOCATION],
			));
		});

		Bifrost::execute_with(|| {
			assert_ok!(Slp::redelegate(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				Some(vec![KUSAMA_ALICE_STASH_MULTILOCATION, KUSAMA_BOB_STASH_MULTILOCATION])
			));
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::nominators(AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
				Some(Nominations {
					targets: BoundedVec::try_from(vec![
						KUSAMA_ALICE_STASH_ACCOUNT.into(),
						KUSAMA_BOB_STASH_ACCOUNT.into(),
					])
					.unwrap(),
					submitted_in: 0,
					suppressed: false
				})
			);
		});
	})
}

#[test]
fn payout_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();
		Bifrost::execute_with(|| {
			// Bond 1 ksm for sub-account index 0
			assert_ok!(Slp::payout(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				Box::new(KUSAMA_ALICE_STASH_MULTILOCATION),
				Some(TimeUnit::Era(27))
			));
		});
	})
}

#[test]
fn liquidize_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();

		Bifrost::execute_with(|| {
			assert_ok!(Slp::bond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
				None
			));
		});

		Bifrost::execute_with(|| {
			assert_ok!(Slp::delegate(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				vec![KUSAMA_ALICE_STASH_MULTILOCATION, KUSAMA_BOB_STASH_MULTILOCATION],
			));
		});

		Bifrost::execute_with(|| {
			assert_ok!(Slp::unbond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				None,
				20 * KSM_DECIMALS
			));
		});

		Bifrost::execute_with(|| {
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

		Bifrost::execute_with(|| {
			assert_ok!(Slp::liquidize(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				Some(TimeUnit::SlashingSpan(5)),
				None,
				None
			));
		});

		Bifrost::execute_with(|| {
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
	})
}

#[test]
fn chill_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();

		Bifrost::execute_with(|| {
			assert_ok!(Slp::bond(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				50 * KSM_DECIMALS,
				None
			));
		});

		Bifrost::execute_with(|| {
			assert_ok!(Slp::delegate(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
				vec![KUSAMA_ALICE_STASH_MULTILOCATION, KUSAMA_BOB_STASH_MULTILOCATION],
			));
		});

		// check if sub-account index 0 belongs to the group of nominators
		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::nominators(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT))
					.is_some(),
				true
			);
			assert_eq!(
				kusama_runtime::Staking::ledger(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT))
					.is_some(),
				true
			);
		});

		Bifrost::execute_with(|| {
			assert_ok!(Slp::chill(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				RelayCurrencyId::get(),
				Box::new(KSM_DELEGATOR_0_MULTILOCATION),
			));
		});

		// check if sub-account index 0 belongs to the group of nominators
		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Staking::nominators(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT))
					.is_some(),
				false
			);
		});
	})
}

#[test]
fn supplement_fee_reserve_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		slp_setup();
		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Balances::free_balance(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
				10000 * KSM_DECIMALS
			);
		});

		Bifrost::execute_with(|| {
			assert_ok!(Slp::supplement_fee_reserve(
				RuntimeOrigin::root(),
				RelayCurrencyId::get(),
				Box::new(KUSAMA_ALICE_MULTILOCATION),
			));
		});

		KusamaNet::execute_with(|| {
			assert_eq!(
				kusama_runtime::Balances::free_balance(&AccountId::from(KSM_DELEGATOR_0_ACCOUNT)),
				10000 * KSM_DECIMALS
			);
		});
	})
}
