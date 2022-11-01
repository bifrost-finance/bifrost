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

use frame_support::{assert_noop, assert_ok, assert_storage_noop, dispatch::EncodeLike};
use frame_system::RawOrigin;
use sp_runtime::traits::{BadOrigin, Identity};

use super::{Vesting as VestingStorage, *};
use crate::mock::{Balances, ExtBuilder, System, Test, Vesting};

const ED: u64 = 1000;

const ALICE: u64 = 1;
const ALICE_INIT_BALANCE: u64 = ED * 10;
const ALICE_INIT_LOCKED: u64 = 10 * ED - 5 * ED;
const ALICE_PER_BLOCK: u64 = 500;

const BOB: u64 = 2;
const BOB_INIT_BALANCE: u64 = ED * 20;
const BOB_INIT_LOCKED: u64 = ED * 20;
const BOB_PER_BLOCK: u64 = 1000;

const CHAR: u64 = 3;
const CHAR_INIT_BALANCE: u64 = ED * 30;
const CHAR_INIT_LOCKED: u64 = ED * 30 - 5 * ED;
const CHAR_PER_BLOCK: u64 = 1250;

const DAVE: u64 = 4;
const DAVE_INIT_BALANCE: u64 = ED * 40;

#[test]
fn check_vesting_status() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let user1_free_balance = Balances::free_balance(&ALICE);
		let user2_free_balance = Balances::free_balance(&BOB);
		let user3_free_balance = Balances::free_balance(&CHAR);
		assert_eq!(user1_free_balance, ALICE_INIT_BALANCE); // Account 1 has free balance
		assert_eq!(user2_free_balance, BOB_INIT_BALANCE); // Account 2 has free balance
		assert_eq!(user3_free_balance, CHAR_INIT_BALANCE); // Account 3 has free balance

		let s = Balances::locks(ALICE).to_vec()[0].amount;
		println!("{:?}", s);
		println!("{:?}", b"vesting ");

		let user1_vesting_schedule = VestingInfo::new(
			ALICE_INIT_LOCKED, // 10 * ED - 5 * ED
			ALICE_PER_BLOCK,   // Vesting over 10 blocks
			0u64,
		);
		let user2_vesting_schedule = VestingInfo::new(
			BOB_INIT_LOCKED, // 20 * ED
			BOB_PER_BLOCK,   // Vesting over 20 blocks
			10u64,
		);
		let user3_vesting_schedule = VestingInfo::new(
			CHAR_INIT_LOCKED, // 30 * ED - ED * 5
			CHAR_PER_BLOCK,   // Vesting over 20 blocks
			10u64,
		);
		assert_eq!(Vesting::vesting(&ALICE).unwrap().to_vec(), vec![user1_vesting_schedule]);
		assert_eq!(Vesting::vesting(&BOB).unwrap().to_vec(), vec![user2_vesting_schedule]);
		assert_eq!(Vesting::vesting(&CHAR).unwrap().to_vec(), vec![user3_vesting_schedule]);
	});
}

#[test]
fn vesting_balance_should_work() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// not set start_at , all return init_locked
		assert_eq!(Vesting::vesting_balance(&ALICE), Some(ALICE_INIT_LOCKED));
		assert_eq!(Vesting::vesting_balance(&BOB), Some(BOB_INIT_LOCKED));
		assert_eq!(Vesting::vesting_balance(&CHAR), Some(CHAR_INIT_LOCKED));

		//set start_at to 0
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 0));

		// current block 1
		assert_eq!(Vesting::vesting_balance(&ALICE), Some(ALICE_INIT_LOCKED - ALICE_PER_BLOCK));
		assert_eq!(Vesting::vesting_balance(&BOB), Some(BOB_INIT_LOCKED));
		assert_eq!(Vesting::vesting_balance(&CHAR), Some(CHAR_INIT_LOCKED));

		//set block to 10
		System::set_block_number(10);
		assert_eq!(System::block_number(), 10);

		assert_eq!(Vesting::vesting_balance(&ALICE), Some(0));
		assert_eq!(Vesting::vesting_balance(&BOB), Some(BOB_INIT_LOCKED));
		assert_eq!(Vesting::vesting_balance(&CHAR), Some(CHAR_INIT_LOCKED));

		//set block to 30
		System::set_block_number(30);
		assert_eq!(System::block_number(), 30);

		assert_eq!(Vesting::vesting_balance(&ALICE), Some(0));
		assert_eq!(Vesting::vesting_balance(&BOB), Some(0));
		assert_eq!(Vesting::vesting_balance(&CHAR), Some(0));
	})
}

