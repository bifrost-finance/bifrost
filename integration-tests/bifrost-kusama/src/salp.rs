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

use crate::{kusama_integration_tests::*, kusama_test_net::*};
use bifrost_asset_registry::AssetIdMaps;
use bifrost_kusama_runtime::{LeasePeriod, MinContribution, Runtime};
use bifrost_salp::{Error, FundInfo, FundStatus};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use xcm_emulator::TestExt;

const KSM: u128 = 1_000_000_000_000;

#[test]
fn set_confirm_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::set_multisig_confirm_account(
				RawOrigin::Root.into(),
				AccountId::new(BOB)
			));
			assert_eq!(Salp::multisig_confirm_account(), Some(AccountId::new(BOB)),);
		});
	})
}

#[test]
fn create_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_eq!(AssetIdMaps::<Runtime>::check_token_registered(TokenSymbol::KSM), true);
			assert_eq!(
				AssetIdMaps::<Runtime>::check_vsbond_registered(TokenSymbol::KSM, 3000, 1, 8),
				false
			);
			// first_slot + 7 >= last_slot
			assert_ok!(Salp::create(
				RawOrigin::Root.into(),
				//paraid
				3_000,
				//cap
				100 * KSM,
				//first_slot
				1,
				//last_slot
				SlotLength::get()
			));
			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: Zero::zero(),
					cap: 100 * KSM,
					first_slot: 1,
					last_slot: SlotLength::get(),
					trie_index: 0,
					status: FundStatus::Ongoing,
				}
			);
			assert_eq!(
				AssetIdMaps::<Runtime>::check_vsbond_registered(TokenSymbol::KSM, 3000, 1, 8),
				true
			);
		});
	})
}

#[test]
fn edit_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(
				RawOrigin::Root.into(),
				3_000,
				100 * KSM,
				1,
				SlotLength::get()
			));

			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: Zero::zero(),
					cap: 100 * KSM,
					first_slot: 1,
					last_slot: SlotLength::get(),
					trie_index: 0,
					status: FundStatus::Ongoing,
				}
			);

			assert_ok!(Salp::edit(
				RawOrigin::Root.into(),
				//paraid
				3_000,
				//cap
				1000 * KSM,
				//raised
				150,
				//first_slot
				2,
				//last_slot
				SlotLength::get() + 1,
				Some(FundStatus::Ongoing)
			));

			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: 150,
					cap: 1000 * KSM,
					first_slot: 2,
					last_slot: 9,
					trie_index: 0,
					status: FundStatus::Ongoing,
				}
			);
		});
	})
}

#[test]
fn contribute_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(
				RawOrigin::Root.into(),
				3_000,
				100 * KSM,
				1,
				SlotLength::get()
			));
			//MinContribution 0.1 KSM
			assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 1 * KSM));
			//7200 1 day
			let (contributed, status) = Salp::contribution(0, &AccountId::from(BOB));
			assert_eq!(contributed, 0);
			assert_eq!(status.is_contributing(), true);
			assert_eq!(status.contributing(), 1 * KSM);
			//check free balance 9 ksm, should reserve 1 ksm
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::Token(TokenSymbol::KSM),
					&AccountId::from(BOB)
				),
				9 * KSM
			);

			assert_ok!(Salp::confirm_contribute(
				Origin::signed(AccountId::new(ALICE)),
				sp_runtime::AccountId32::from(BOB),
				3_000,
				true,
				CONTRIBUTON_INDEX
			));

			//check free balance 9 ksm
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::Token(TokenSymbol::KSM),
					&AccountId::from(BOB)
				),
				9 * KSM
			);

			//check free balance 9 ksm
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::Token(TokenSymbol::KSM),
					&AccountId::from(BOB)
				),
				9 * KSM
			);

			//check free balance 1 vsksm
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::VSToken(TokenSymbol::KSM),
					&AccountId::from(BOB)
				),
				1 * KSM
			);
			//check free balance 1 vsbondksm
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::VSBond(TokenSymbol::KSM, 3000, 1, 8),
					&AccountId::from(BOB)
				),
				1 * KSM
			);

			let (contributed, status) = Salp::contribution(0, &sp_runtime::AccountId32::from(BOB));
			assert_eq!(contributed, 1 * KSM);
			assert_eq!(status.is_contributing(), false);
			assert_eq!(status.contributing(), 0);

			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: 1 * KSM,
					cap: 100 * KSM,
					first_slot: 1,
					last_slot: 8,
					trie_index: 0,
					status: FundStatus::Ongoing,
				}
			);
		});
	})
}

