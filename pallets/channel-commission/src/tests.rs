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

#![cfg(test)]

use crate::{mock::*, *};
use bifrost_primitives::{currency::KSM, BNC, VBNC, VKSM};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::AccountId32;

const CHANNEL_A_NAME: &[u8] = b"channel_a";
const CHANNEL_B_NAME: &[u8] = b"channel_b";

const CHANNEL_A_RECEIVER: AccountId = AccountId32::new([3u8; 32]);
const CHANNEL_B_RECEIVER: AccountId = AccountId32::new([4u8; 32]);
const CHANNEL_A_BACKUP_RECEIVER: AccountId = AccountId32::new([5u8; 32]);

fn setup() {
	// set commission tokens: VKSM -> KSM
	assert_ok!(ChannelCommission::set_commission_tokens(
		RuntimeOrigin::signed(ALICE),
		VKSM,
		Some(KSM),
	));

	// set commission tokens: VBNC -> BNC
	assert_ok!(ChannelCommission::set_commission_tokens(
		RuntimeOrigin::signed(ALICE),
		VBNC,
		Some(BNC),
	));

	// register channel A
	assert_ok!(ChannelCommission::register_channel(
		RuntimeOrigin::signed(ALICE),
		CHANNEL_A_NAME.to_vec(),
		CHANNEL_A_RECEIVER.clone(),
	));

	// register channel B
	assert_ok!(ChannelCommission::register_channel(
		RuntimeOrigin::signed(ALICE),
		CHANNEL_B_NAME.to_vec(),
		CHANNEL_B_RECEIVER.clone(),
	));
}

#[test]
fn set_commission_tokens_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(ChannelCommission::set_commission_tokens(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			Some(KSM),
		));

		// Channel A is registered
		assert_eq!(CommissionTokens::<Runtime>::get(VKSM), Some(KSM));

		assert_ok!(ChannelCommission::set_commission_tokens(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			None,
		));

		assert_eq!(CommissionTokens::<Runtime>::get(VKSM), None);
		assert_eq!(VtokenIssuanceSnapshots::<Runtime>::get(VKSM), Default::default());
	});
}

#[test]
fn set_commission_tokens_should_fail_with_invalid_vtoken() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_noop!(
			ChannelCommission::set_commission_tokens(RuntimeOrigin::signed(ALICE), KSM, Some(KSM)),
			Error::<Runtime>::InvalidVtoken
		);

		assert_noop!(
			ChannelCommission::set_commission_tokens(RuntimeOrigin::signed(ALICE), KSM, None),
			Error::<Runtime>::InvalidVtoken
		);
	});
}

#[test]
fn set_commission_tokens_should_fail_with_no_change() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(ChannelCommission::set_commission_tokens(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			Some(KSM),
		));

		assert_noop!(
			ChannelCommission::set_commission_tokens(RuntimeOrigin::signed(ALICE), VKSM, Some(KSM)),
			Error::<Runtime>::NoChangesMade
		);

		assert_ok!(ChannelCommission::set_commission_tokens(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			None,
		));

		assert_noop!(
			ChannelCommission::set_commission_tokens(RuntimeOrigin::signed(ALICE), VKSM, None),
			Error::<Runtime>::NoChangesMade
		);
	});
}

#[test]
fn register_channel_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		// set commission tokens: VKSM -> KSM
		assert_ok!(ChannelCommission::set_commission_tokens(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			Some(KSM),
		));

		// set commission tokens: VBNC -> BNC
		assert_ok!(ChannelCommission::set_commission_tokens(
			RuntimeOrigin::signed(ALICE),
			VBNC,
			Some(BNC),
		));

		assert_ok!(ChannelCommission::register_channel(
			RuntimeOrigin::signed(ALICE),
			CHANNEL_A_NAME.to_vec(),
			CHANNEL_A_RECEIVER.clone(),
		));

		// Channel A is registered
		assert_eq!(
			Channels::<Runtime>::get(0),
			Some((CHANNEL_A_RECEIVER, BoundedVec::try_from(CHANNEL_A_NAME.to_vec()).unwrap()))
		);

		// next channel id has been increased
		assert_eq!(ChannelNextId::<Runtime>::get(), 1);

		// ChannelCommissionTokenRates have been set for Channel A in both VKSM and VBNC
		assert_eq!(ChannelCommissionTokenRates::<Runtime>::get(0, VKSM), DEFAULT_COMMISSION_RATE);
		assert_eq!(ChannelCommissionTokenRates::<Runtime>::get(0, VBNC), DEFAULT_COMMISSION_RATE);

		// ChannelVtokenShares has been initialized for Channel A in both VKSM and VBNC
		assert_eq!(ChannelVtokenShares::<Runtime>::get(0, VKSM), Permill::from_percent(0));

		assert_eq!(ChannelVtokenShares::<Runtime>::get(0, VBNC), Permill::from_percent(0));
	});
}