#[test]
fn vesting_balance_with_start_at_should_work() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// not set start_at , all return init_locked
		assert_eq!(Vesting::vesting_balance(&ALICE), Some(ALICE_INIT_LOCKED));
		assert_eq!(Vesting::vesting_balance(&BOB), Some(BOB_INIT_LOCKED));
		assert_eq!(Vesting::vesting_balance(&CHAR), Some(CHAR_INIT_LOCKED));

		//set start_at to 0
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 1));

		// current block 1
		assert_eq!(Vesting::vesting_balance(&ALICE), Some(ALICE_INIT_LOCKED));
		assert_eq!(Vesting::vesting_balance(&BOB), Some(BOB_INIT_LOCKED));
		assert_eq!(Vesting::vesting_balance(&CHAR), Some(CHAR_INIT_LOCKED));

		//set block to 10
		System::set_block_number(10);
		assert_eq!(System::block_number(), 10);

		//set start_at to 3
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 3));

		assert_eq!(Vesting::vesting_balance(&ALICE), Some(3 * ALICE_PER_BLOCK));
		assert_eq!(Vesting::vesting_balance(&BOB), Some(BOB_INIT_LOCKED));
		assert_eq!(Vesting::vesting_balance(&CHAR), Some(CHAR_INIT_LOCKED));

		//set block to 30
		System::set_block_number(30);
		assert_eq!(System::block_number(), 30);

		assert_eq!(Vesting::vesting_balance(&ALICE), Some(0));
		assert_eq!(Vesting::vesting_balance(&BOB), Some(3 * BOB_PER_BLOCK));
		assert_eq!(Vesting::vesting_balance(&CHAR), Some(3 * CHAR_PER_BLOCK));
	})
}

#[test]
fn vested_transfer_should_work() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let user1_vesting_schedule_1 = VestingInfo::new(ALICE_INIT_LOCKED, ALICE_PER_BLOCK, 0u64);
		let user1_vesting_schedule_2 = VestingInfo::new(10000, 1000, 15);

		assert_ok!(Vesting::vested_transfer(
			RawOrigin::Signed(DAVE).into(),
			ALICE,
			user1_vesting_schedule_2
		));
		assert_eq!(
			Vesting::vesting(&ALICE).unwrap().to_vec(),
			vec![user1_vesting_schedule_1, user1_vesting_schedule_2]
		);
	})
}

#[test]
fn do_vest_should_work() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		//set start_at to 0
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 0));

		//set block to 5
		System::set_block_number(5);
		assert_eq!(System::block_number(), 5);

		//check alice
		let alice_locked_1 = Balances::locks(ALICE).to_vec()[0].amount;
		assert_ok!(Vesting::vest(RawOrigin::Signed(ALICE).into()));
		let alice_locked_2 = Balances::locks(ALICE).to_vec()[0].amount;

		assert_eq!(alice_locked_1 - 5 * ALICE_PER_BLOCK, alice_locked_2);

		// vested transfer to alice
		assert_ok!(Vesting::vested_transfer(
			RawOrigin::Signed(DAVE).into(),
			ALICE,
			VestingInfo::new(10000, 1000, 2,)
		));
		//check start block is 2 , add locked = 10000 - 3 * 1000
		assert_eq!(
			Vesting::vesting_balance(&ALICE),
			Some(ALICE_INIT_LOCKED - 5 * ALICE_PER_BLOCK + 10000 - 3 * 1000)
		);
		let alice_locked_3 = Balances::locks(ALICE).to_vec()[0].amount;
		assert_eq!(alice_locked_2 + 10000 - 3 * 1000, alice_locked_3);
		assert_ok!(Vesting::vest(RawOrigin::Signed(ALICE).into()));
		let alice_locked_4 = Balances::locks(ALICE).to_vec()[0].amount;
		assert_eq!(alice_locked_3, alice_locked_4);
	})
}