#[test]
fn double_contribute_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(RawOrigin::Root.into(), 3_000, 5 * KSM, 1, SlotLength::get()));
			assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 1 * KSM));

			assert_ok!(Salp::confirm_contribute(
				Origin::signed(AccountId::new(ALICE)),
				AccountId::new(BOB),
				3_000,
				true,
				CONTRIBUTON_INDEX
			));

			assert_noop!(
				Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 5 * KSM),
				Error::<Runtime>::CapExceeded
			);

			assert_noop!(
				Salp::contribute(
					Origin::signed(AccountId::new(BOB)),
					3_000,
					MinContribution::get() - 1
				),
				Error::<Runtime>::ContributionTooSmall
			);

			assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 2 * KSM));

			let (contributed, status) = Salp::contribution(0, &sp_runtime::AccountId32::from(BOB));
			assert_eq!(contributed, 1 * KSM);
			assert_eq!(status.is_contributing(), true);
			assert_eq!(status.contributing(), 2 * KSM);

			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: 1 * KSM,
					cap: 5 * KSM,
					first_slot: 1,
					last_slot: 8,
					trie_index: 0,
					status: FundStatus::Ongoing,
				}
			);
		});
	})
}

#[test]
fn fund_fail_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(RawOrigin::Root.into(), 3_000, 5 * KSM, 1, SlotLength::get()));
			// crownload is failed, so enable the withdrawal function of vsToken/vsBond
			assert_ok!(Salp::fund_fail(RawOrigin::Root.into(), 3_000,));
			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: 0,
					cap: 5 * KSM,
					first_slot: 1,
					last_slot: 8,
					trie_index: 0,
					status: FundStatus::Failed,
				}
			);
		});
	})
}

#[test]
fn fund_retire_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(RawOrigin::Root.into(), 3_000, 10 * KSM, 1, SlotLength::get()));
			//Failed/Retired
			assert_ok!(Salp::fund_success(RawOrigin::Root.into(), 3_000));
			assert_ok!(Salp::fund_retire(RawOrigin::Root.into(), 3_000));
			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: 0,
					cap: 10 * KSM,
					first_slot: 1,
					last_slot: 8,
					trie_index: 0,
					status: FundStatus::Retired,
				}
			);
		});
	})
}

#[test]
fn fund_retire_withdraw_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(RawOrigin::Root.into(), 3_000, 10 * KSM, 1, SlotLength::get()));
			assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 1 * KSM));
			assert_ok!(Salp::confirm_contribute(
				Origin::signed(AccountId::new(ALICE)),
				AccountId::new(BOB),
				3_000,
				true,
				CONTRIBUTON_INDEX
			));
			//Failed/Retired
			assert_ok!(Salp::fund_success(RawOrigin::Root.into(), 3_000));
			assert_ok!(Salp::fund_retire(RawOrigin::Root.into(), 3_000));
			assert_ok!(Salp::withdraw(RawOrigin::Root.into(), 3_000));

			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: 1 * KSM,
					cap: 10 * KSM,
					first_slot: 1,
					last_slot: 8,
					trie_index: 0,
					status: FundStatus::RedeemWithdrew,
				}
			);
			assert_eq!(Salp::redeem_pool(), 1 * KSM);
		});
	})
}