#[test]
fn remove_channel_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		setup();

		// assure Channel A is registered
		assert_eq!(
			Channels::<Runtime>::get(0),
			Some((CHANNEL_A_RECEIVER, BoundedVec::try_from(CHANNEL_A_NAME.to_vec()).unwrap()))
		);

		// assure Channel A has records in ChannelCommissionTokenRates in both VKSM and VBNC
		assert_eq!(ChannelCommissionTokenRates::<Runtime>::get(0, VKSM), DEFAULT_COMMISSION_RATE);

		assert_eq!(ChannelCommissionTokenRates::<Runtime>::get(0, VBNC), DEFAULT_COMMISSION_RATE);

		// successfully remove Channel A
		assert_ok!(ChannelCommission::remove_channel(RuntimeOrigin::signed(ALICE), 0));

		// Channel A is removed
		assert_eq!(Channels::<Runtime>::get(0), None);

		// Channel A has no records in ChannelCommissionTokenRates in both VKSM and VBNC
		assert_eq!(ChannelCommissionTokenRates::<Runtime>::get(0, VKSM), Zero::zero());

		assert_eq!(ChannelCommissionTokenRates::<Runtime>::get(0, VBNC), Zero::zero());
	});
}

#[test]
fn remove_channel_should_fail_with_channel_not_exist() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_eq!(Channels::<Runtime>::get(0), None);
		assert_noop!(
			ChannelCommission::remove_channel(RuntimeOrigin::signed(ALICE), 0),
			Error::<Runtime>::ChannelNotExist
		);
	});
}

#[test]
fn update_channel_receive_account_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		setup();

		// Channel A is registered
		assert_eq!(
			Channels::<Runtime>::get(0),
			Some((CHANNEL_A_RECEIVER, BoundedVec::try_from(CHANNEL_A_NAME.to_vec()).unwrap()))
		);

		// update Channel A's receive account
		assert_ok!(ChannelCommission::update_channel_receive_account(
			RuntimeOrigin::signed(ALICE),
			0,
			CHANNEL_A_BACKUP_RECEIVER.clone(),
		));

		// Channel A's receive account is updated
		assert_eq!(
			Channels::<Runtime>::get(0),
			Some((
				CHANNEL_A_BACKUP_RECEIVER,
				BoundedVec::try_from(CHANNEL_A_NAME.to_vec()).unwrap()
			))
		);
	});
}

#[test]
fn update_channel_receive_account_should_fail_with_channel_not_exist() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_eq!(Channels::<Runtime>::get(0), None);
		assert_noop!(
			ChannelCommission::update_channel_receive_account(
				RuntimeOrigin::signed(ALICE),
				0,
				CHANNEL_A_RECEIVER,
			),
			Error::<Runtime>::ChannelNotExist
		);
	});
}

#[test]
fn update_channel_receive_account_should_fail_with_no_changes() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		setup();

		// Channel A is registered
		assert_eq!(
			Channels::<Runtime>::get(0),
			Some((CHANNEL_A_RECEIVER, BoundedVec::try_from(CHANNEL_A_NAME.to_vec()).unwrap()))
		);

		assert_noop!(
			ChannelCommission::update_channel_receive_account(
				RuntimeOrigin::signed(ALICE),
				0,
				CHANNEL_A_RECEIVER,
			),
			Error::<Runtime>::NoChangesMade
		);

		// Channel A's receive account is updated
		assert_eq!(
			Channels::<Runtime>::get(0),
			Some((CHANNEL_A_RECEIVER, BoundedVec::try_from(CHANNEL_A_NAME.to_vec()).unwrap()))
		);
	});
}

