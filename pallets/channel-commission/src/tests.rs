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
use sp_runtime::{AccountId32, DispatchError::BadOrigin};

const CHANNEL_A_NAME: &[u8] = b"channel_a";
const CHANNEL_B_NAME: &[u8] = b"channel_b";
const CHANNEL_C_NAME: &[u8] = b"channel_c";

const CHANNEL_A_RECEIVER: AccountId = AccountId32::new([3u8; 32]);
const CHANNEL_B_RECEIVER: AccountId = AccountId32::new([4u8; 32]);
const CHANNEL_C_RECEIVER: AccountId = AccountId32::new([5u8; 32]);
const CHANNEL_A_BACKUP_RECEIVER: AccountId = AccountId32::new([6u8; 32]);

fn setup() {
	// set commission tokens: VKSM -> KSM
	assert_ok!(ChannelCommission::set_commission_tokens(RuntimeOrigin::signed(ALICE), VKSM, KSM,));

	// set commission tokens: VBNC -> BNC
	assert_ok!(ChannelCommission::set_commission_tokens(RuntimeOrigin::signed(ALICE), VBNC, BNC,));

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

	// register channel C
	assert_ok!(ChannelCommission::register_channel(
		RuntimeOrigin::signed(ALICE),
		CHANNEL_C_NAME.to_vec(),
		CHANNEL_C_RECEIVER.clone(),
	));
}

#[test]
fn set_commission_tokens_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(ChannelCommission::set_commission_tokens(
			RuntimeOrigin::signed(ALICE),
			VKSM,
			KSM,
		));

		// Channel A is registered
		assert_eq!(CommissionTokens::<Runtime>::get(VKSM), Some(KSM));
	});
}

#[test]
fn register_channel_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
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
		assert_eq!(
			ChannelCommissionTokenRates::<Runtime>::get(0, VKSM),
			Some(DEFAULT_COMMISSION_RATE)
		);

		assert_eq!(
			ChannelCommissionTokenRates::<Runtime>::get(0, VBNC),
			Some(DEFAULT_COMMISSION_RATE)
		);

		// successfully remove Channel A
		assert_ok!(ChannelCommission::remove_channel(RuntimeOrigin::signed(ALICE), 0));

		// Channel A is removed
		assert_eq!(Channels::<Runtime>::get(0), None);

		// Channel A has no records in ChannelCommissionTokenRates in both VKSM and VBNC
		assert_eq!(ChannelCommissionTokenRates::<Runtime>::get(0, VKSM), None);

		assert_eq!(ChannelCommissionTokenRates::<Runtime>::get(0, VBNC), None);
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
fn set_channel_commission_token_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		setup();

		// assure Channel A has records in ChannelCommissionTokenRates in both VKSM and VBNC
		assert_eq!(
			ChannelCommissionTokenRates::<Runtime>::get(0, VKSM),
			Some(DEFAULT_COMMISSION_RATE)
		);

		assert_eq!(
			ChannelCommissionTokenRates::<Runtime>::get(0, VBNC),
			Some(DEFAULT_COMMISSION_RATE)
		);

		let new_rate = Percent::from_percent(50);
		// set commission token VKSM to 50%
		assert_ok!(ChannelCommission::set_channel_commission_token(
			RuntimeOrigin::signed(ALICE),
			0,
			VKSM,
			Some(50)
		));

		assert_eq!(ChannelCommissionTokenRates::<Runtime>::get(0, VKSM), Some(new_rate));

		// set commission token VBNC to None, which means removing it
		assert_ok!(ChannelCommission::set_channel_commission_token(
			RuntimeOrigin::signed(ALICE),
			0,
			VBNC,
			None
		));

		assert_eq!(ChannelCommissionTokenRates::<Runtime>::get(0, VBNC), None);
	});
}

#[test]
fn claim_commissions_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		setup();

		let commission_account = CommissionPalletId::get().into_account_truncating();
		// endow CommissionPalletId account with 1000 KSM and 1000 BNC
		assert_ok!(Currencies::deposit(KSM, &commission_account, 1000));
		assert_ok!(Currencies::deposit(BNC, &commission_account, 1000));

		// set channel A's claimable KSM amount to 100
		ChannelClaimableCommissions::<Runtime>::insert(0, KSM, 100);

		// set channel A's claimable BNC amount to 120
		ChannelClaimableCommissions::<Runtime>::insert(0, BNC, 120);

		// assure channel A's claimable KSM amount is 100
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(0, KSM), Some(100));

		// assure channel A's claimable BNC amount is 120
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(0, BNC), Some(120));

		let receiver_ksm_before = Currencies::free_balance(KSM, &CHANNEL_A_RECEIVER);
		let receiver_bnc_before = Currencies::free_balance(BNC, &CHANNEL_A_RECEIVER);

		// claim 50 KSM from channel A
		assert_ok!(ChannelCommission::claim_commissions(
			RuntimeOrigin::signed(CHANNEL_A_RECEIVER.clone()),
			0,
		));

		// assure channel A's claimable KSM amount is None
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(0, KSM), None);

		// assure channel A's claimable BNC amount is None
		assert_eq!(ChannelClaimableCommissions::<Runtime>::get(0, BNC), None);

		// assure channel A's receiver's KSM balance is increased by 100
		assert_eq!(Currencies::free_balance(KSM, &CHANNEL_A_RECEIVER), receiver_ksm_before + 100);

		// assure channel A's receiver's BNC balance is increased by 120
		assert_eq!(Currencies::free_balance(BNC, &CHANNEL_A_RECEIVER), receiver_bnc_before + 120);
	});
}