#[test]
fn fund_fail_withdraw_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(RawOrigin::Root.into(), 3_000, 10 * KSM, 1, SlotLength::get()));
			assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 1 * KSM));
			assert_ok!(Salp::confirm_contribute(
				Origin::signed(AccountId::new(ALICE)),
				AccountId::new(BOB),
				3_000,
				true,
				CONTRIBUTON_INDEX
			));
			//Failed/Retired
			assert_ok!(Salp::fund_fail(RawOrigin::Root.into(), 3_000));
			assert_ok!(Salp::withdraw(RawOrigin::Root.into(), 3_000));

			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: 1 * KSM,
					cap: 10 * KSM,
					first_slot: 1,
					last_slot: 8,
					trie_index: 0,
					status: FundStatus::RefundWithdrew,
				}
			);
			assert_eq!(Salp::redeem_pool(), 1 * KSM);
		});
	})
}

#[test]
fn continue_fund_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(RawOrigin::Root.into(), 3_000, 10 * KSM, 1, SlotLength::get()));
			assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 1 * KSM));
			assert_ok!(Salp::confirm_contribute(
				Origin::signed(AccountId::new(ALICE)),
				AccountId::new(BOB),
				3_000,
				true,
				CONTRIBUTON_INDEX
			));
			//Failed/Retired
			assert_ok!(Salp::fund_fail(RawOrigin::Root.into(), 3_000));
			assert_ok!(Salp::withdraw(RawOrigin::Root.into(), 3_000));

			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: 1 * KSM,
					cap: 10 * KSM,
					first_slot: 1,
					last_slot: 8,
					trie_index: 0,
					status: FundStatus::RefundWithdrew,
				}
			);
			assert_eq!(Salp::redeem_pool(), 1 * KSM);

			assert_ok!(Salp::continue_fund(RawOrigin::Root.into(), 3_000, 2, 9));
			assert_eq!(
				Salp::failed_funds_to_refund((3_000, 1, 8)).unwrap(),
				FundInfo {
					raised: 1 * KSM,
					cap: 10 * KSM,
					first_slot: 1,
					last_slot: 8,
					trie_index: 0,
					status: FundStatus::FailedToContinue,
				}
			);
			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: 1 * KSM,
					cap: 10 * KSM,
					first_slot: 2,
					last_slot: 9,
					trie_index: 0,
					status: FundStatus::Ongoing,
				}
			);
		});
	})
}

#[test]
fn refund_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(RawOrigin::Root.into(), 3_000, 10 * KSM, 1, SlotLength::get()));
			assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 1 * KSM));
			assert_ok!(Salp::confirm_contribute(
				Origin::signed(AccountId::new(ALICE)),
				AccountId::new(BOB),
				3_000,
				true,
				CONTRIBUTON_INDEX
			));
			//check free balance 1 vsksm
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::VSToken(TokenSymbol::KSM),
					&AccountId::from(BOB)
				),
				1 * KSM
			);
			//check free balance 1 vsbondksm
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::VSBond(TokenSymbol::KSM, 3000, 1, 8),
					&AccountId::from(BOB)
				),
				1 * KSM
			);
			assert_ok!(Salp::fund_fail(RawOrigin::Root.into(), 3_000));
			assert_ok!(Salp::withdraw(RawOrigin::Root.into(), 3_000));
			//fund_fail-> withdraw -> RefundWithdrew
			assert_ok!(Salp::refund(
				Origin::signed(AccountId::new(BOB)),
				3_000,
				1,
				SlotLength::get(),
				KSM / 2
			));

			//check free balance 0.5 vsksm
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::VSToken(TokenSymbol::KSM),
					&AccountId::from(BOB)
				),
				KSM / 2
			);
			//check free balance 0.5 vsbondksm
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::VSBond(TokenSymbol::KSM, 3000, 1, 8),
					&AccountId::from(BOB)
				),
				KSM / 2
			);
			//check free balance 0.5 ksm
			assert_eq!(Salp::redeem_pool(), KSM / 2);

			//check funds
			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: 1 * KSM - KSM / 2,
					cap: 10 * KSM,
					first_slot: 1,
					last_slot: 8,
					trie_index: 0,
					status: FundStatus::RefundWithdrew,
				}
			);

			//check free balance 9.5 ksm
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::Token(TokenSymbol::KSM),
					&AccountId::from(BOB)
				),
				9 * KSM + KSM / 2
			);
		});
	})
}