#[test]
fn set_channel_commission_token_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		setup();

		// assure Channel A has records in ChannelCommissionTokenRates in both VKSM and VBNC
		assert_eq!(ChannelCommissionTokenRates::<Runtime>::get(0, VKSM), DEFAULT_COMMISSION_RATE);

		assert_eq!(ChannelCommissionTokenRates::<Runtime>::get(0, VBNC), DEFAULT_COMMISSION_RATE);

		let new_rate = Percent::from_percent(50);
		// set commission token VKSM to 50%
		assert_ok!(ChannelCommission::set_channel_commission_token(
			RuntimeOrigin::signed(ALICE),
			0,
			VKSM,
			Percent::from_percent(50),
		));

		assert_eq!(ChannelCommissionTokenRates::<Runtime>::get(0, VKSM), new_rate);

		// set commission token VBNC to None, which means removing it
		assert_ok!(ChannelCommission::set_channel_commission_token(
			RuntimeOrigin::signed(ALICE),
			0,
			VBNC,
			Zero::zero(),
		));

		assert_eq!(ChannelCommissionTokenRates::<Runtime>::get(0, VBNC), Zero::zero());
	});
}

#[test]
fn set_channel_commission_token_should_fail_with_invalid_vtoken() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_noop!(
			ChannelCommission::set_channel_commission_token(
				RuntimeOrigin::signed(ALICE),
				0,
				KSM,
				Percent::from_percent(50),
			),
			Error::<Runtime>::InvalidVtoken
		);
	});
}

#[test]
fn set_channel_commission_token_should_fail_with_channel_not_exist() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_eq!(Channels::<Runtime>::get(0), None);
		assert_noop!(
			ChannelCommission::set_channel_commission_token(
				RuntimeOrigin::signed(ALICE),
				0,
				VKSM,
				Percent::from_percent(50),
			),
			Error::<Runtime>::ChannelNotExist
		);
	});
}

#[test]
fn set_channel_commission_token_should_fail_with_not_configure_commission() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(ChannelCommission::register_channel(
			RuntimeOrigin::signed(ALICE),
			CHANNEL_A_NAME.to_vec(),
			CHANNEL_A_RECEIVER.clone(),
		));

		assert_noop!(
			ChannelCommission::set_channel_commission_token(
				RuntimeOrigin::signed(ALICE),
				0,
				VKSM,
				Percent::from_percent(50),
			),
			Error::<Runtime>::VtokenNotConfiguredForCommission
		);
	});
}

#[test]
fn claim_commissions_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		setup();

		let commission_account: AccountId =
			<Runtime as crate::Config>::CommissionPalletId::get().into_account_truncating();
		// endow CommissionPalletId account with 1000 KSM and 1000 BNC
		assert_ok!(Currencies::deposit(KSM, &commission_account, 1000));
		assert_ok!(Currencies::deposit(BNC, &commission_account, 1000));

		// set channel A's claimable KSM amount to 100
		ChannelClaimableCommissions::<Runtime>::insert(0, KSM, 100);

		// set channel A's claimable BNC amount to 120
		ChannelClaimableCommissions::<Runtime>::insert(0, BNC, 120);

		// assure channel A's claimable KSM amount is 100
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(0, KSM), 100);

		// assure channel A's claimable BNC amount is 120
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(0, BNC), 120);

		let receiver_ksm_before = Currencies::free_balance(KSM, &CHANNEL_A_RECEIVER);
		let receiver_bnc_before = Currencies::free_balance(BNC, &CHANNEL_A_RECEIVER);

		// claim 50 KSM from channel A
		assert_ok!(ChannelCommission::claim_commissions(
			RuntimeOrigin::signed(CHANNEL_A_RECEIVER.clone()),
			0,
		));

		// assure channel A's claimable KSM amount is None
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(0, KSM), 0);

		// assure channel A's claimable BNC amount is None
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(0, BNC), 0);

		// assure channel A's receiver's KSM balance is increased by 100
		assert_eq!(Currencies::free_balance(KSM, &CHANNEL_A_RECEIVER), receiver_ksm_before + 100);

		// assure channel A's receiver's BNC balance is increased by 120
		assert_eq!(Currencies::free_balance(BNC, &CHANNEL_A_RECEIVER), receiver_bnc_before + 120);
	});
}