#[test]
fn do_vest_with_cliff_should_work() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		//set start_at to 0
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 0));
		assert_ok!(Vesting::force_set_cliff(RawOrigin::Root.into(), ALICE, 4));
		assert_eq!(Vesting::cliffs(ALICE), Some(4));

		//set block to 4
		System::set_block_number(4);
		assert_eq!(System::block_number(), 4);

		assert_noop!(
			Vesting::vest(RawOrigin::Signed(ALICE).into()),
			Error::<Test>::WrongCliffVesting
		);

		//set block to 5
		System::set_block_number(5);
		assert_eq!(System::block_number(), 5);

		//check alice
		let alice_locked_1 = Balances::locks(ALICE).to_vec()[0].amount;
		assert_ok!(Vesting::vest(RawOrigin::Signed(ALICE).into()));
		let alice_locked_2 = Balances::locks(ALICE).to_vec()[0].amount;
		assert_eq!(alice_locked_1 - 5 * ALICE_PER_BLOCK, alice_locked_2);
	})
}

#[test]
fn do_vest_with_start_at_should_work() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		//set start_at to 3
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 3));

		//set block to 5
		System::set_block_number(5);
		assert_eq!(System::block_number(), 5);

		//check alice
		let alice_locked_1 = Balances::locks(ALICE).to_vec()[0].amount;
		assert_ok!(Vesting::vest(RawOrigin::Signed(ALICE).into()));
		let alice_locked_2 = Balances::locks(ALICE).to_vec()[0].amount;
		assert_eq!(alice_locked_1 - 2 * ALICE_PER_BLOCK, alice_locked_2);

		//set start_at to 3
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 5));

		//set block to 10
		System::set_block_number(10);
		assert_eq!(System::block_number(), 10);
		//check alice
		let alice_locked_3 = Balances::locks(ALICE).to_vec()[0].amount;
		assert_ok!(Vesting::vest(RawOrigin::Signed(ALICE).into()));
		let alice_locked_4 = Balances::locks(ALICE).to_vec()[0].amount;

		//check current locked = 5 * alice_pre_block , before vest 2 * alice_pre_block , now vest 3
		// * alice_pre_block
		assert_eq!(alice_locked_3 - 3 * ALICE_PER_BLOCK, alice_locked_4);
	})
}

#[test]
fn set_vesting_per_block_should_work() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		//set start_at to 0
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 0));

		//set block to 5
		System::set_block_number(5);
		assert_eq!(System::block_number(), 5);

		//check bob
		let alice_locked_1 = Balances::locks(BOB).to_vec()[0].amount;
		assert_eq!(alice_locked_1, BOB_INIT_LOCKED);

		//set vesting_per_block to 100
		assert_ok!(Vesting::set_vesting_per_block(RawOrigin::Root.into(), BOB, 0, 100));

		//check result start_at 10 > now 5 => 10 start_at
		let user_vesting_schedule_1 = VestingInfo::new(20000, 100, 10);
		assert_eq!(Vesting::vesting(&BOB).unwrap().to_vec(), vec![user_vesting_schedule_1]);

		//set block to 15
		System::set_block_number(15);
		assert_eq!(System::block_number(), 15);

		let bob_locked_1 = Balances::locks(BOB).to_vec()[0].amount;

		//set vesting_per_block to 10
		assert_ok!(Vesting::set_vesting_per_block(RawOrigin::Root.into(), BOB, 0, 10));

		//check result start_at 10 < now 15 => now 15 - absolute_start 0
		//old_start_at = old_start_block 10 + absolute_start 0
		//remained_vesting = 20000 - 5 * 100
		let user_vesting_schedule_2 = VestingInfo::new(20000 - 5 * 100, 10, 15);
		assert_eq!(Vesting::vesting(&BOB).unwrap().to_vec(), vec![user_vesting_schedule_2]);
		let bob_locked_2 = Balances::locks(BOB).to_vec()[0].amount;
		assert_eq!(bob_locked_1 - 5 * 100, bob_locked_2);
	})
}