#[test]
fn redeem_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(RawOrigin::Root.into(), 3_000, 10 * KSM, 1, SlotLength::get()));
			assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 1 * KSM));
			assert_ok!(Salp::confirm_contribute(
				Origin::signed(AccountId::new(ALICE)),
				AccountId::new(BOB),
				3_000,
				true,
				CONTRIBUTON_INDEX
			));
			//Failed/Retired
			assert_ok!(Salp::fund_success(RawOrigin::Root.into(), 3_000));
			assert_ok!(Salp::fund_retire(RawOrigin::Root.into(), 3_000));
			assert_ok!(Salp::withdraw(RawOrigin::Root.into(), 3_000));

			//check free balance 1 vsksm
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::VSToken(TokenSymbol::KSM),
					&AccountId::from(BOB)
				),
				1 * KSM
			);
			//check free balance 1 vsbondksm
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::VSBond(TokenSymbol::KSM, 3000, 1, 8),
					&AccountId::from(BOB)
				),
				1 * KSM
			);

			assert_ok!(Salp::redeem(Origin::signed(AccountId::new(BOB)), 3_000, KSM / 2));

			//check free balance 0.5 ksm
			assert_eq!(Salp::redeem_pool(), KSM / 2);

			//check funds
			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: 1 * KSM - KSM / 2,
					cap: 10 * KSM,
					first_slot: 1,
					last_slot: 8,
					trie_index: 0,
					status: FundStatus::RedeemWithdrew,
				}
			);
			//check free balance 9.5 ksm
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::Token(TokenSymbol::KSM),
					&AccountId::from(BOB)
				),
				9 * KSM + KSM / 2
			);
		});
	})
}

#[test]
fn redeem_with_speical_vsbond_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(RawOrigin::Root.into(), 2001, 1000_000_000_000, 13, 20));
			assert_ok!(Salp::contribute(
				Origin::signed(AccountId::new(BOB)),
				2001,
				100_000_000_000
			));
			assert_ok!(Salp::confirm_contribute(
				Origin::signed(AccountId::new(ALICE)),
				AccountId::new(BOB),
				2001,
				true,
				CONTRIBUTON_INDEX
			));

			assert_ok!(Salp::fund_success(RawOrigin::Root.into(), 2001));
			assert_ok!(Salp::unlock(
				Origin::signed(AccountId::new(ALICE)),
				AccountId::new(BOB),
				2001
			));

			// Mock the BlockNumber
			let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
			System::set_block_number(block_begin_redeem);

			assert_ok!(Salp::fund_retire(RawOrigin::Root.into(), 2001));
			assert_ok!(Salp::withdraw(RawOrigin::Root.into(), 2001));

			let vs_token =
				<Runtime as bifrost_salp::Config>::CurrencyIdConversion::convert_to_vstoken(
					RelayCurrencyId::get(),
				)
				.unwrap();
			let vs_bond =
				<Runtime as bifrost_salp::Config>::CurrencyIdConversion::convert_to_vsbond(
					RelayCurrencyId::get(),
					2001,
					13,
					20,
				)
				.unwrap();

			assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(
				vs_token,
				&AccountId::new(BOB),
				&AccountId::new(CATHI),
				500_000_000
			));
			assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(
				vs_bond,
				&AccountId::new(BOB),
				&AccountId::new(CATHI),
				500_000_000
			));
			assert_ok!(Salp::redeem(Origin::signed(AccountId::new(BOB)), 2001, 500_000_000));
			assert_ok!(Salp::redeem(Origin::signed(AccountId::new(CATHI)), 2001, 500_000_000));
		});
	})
}