#[test]
fn claim_commissions_should_fail_with_channel_not_exist() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_noop!(
			ChannelCommission::claim_commissions(
				RuntimeOrigin::signed(CHANNEL_A_RECEIVER.clone()),
				0,
			),
			Error::<Runtime>::ChannelNotExist
		);
	});
}

#[test]
fn claim_commissions_should_fail_with_transfer_error() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		setup();

		ChannelClaimableCommissions::<Runtime>::insert(0, KSM, 100);
		assert_noop!(
			ChannelCommission::claim_commissions(
				RuntimeOrigin::signed(CHANNEL_A_RECEIVER.clone()),
				0,
			),
			Error::<Runtime>::TransferError
		);
	});
}

#[test]
fn channel_commission_distribution_with_net_mint_positive_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let commission_account: AccountId =
			<Runtime as crate::Config>::CommissionPalletId::get().into_account_truncating();

		// set the block number to 35
		System::set_block_number(35);

		setup();

		// test case 1: net mint is positive.(VKSM)

		// first, set storages for test case 1
		// The first round, set channel A has a share of 20%, channel B has a share of 10%. Channel
		// C has not participated yet.
		let channel_a_share = Permill::from_percent(20);
		ChannelVtokenShares::<Runtime>::insert(0, VKSM, channel_a_share);

		let channel_b_share = Permill::from_percent(10);
		ChannelVtokenShares::<Runtime>::insert(1, VKSM, channel_b_share);

		// VtokenIssuanceSnapshots, set both VKSM and VBNC old total issuance to 10000. newly minted
		// VKSM is 1000, VBNC is 1000.
		VtokenIssuanceSnapshots::<Runtime>::insert(VKSM, (9000, 10000));

		// PeriodVtokenTotalMint
		PeriodVtokenTotalMint::<Runtime>::insert(VKSM, (10000, 2000));

		// PeriodVtokenTotalRedeem
		PeriodVtokenTotalRedeem::<Runtime>::insert(VKSM, (0, 1000));

		// PeriodChannelVtokenMint. Channel A mint 1000 VKSM, Channel B mint 1000 VKSM.
		PeriodChannelVtokenMint::<Runtime>::insert(0, VKSM, (2000, 500));
		PeriodChannelVtokenMint::<Runtime>::insert(1, VKSM, (2000, 100));

		// PeriodTotalCommissions
		PeriodTotalCommissions::<Runtime>::insert(KSM, (0, 100));

		// set vksm token issuance to 11000
		let _ = Currencies::update_balance(
			RuntimeOrigin::root(),
			commission_account.clone(),
			VKSM,
			11000,
		);

		// set ksm token issuance to 11000
		let _ = Currencies::update_balance(
			RuntimeOrigin::root(),
			commission_account.clone(),
			KSM,
			11000,
		);

		// check balance of commission account
		assert_eq!(Currencies::free_balance(VKSM, &commission_account), 11000);

		// set block number to 100
		run_to_block(100);
		// set_clearing_environment already been called in block 100
		// check whether the clearing environment is set correctly for block 100
		assert_eq!(VtokenIssuanceSnapshots::<Runtime>::get(VKSM), (10000, 11000));
		assert_eq!(PeriodVtokenTotalMint::<Runtime>::get(VKSM), (2000, 0));
		assert_eq!(PeriodVtokenTotalRedeem::<Runtime>::get(VKSM), (1000, 0));
		assert_eq!(PeriodChannelVtokenMint::<Runtime>::get(0, VKSM), (500, 0));
		assert_eq!(PeriodChannelVtokenMint::<Runtime>::get(1, VKSM), (100, 0));
		assert_eq!(PeriodTotalCommissions::<Runtime>::get(KSM), (100, 0));

		// get channel B's vtoken share before being cleared
		let channel_b_vtoken_share_before = ChannelVtokenShares::<Runtime>::get(1, VKSM);

		run_to_block(101);

		let channel_a_commission = 4;
		// check channel A claimable KSM amount after being cleared
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(0, KSM), channel_a_commission);

		let channel_a_new_percentage =
			Permill::from_rational_with_rounding(2250u32, 11000u32, Rounding::Down).unwrap();
		// check channel A vtoken share after being cleared
		assert_eq!(ChannelVtokenShares::<Runtime>::get(0, VKSM), channel_a_new_percentage);

		// check channel B has not been cleared yet
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(1, KSM), 0);
		assert_eq!(ChannelVtokenShares::<Runtime>::get(1, VKSM), channel_b_vtoken_share_before);

		run_to_block(102);

		let channel_b_commission = 2;
		// check channel B claimable KSM amount after being cleared
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(1, KSM), channel_b_commission);

		let channel_b_new_percentage =
			Permill::from_rational_with_rounding(1050u32, 11000u32, Rounding::Down).unwrap();
		// check channel B vtoken share after being cleared
		assert_eq!(ChannelVtokenShares::<Runtime>::get(1, VKSM), channel_b_new_percentage);

		// check PeriodClearedCommissions, should be channel a commission + channel b commission
		assert_eq!(PeriodClearedCommissions::<Runtime>::get(KSM), 6);

		let bifrost_commission_receiver: AccountId32 =
			<Runtime as crate::Config>::BifrostCommissionReceiver::get();
		// check Bifrost commission balance before being cleared
		let bifrost_account_balance_before =
			Currencies::free_balance(KSM, &bifrost_commission_receiver);
		assert_eq!(bifrost_account_balance_before, 0);

		run_to_block(103);
		// cleared commissions should be none
		assert_eq!(PeriodClearedCommissions::<Runtime>::get(KSM), 0);

		// check Bifrost commission balance after being cleared
		let bifrost_commission_balance_after =
			Currencies::free_balance(KSM, &bifrost_commission_receiver);
		assert_eq!(bifrost_commission_balance_after, 100 - 6);
	});
}