#[test]
fn set_vesting_per_block_with_start_at_should_work() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		//set start_at to 2
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 2));

		//set block to 5
		System::set_block_number(5);
		assert_eq!(System::block_number(), 5);

		//check bob
		let alice_locked_1 = Balances::locks(BOB).to_vec()[0].amount;
		assert_eq!(alice_locked_1, BOB_INIT_LOCKED);

		//set vesting_per_block to 100
		assert_ok!(Vesting::set_vesting_per_block(RawOrigin::Root.into(), BOB, 0, 100));

		//check result old_start_at 12 > now 5 => 10 start_at
		let user_vesting_schedule_1 = VestingInfo::new(20000, 100, 10);
		assert_eq!(Vesting::vesting(&BOB).unwrap().to_vec(), vec![user_vesting_schedule_1]);

		//set block to 15
		System::set_block_number(15);
		assert_eq!(System::block_number(), 15);

		let bob_locked_1 = Balances::locks(BOB).to_vec()[0].amount;

		//set vesting_per_block to 10
		assert_ok!(Vesting::set_vesting_per_block(RawOrigin::Root.into(), BOB, 0, 10));

		//old_start_at = old_start_block 10 + absolute_start 2
		//old_start_at 12 < now 15 => now 15 - absolute_start 2 = 13
		//remained_vesting = 20000 - 3 * 100
		let user_vesting_schedule_2 = VestingInfo::new(20000 - 3 * 100, 10, 13);
		assert_eq!(Vesting::vesting(&BOB).unwrap().to_vec(), vec![user_vesting_schedule_2]);
		let bob_locked_2 = Balances::locks(BOB).to_vec()[0].amount;
		assert_eq!(bob_locked_1 - 3 * 100, bob_locked_2);
	})
}

#[test]
fn repeatedly_set_vesting_per_block_should_work() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		//set start_at to 0
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 2));

		let user_vesting_schedule_1 = VestingInfo::new(BOB_INIT_LOCKED, BOB_PER_BLOCK, 10u64);
		let user_vesting_schedule_2 = VestingInfo::new(10000, 1000, 12);
		let user_vesting_schedule_3 = VestingInfo::new(10000, 1000, 20);

		assert_ok!(Vesting::vested_transfer(
			RawOrigin::Signed(DAVE).into(),
			BOB,
			user_vesting_schedule_2
		));
		assert_ok!(Vesting::vested_transfer(
			RawOrigin::Signed(DAVE).into(),
			BOB,
			user_vesting_schedule_3
		));
		assert_eq!(
			Vesting::vesting(&BOB).unwrap().to_vec(),
			vec![user_vesting_schedule_1, user_vesting_schedule_2, user_vesting_schedule_3]
		);

		//set block to 5
		System::set_block_number(5);
		assert_eq!(System::block_number(), 5);

		//check bob
		let alice_locked_1 = Balances::locks(BOB).to_vec()[0].amount;
		assert_eq!(alice_locked_1, 40000);

		//error OutOfBounds
		assert_noop!(
			Vesting::set_vesting_per_block(RawOrigin::Root.into(), BOB, 3, 100),
			Error::<Test>::ScheduleIndexOutOfBounds
		);
		//set vesting_per_block to 100
		assert_ok!(Vesting::set_vesting_per_block(RawOrigin::Root.into(), BOB, 1, 100));

		//check result old_start_at 12 > now 5 => 12 start_at
		let new_user_vesting_schedule_1 = VestingInfo::new(10000, 100, 12);
		assert_eq!(
			Vesting::vesting(&BOB).unwrap().to_vec(),
			vec![user_vesting_schedule_1, new_user_vesting_schedule_1, user_vesting_schedule_3]
		);

		//set block to 15
		System::set_block_number(15);
		assert_eq!(System::block_number(), 15);

		let bob_locked_1 = Balances::locks(BOB).to_vec()[0].amount;

		//set vesting_per_block to 10
		assert_ok!(Vesting::set_vesting_per_block(RawOrigin::Root.into(), BOB, 0, 10));

		//old_start_at = old_start_block 10 + absolute_start 2
		//old_start_block 12 < now 15 => now 15 - absolute_start 2 = 13
		//remained_vesting = 20000 - 3 * 1000
		let new_user_vesting_schedule_2 = VestingInfo::new(20000 - 3 * 1000, 10, 13);
		assert_eq!(
			Vesting::vesting(&BOB).unwrap().to_vec(),
			vec![new_user_vesting_schedule_2, new_user_vesting_schedule_1, user_vesting_schedule_3]
		);

		let bob_locked_2 = Balances::locks(BOB).to_vec()[0].amount;
		assert_eq!(bob_locked_1 - 3 * 1000 - 100, bob_locked_2);
	})
}