#[test]
fn dissolve_refunded_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(RawOrigin::Root.into(), 3_000, 10 * KSM, 1, SlotLength::get()));
			assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 1 * KSM));
			assert_ok!(Salp::confirm_contribute(
				Origin::signed(AccountId::new(ALICE)),
				AccountId::new(BOB),
				3_000,
				true,
				CONTRIBUTON_INDEX
			));
			//Failed/Retired
			assert_ok!(Salp::fund_fail(RawOrigin::Root.into(), 3_000));
			assert_ok!(Salp::withdraw(RawOrigin::Root.into(), 3_000));

			assert_eq!(
				Salp::funds(3_000).unwrap(),
				FundInfo {
					raised: 1 * KSM,
					cap: 10 * KSM,
					first_slot: 1,
					last_slot: 8,
					trie_index: 0,
					status: FundStatus::RefundWithdrew,
				}
			);
			assert_eq!(Salp::redeem_pool(), 1 * KSM);

			assert_ok!(Salp::continue_fund(RawOrigin::Root.into(), 3_000, 2, 9));
			assert_eq!(
				Salp::failed_funds_to_refund((3_000, 1, 8)).unwrap(),
				FundInfo {
					raised: 1 * KSM,
					cap: 10 * KSM,
					first_slot: 1,
					last_slot: 8,
					trie_index: 0,
					status: FundStatus::FailedToContinue,
				}
			);

			assert_ok!(Salp::dissolve_refunded(RawOrigin::Root.into(), 3_000, 1, 8));

			assert_eq!(Salp::failed_funds_to_refund((3_000, 1, 8)), None);
		});
	})
}

#[test]
fn dissolve_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(
				RawOrigin::Root.into(),
				3_000,
				1000_000_000_000,
				1,
				SlotLength::get()
			));
			assert_ok!(Salp::contribute(
				Origin::signed(AccountId::new(BOB)),
				3_000,
				100_000_000_000
			));
			assert_ok!(Salp::confirm_contribute(
				Origin::signed(AccountId::new(ALICE)),
				AccountId::new(BOB),
				3_000,
				true,
				CONTRIBUTON_INDEX
			));
			assert_ok!(Salp::fund_success(RawOrigin::Root.into(), 3_000));
			assert_ok!(Salp::fund_retire(RawOrigin::Root.into(), 3_000));
			assert_ok!(Salp::withdraw(RawOrigin::Root.into(), 3_000));
			assert_ok!(Salp::fund_end(RawOrigin::Root.into(), 3_000));

			assert_ok!(Salp::dissolve(RawOrigin::Root.into(), 3_000));

			assert!(Salp::funds(3_000).is_none());
		});
	})
}

#[test]
fn batch_unlock_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(
				RawOrigin::Root.into(),
				3_000,
				1000_000_000_000,
				1,
				SlotLength::get()
			));
			assert_ok!(Salp::contribute(
				Origin::signed(AccountId::new(BOB)),
				3_000,
				100_000_000_000
			));
			assert_ok!(Salp::confirm_contribute(
				Origin::signed(AccountId::new(ALICE)),
				AccountId::new(BOB),
				3_000,
				true,
				CONTRIBUTON_INDEX
			));

			assert_ok!(Salp::fund_success(RawOrigin::Root.into(), 3_000));
			assert_ok!(Salp::batch_unlock(Origin::signed(AccountId::new(ALICE)), 3_000));
		})
	})
}

#[test]
fn unlock_when_fund_ongoing_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		SalpTest::execute_with(|| {
			assert_ok!(Salp::create(
				RawOrigin::Root.into(),
				3_000,
				1000_000_000_000,
				1,
				SlotLength::get()
			));
			assert_ok!(Salp::contribute(
				Origin::signed(AccountId::new(BOB)),
				3_000,
				100_000_000_000
			));
			assert_ok!(Salp::confirm_contribute(
				Origin::signed(AccountId::new(ALICE)),
				AccountId::new(BOB),
				3_000,
				true,
				CONTRIBUTON_INDEX
			));
			assert_ok!(Salp::unlock(
				Origin::signed(AccountId::new(BOB)),
				AccountId::new(BOB),
				3_000
			));
		});
	})
}