// test case 2: net mint is negative.(VBNC)
#[test]
fn channel_commission_distribution_with_net_mint_negative_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let commission_account: AccountId =
			<Runtime as crate::Config>::CommissionPalletId::get().into_account_truncating();

		// set the block number to 35
		System::set_block_number(35);

		setup();

		// first, set storages for test case 1
		// The first round, set channel A has a share of 20%, channel B has a share of 10%. Channel
		// C has not participated yet.
		let channel_a_share = Permill::from_percent(20);
		ChannelVtokenShares::<Runtime>::insert(0, VBNC, channel_a_share);

		let channel_b_share = Permill::from_percent(10);
		ChannelVtokenShares::<Runtime>::insert(1, VBNC, channel_b_share);

		// VtokenIssuanceSnapshots, set both VBNC old total issuance to 10000. newly minted
		// VBNC is 1000.
		VtokenIssuanceSnapshots::<Runtime>::insert(VBNC, (9000, 10000));

		// PeriodVtokenTotalMint
		PeriodVtokenTotalMint::<Runtime>::insert(VBNC, (10000, 1000));

		// PeriodVtokenTotalRedeem
		PeriodVtokenTotalRedeem::<Runtime>::insert(VBNC, (0, 2000));

		// PeriodChannelVtokenMint. Channel A mint 200 VBNC, Channel B mint 100 VBNC.
		PeriodChannelVtokenMint::<Runtime>::insert(0, VBNC, (2000, 200));
		PeriodChannelVtokenMint::<Runtime>::insert(1, VBNC, (1000, 100));

		// PeriodTotalCommissions
		PeriodTotalCommissions::<Runtime>::insert(BNC, (0, 100));

		// set vbnc token issuance to 9000
		let _ = Currencies::update_balance(
			RuntimeOrigin::root(),
			commission_account.clone(),
			VBNC,
			9000,
		);

		// set bnc token issuance to 9000
		let _ = Currencies::update_balance(
			RuntimeOrigin::root(),
			commission_account.clone(),
			BNC,
			9000,
		);
		// check balance of commission account
		assert_eq!(Currencies::free_balance(VBNC, &commission_account), 9000);

		// set block number to 100
		run_to_block(100);
		// set_clearing_environment already been called in block 100
		// check whether the clearing environment is set correctly for block 100
		assert_eq!(VtokenIssuanceSnapshots::<Runtime>::get(VBNC), (10000, 9000));
		assert_eq!(PeriodVtokenTotalMint::<Runtime>::get(VBNC), (1000, 0));
		assert_eq!(PeriodVtokenTotalRedeem::<Runtime>::get(VBNC), (2000, 0));
		assert_eq!(PeriodChannelVtokenMint::<Runtime>::get(0, VBNC), (200, 0));
		assert_eq!(PeriodChannelVtokenMint::<Runtime>::get(1, VBNC), (100, 0));
		assert_eq!(PeriodTotalCommissions::<Runtime>::get(BNC), (100, 0));

		// get channel A's vtoken share before being cleared
		let channel_a_vtoken_share_before = ChannelVtokenShares::<Runtime>::get(0, VBNC);

		// get channel B's vtoken share before being cleared
		let channel_b_vtoken_share_before = ChannelVtokenShares::<Runtime>::get(1, VBNC);

		run_to_block(101);

		// Since the net mint is negative, the share of channels should not be changed.
		let channel_a_commission = 4;
		// check channel A claimable BNC amount after being cleared
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(0, BNC), channel_a_commission);

		// check channel A vtoken share after being cleared
		assert_eq!(ChannelVtokenShares::<Runtime>::get(0, VBNC), channel_a_vtoken_share_before);

		// check channel B has not been cleared yet
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(1, BNC), 0);
		assert_eq!(ChannelVtokenShares::<Runtime>::get(1, VBNC), channel_b_vtoken_share_before);

		run_to_block(102);

		// Since the net mint is negative, the share of channels should not be changed.
		let channel_b_commission = 2;
		// check channel B claimable BNC amount after being cleared
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(1, BNC), channel_b_commission);

		// check channel B vtoken share after being cleared
		assert_eq!(ChannelVtokenShares::<Runtime>::get(1, VBNC), channel_b_vtoken_share_before);

		// check PeriodClearedCommissions, should be channel a commission + channel b commission
		assert_eq!(PeriodClearedCommissions::<Runtime>::get(BNC), 6);

		let bifrost_commission_receiver: AccountId32 =
			<Runtime as crate::Config>::BifrostCommissionReceiver::get();
		// check Bifrost commission balance before being cleared
		let bifrost_account_balance_before =
			Currencies::free_balance(BNC, &bifrost_commission_receiver);
		assert_eq!(bifrost_account_balance_before, 0);

		run_to_block(103);
		// cleared commissions should be none
		assert_eq!(PeriodClearedCommissions::<Runtime>::get(BNC), 0);

		// check Bifrost commission balance after being cleared
		let bifrost_commission_balance_after =
			Currencies::free_balance(BNC, &bifrost_commission_receiver);
		assert_eq!(bifrost_commission_balance_after, 100 - 6);
	});
}