/*
merge_vesting_info

1. now < schedule1_ending_block < schedule2_ending_block => VestingInfo::new(locked, per_block, starting_block)
locked = schedule1_locked_at + schedule2_locked_at
ending_block = bigger ending_block
starting_block = bigger starting_block(inclued now)
per_block = locked / (ending_block - starting_block)

2. schedule1_ending_block <= now < schedule2_ending_block => schedule2
3. schedule2_ending_block <= now < schedule1_ending_block => schedule1
return bigger
4. schedule1_ending_block <= now && schedule2_ending_block <= now =>None
*/

#[test]
fn merge_schedules_has_not_started_should_work() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		//set start_at to 0
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 2));

		let user_vesting_schedule_1 = VestingInfo::new(BOB_INIT_LOCKED, BOB_PER_BLOCK, 10u64);
		let user_vesting_schedule_2 = VestingInfo::new(20000, 1000, 12);

		assert_ok!(Vesting::vested_transfer(
			RawOrigin::Signed(DAVE).into(),
			BOB,
			user_vesting_schedule_2
		));
		assert_eq!(
			Vesting::vesting(&BOB).unwrap().to_vec(),
			vec![user_vesting_schedule_1, user_vesting_schedule_2]
		);

		//set block to 5
		System::set_block_number(5);
		assert_eq!(System::block_number(), 5);

		// ending_block = 32
		// starting_block = 12
		// locked = 40000
		// per_block = 40000 / 20 = 2000
		assert_ok!(Vesting::merge_schedules(RawOrigin::Signed(BOB).into(), 0, 1));

		assert_eq!(
			Vesting::vesting(&BOB).unwrap().to_vec(),
			vec![VestingInfo::new(BOB_INIT_LOCKED * 2, 2000, 12)]
		);
		assert_eq!(40000, Balances::locks(BOB).to_vec()[0].amount);
	})
}

#[test]
fn merge_ongoing_schedules_should_work() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		//set start_at to 0
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 2));

		let user_vesting_schedule_1 = VestingInfo::new(BOB_INIT_LOCKED, BOB_PER_BLOCK, 10u64);
		let user_vesting_schedule_2 = VestingInfo::new(20000, 1000, 40);

		assert_ok!(Vesting::vested_transfer(
			RawOrigin::Signed(DAVE).into(),
			BOB,
			user_vesting_schedule_2
		));
		assert_eq!(
			Vesting::vesting(&BOB).unwrap().to_vec(),
			vec![user_vesting_schedule_1, user_vesting_schedule_2]
		);

		//set block to 50
		System::set_block_number(50);
		assert_eq!(System::block_number(), 50);

		// ending_block = 60
		// starting_block = 40
		// locked = 40000
		// per_block = 40000 / 20 = 2000
		assert_ok!(Vesting::merge_schedules(RawOrigin::Signed(BOB).into(), 0, 1));

		assert_eq!(
			Vesting::vesting(&BOB).unwrap().to_vec(),
			vec![VestingInfo::new(BOB_INIT_LOCKED, 1000, 40)]
		);
		assert_eq!(12000, Balances::locks(BOB).to_vec()[0].amount);
	})
}

#[test]
fn merge_finished_schedules_should_work() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		//set start_at to 0
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 2));

		let user_vesting_schedule_1 = VestingInfo::new(BOB_INIT_LOCKED, BOB_PER_BLOCK, 10u64);
		let user_vesting_schedule_2 = VestingInfo::new(20000, 1000, 40);

		assert_ok!(Vesting::vested_transfer(
			RawOrigin::Signed(DAVE).into(),
			BOB,
			user_vesting_schedule_2
		));
		assert_eq!(
			Vesting::vesting(&BOB).unwrap().to_vec(),
			vec![user_vesting_schedule_1, user_vesting_schedule_2]
		);

		//set block to 60
		System::set_block_number(60);
		assert_eq!(System::block_number(), 60);

		//None
		assert_ok!(Vesting::merge_schedules(RawOrigin::Signed(BOB).into(), 0, 1));

		assert_eq!(Vesting::vesting(&BOB), None);
		assert_eq!(0, Balances::locks(BOB).to_vec().len());
	})
}