#[test]
fn set_channel_vtoken_shares_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		setup();

		// assure Channel A has no 0 percent in ChannelVtokenShares in VKSM
		assert_eq!(ChannelVtokenShares::<Runtime>::get(0, VKSM), Permill::from_percent(0));

		// set channel A's vtoken share in VKSM to 50%
		assert_ok!(ChannelCommission::set_channel_vtoken_shares(
			RuntimeOrigin::signed(ALICE),
			0,
			VKSM,
			Permill::from_percent(50),
		));

		assert_eq!(ChannelVtokenShares::<Runtime>::get(0, VKSM), Permill::from_percent(50));

		// set channel B's vtoken share in VKSM to 90%, it should fail because the sum of all shares
		// should be less than or equal to 100%
		assert_noop!(
			ChannelCommission::set_channel_vtoken_shares(
				RuntimeOrigin::signed(ALICE),
				1,
				VKSM,
				Permill::from_percent(90),
			),
			Error::<Runtime>::InvalidCommissionRate
		);

		// set channel B's vtoken share in VKSM to 30%, it should be ok now
		assert_ok!(ChannelCommission::set_channel_vtoken_shares(
			RuntimeOrigin::signed(ALICE),
			1,
			VKSM,
			Permill::from_percent(30),
		));

		assert_eq!(ChannelVtokenShares::<Runtime>::get(1, VKSM), Permill::from_percent(30));
	});
}

#[test]
fn set_channel_vtoken_shares_should_fail_with_channel_not_exist() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_noop!(
			ChannelCommission::set_channel_vtoken_shares(
				RuntimeOrigin::signed(ALICE),
				0,
				VKSM,
				Permill::from_percent(90),
			),
			Error::<Runtime>::ChannelNotExist
		);
	});
}

#[test]
fn set_channel_vtoken_shares_should_fail_with_vtoken_not_configured() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(ChannelCommission::register_channel(
			RuntimeOrigin::signed(ALICE),
			CHANNEL_A_NAME.to_vec(),
			CHANNEL_A_RECEIVER.clone(),
		));

		assert_noop!(
			ChannelCommission::set_channel_vtoken_shares(
				RuntimeOrigin::signed(ALICE),
				0,
				VKSM,
				Permill::from_percent(90),
			),
			Error::<Runtime>::VtokenNotConfiguredForCommission
		);
	});
}

// register a new channel base on some existing channels, and mint some tokens to see whether the
// shares of existing channels are updated correctly.s
#[test]
fn register_a_new_channel_and_mint_should_update_shares_and_get_claimable_tokens() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let commission_account: AccountId =
			<Runtime as crate::Config>::CommissionPalletId::get().into_account_truncating();

		// set the block number to 35
		System::set_block_number(35);

		// we have registered channel A and channel B for 0 shares for VKSM
		setup();

		// VtokenIssuanceSnapshots, set VKSM old total issuance to 10000. newly minted
		// VKSM is 1000
		VtokenIssuanceSnapshots::<Runtime>::insert(VKSM, (9000, 10000));

		// PeriodVtokenTotalMint
		PeriodVtokenTotalMint::<Runtime>::insert(VKSM, (10000, 2000));

		// PeriodVtokenTotalRedeem
		PeriodVtokenTotalRedeem::<Runtime>::insert(VKSM, (0, 1000));

		// PeriodChannelVtokenMint. Channel A mint 1000 VKSM, Channel B mint 1000 VKSM.
		PeriodChannelVtokenMint::<Runtime>::insert(0, VKSM, (2000, 500));
		PeriodChannelVtokenMint::<Runtime>::insert(1, VKSM, (2000, 100));

		// set vksm token issuance to 11000
		let _ = Currencies::update_balance(
			RuntimeOrigin::root(),
			commission_account.clone(),
			VKSM,
			11000,
		);

		// set ksm token issuance to 11000
		let _ = Currencies::update_balance(
			RuntimeOrigin::root(),
			commission_account.clone(),
			KSM,
			11000,
		);

		// set block number to 100
		run_to_block(100);
		run_to_block(101);

		let channel_a_new_net_mint: u32 = 500 * 1000 / 2000;

		let channel_a_new_percentage =
			Permill::from_rational_with_rounding(channel_a_new_net_mint, 11000u32, Rounding::Down)
				.unwrap();
		// check channel A vtoken share after being cleared
		assert_eq!(ChannelVtokenShares::<Runtime>::get(0, VKSM), channel_a_new_percentage);
	});
}

#[test]
fn on_initialize_hook_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let commission_account: AccountId =
			<Runtime as crate::Config>::CommissionPalletId::get().into_account_truncating();

		// set the block number to 35
		System::set_block_number(35);

		setup();

		// test case 1: net mint is positive.(VKSM)

		// first, set storages for test case 1
		// The first round, set channel A has a share of 20%, channel B has a share of 10%. Channel
		// C has not participated yet.
		let channel_a_share = Permill::from_percent(20);
		ChannelVtokenShares::<Runtime>::insert(0, VKSM, channel_a_share);

		let channel_b_share = Permill::from_percent(10);
		ChannelVtokenShares::<Runtime>::insert(1, VKSM, channel_b_share);

		// VtokenIssuanceSnapshots, set both VKSM and VBNC old total issuance to 10000. newly minted
		// VKSM is 1000, VBNC is 1000.
		VtokenIssuanceSnapshots::<Runtime>::insert(VKSM, (9000, 10000));

		// PeriodVtokenTotalMint
		PeriodVtokenTotalMint::<Runtime>::insert(VKSM, (10000, 2000));

		// PeriodVtokenTotalRedeem
		PeriodVtokenTotalRedeem::<Runtime>::insert(VKSM, (0, 1000));

		// PeriodChannelVtokenMint. Channel A mint 1000 VKSM, Channel B mint 1000 VKSM.
		PeriodChannelVtokenMint::<Runtime>::insert(0, VKSM, (2000, 500));
		PeriodChannelVtokenMint::<Runtime>::insert(1, VKSM, (2000, 100));

		// PeriodTotalCommissions
		PeriodTotalCommissions::<Runtime>::insert(KSM, (0, 100));

		// set vksm token issuance to 11000
		let _ = Currencies::update_balance(
			RuntimeOrigin::root(),
			commission_account.clone(),
			VKSM,
			11000,
		);

		// set ksm token issuance to 11000
		let _ = Currencies::update_balance(
			RuntimeOrigin::root(),
			commission_account.clone(),
			KSM,
			11000,
		);

		// check balance of commission account
		assert_eq!(Currencies::free_balance(VKSM, &commission_account), 11000);

		// set block number to 100
		ChannelCommission::on_initialize(100);
		// set_clearing_environment already been called in block 100
		// check whether the clearing environment is set correctly for block 100
		assert_eq!(VtokenIssuanceSnapshots::<Runtime>::get(VKSM), (10000, 11000));
		assert_eq!(PeriodVtokenTotalMint::<Runtime>::get(VKSM), (2000, 0));
		assert_eq!(PeriodVtokenTotalRedeem::<Runtime>::get(VKSM), (1000, 0));
		assert_eq!(PeriodChannelVtokenMint::<Runtime>::get(0, VKSM), (500, 0));
		assert_eq!(PeriodChannelVtokenMint::<Runtime>::get(1, VKSM), (100, 0));
		assert_eq!(PeriodTotalCommissions::<Runtime>::get(KSM), (100, 0));

		// get channel B's vtoken share before being cleared
		let channel_b_vtoken_share_before = ChannelVtokenShares::<Runtime>::get(1, VKSM);

		ChannelCommission::on_initialize(101);

		let channel_a_commission = 4;
		// check channel A claimable KSM amount after being cleared
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(0, KSM), channel_a_commission);

		let channel_a_new_percentage =
			Permill::from_rational_with_rounding(2250u32, 11000u32, Rounding::Down).unwrap();
		// check channel A vtoken share after being cleared
		assert_eq!(ChannelVtokenShares::<Runtime>::get(0, VKSM), channel_a_new_percentage);

		// check channel B has not been cleared yet
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(1, KSM), 0);
		assert_eq!(ChannelVtokenShares::<Runtime>::get(1, VKSM), channel_b_vtoken_share_before);

		ChannelCommission::on_initialize(102);

		let channel_b_commission = 2;
		// check channel B claimable KSM amount after being cleared
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(1, KSM), channel_b_commission);

		let channel_b_new_percentage =
			Permill::from_rational_with_rounding(1050u32, 11000u32, Rounding::Down).unwrap();
		// check channel B vtoken share after being cleared
		assert_eq!(ChannelVtokenShares::<Runtime>::get(1, VKSM), channel_b_new_percentage);

		// check PeriodClearedCommissions, should be channel a commission + channel b commission
		assert_eq!(PeriodClearedCommissions::<Runtime>::get(KSM), 6);

		let bifrost_commission_receiver: AccountId32 =
			<Runtime as crate::Config>::BifrostCommissionReceiver::get();
		// check Bifrost commission balance before being cleared
		let bifrost_account_balance_before =
			Currencies::free_balance(KSM, &bifrost_commission_receiver);
		assert_eq!(bifrost_account_balance_before, 0);

		ChannelCommission::on_initialize(103);

		// cleared commissions should be none
		assert_eq!(PeriodClearedCommissions::<Runtime>::get(KSM), 0);

		// check Bifrost commission balance after being cleared
		let bifrost_commission_balance_after =
			Currencies::free_balance(KSM, &bifrost_commission_receiver);
		assert_eq!(bifrost_commission_balance_after, 100 - 6);
	});
}
