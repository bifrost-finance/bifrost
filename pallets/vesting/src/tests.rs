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

use frame_support::{assert_noop, assert_ok, assert_storage_noop};
use frame_system::RawOrigin;
use sp_runtime::{traits::Identity, TokenError};

use super::{Vesting as VestingStorage, *};
use crate::{
	mock::{Balances, ExtBuilder, System, Test, Vesting},
	Vesting as vesting,
};

const ED: u64 = 1000;

const ALICE: u64 = 1;
const ALICE_INIT_BALANCE: u64 = ED * 10;
const ALICE_INIT_LOCKED: u64 = 10 * ED - 5 * ED;
const ALICE_PER_BLOCK: u64 = 500;

const BOB: u64 = 2;
const BOB_INIT_BALANCE: u64 = ED * 20;
const BOB_INIT_LOCKED: u64 = ED * 20;
const BOB_PER_BLOCK: u64 = 1000;

const CHAR: u64 = 12;
const CHAR_INIT_BALANCE: u64 = ED * 30;
const CHAR_INIT_LOCKED: u64 = ED * 30 - 5 * ED;
const CHAR_PER_BLOCK: u64 = 1250;

const DAVE: u64 = 4;

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
		assert_eq!(vesting::<Test>::get(&ALICE).unwrap().to_vec(), vec![user1_vesting_schedule]);
		assert_eq!(vesting::<Test>::get(&BOB).unwrap().to_vec(), vec![user2_vesting_schedule]);
		assert_eq!(vesting::<Test>::get(&CHAR).unwrap().to_vec(), vec![user3_vesting_schedule]);
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
			vesting::<Test>::get(&ALICE).unwrap().to_vec(),
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
		assert_eq!(Cliff::<Test>::get(ALICE), Some(4));

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
		assert_eq!(vesting::<Test>::get(&BOB).unwrap().to_vec(), vec![user_vesting_schedule_1]);

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
		assert_eq!(vesting::<Test>::get(&BOB).unwrap().to_vec(), vec![user_vesting_schedule_2]);
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
		assert_eq!(vesting::<Test>::get(&BOB).unwrap().to_vec(), vec![user_vesting_schedule_1]);

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
		assert_eq!(vesting::<Test>::get(&BOB).unwrap().to_vec(), vec![user_vesting_schedule_2]);
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
			vesting::<Test>::get(&BOB).unwrap().to_vec(),
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
			vesting::<Test>::get(&BOB).unwrap().to_vec(),
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
			vesting::<Test>::get(&BOB).unwrap().to_vec(),
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
			vesting::<Test>::get(&BOB).unwrap().to_vec(),
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
			vesting::<Test>::get(&BOB).unwrap().to_vec(),
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
			vesting::<Test>::get(&BOB).unwrap().to_vec(),
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
			vesting::<Test>::get(&BOB).unwrap().to_vec(),
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
			vesting::<Test>::get(&BOB).unwrap().to_vec(),
			vec![user_vesting_schedule_1, user_vesting_schedule_2]
		);

		//set block to 60
		System::set_block_number(60);
		assert_eq!(System::block_number(), 60);

		//None
		assert_ok!(Vesting::merge_schedules(RawOrigin::Signed(BOB).into(), 0, 1));

		assert_eq!(vesting::<Test>::get(&BOB), None);
		assert_eq!(0, Balances::locks(BOB).to_vec().len());
	})
}

#[test]
fn merge_schedules_that_have_not_started() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // Vest over 20 blocks.
			10,
		);
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched0]);
		assert_eq!(Balances::usable_balance(&2), 0);

		// Add a schedule that is identical to the one that already exists.
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 2, sched0));
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched0, sched0]);
		assert_eq!(Balances::usable_balance(&2), 0);
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1));

		// Since we merged identical schedules, the new schedule finishes at the same
		// time as the original, just with double the amount.
		let sched1 = VestingInfo::new(
			sched0.locked() * 2,
			sched0.per_block() * 2,
			10, // Starts at the block the schedules are merged/
		);
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched1]);

		assert_eq!(Balances::usable_balance(&2), 0);
	});
}

#[test]
fn merge_ongoing_schedules() {
	// Merging two schedules that have started will vest both before merging.
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 0));
		// Account 2 should already have a vesting schedule.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // Vest over 20 blocks.
			10,
		);
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched0]);

		let sched1 = VestingInfo::new(
			ED * 10,
			ED,                          // Vest over 10 blocks.
			sched0.starting_block() + 5, // Start at block 15.
		);
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 2, sched1));
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched0, sched1]);

		// assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1));
		// assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![]);

		// Got to half way through the second schedule where both schedules are actively vesting.
		let cur_block = 20;
		System::set_block_number(cur_block);

		// Account 2 has no usable balances prior to the merge because they have not unlocked
		// with `vest` yet.
		assert_eq!(Balances::usable_balance(&2), 0);

		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1));

		// Merging schedules un-vests all pre-existing schedules prior to merging, which is
		// reflected in account 2's updated usable balance.
		let sched0_vested_now = sched0.per_block() * (cur_block - sched0.starting_block());
		let sched1_vested_now = sched1.per_block() * (cur_block - sched1.starting_block());
		assert_eq!(Balances::usable_balance(&2), sched0_vested_now + sched1_vested_now);

		// The locked amount is the sum of what both schedules have locked at the current block.
		let sched2_locked = sched1
			.locked_at::<Identity>(cur_block, Some(15))
			.saturating_add(sched0.locked_at::<Identity>(cur_block, Some(10)));
		// End block of the new schedule is the greater of either merged schedule.
		let sched2_end = sched1
			.ending_block_as_balance::<Identity>()
			.max(sched0.ending_block_as_balance::<Identity>());
		let sched2_duration = sched2_end - cur_block;
		// Based off the new schedules total locked and its duration, we can calculate the
		// amount to unlock per block.
		let sched2_per_block = sched2_locked / sched2_duration;

		let sched2 = VestingInfo::new(sched2_locked, sched2_per_block, cur_block);
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched2]);

		// And just to double check, we assert the new merged schedule we be cleaned up as expected.
		System::set_block_number(30);
		vest_and_assert_no_vesting::<Test>(2);
	});
}

/// Calls vest, and asserts that there is no entry for `account`
/// in the `Vesting` storage item.
fn vest_and_assert_no_vesting<T>(account: u64)
where
	u64: parity_scale_codec::EncodeLike<<T as frame_system::Config>::AccountId>,
	T: pallet::Config,
{
	// Its ok for this to fail because the user may already have no schedules.
	let _result = Vesting::vest(Some(account).into());
	assert!(!<VestingStorage<T>>::contains_key(account));
}

#[test]
fn merging_shifts_other_schedules_index() {
	// Schedules being merged are filtered out, schedules to the right of any merged
	// schedule shift left and the merged schedule is always last.
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 0));
		let sched0 = VestingInfo::new(
			ED * 10,
			ED, // Vesting over 10 blocks.
			10,
		);
		let sched1 = VestingInfo::new(
			ED * 11,
			ED, // Vesting over 11 blocks.
			11,
		);
		let sched2 = VestingInfo::new(
			ED * 12,
			ED, // Vesting over 12 blocks.
			12,
		);

		// Account 3 starts out with no schedules,
		assert_eq!(vesting::<Test>::get(&3), None);
		// and some usable balance.
		let usable_balance = Balances::usable_balance(&3);
		assert_eq!(usable_balance, 30 * ED);

		let cur_block = 1;
		assert_eq!(System::block_number(), cur_block);

		// Transfer the above 3 schedules to account 3.
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 3, sched0));
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 3, sched1));
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 3, sched2));

		// With no schedules vested or merged they are in the order they are created
		assert_eq!(vesting::<Test>::get(&3).unwrap(), vec![sched0, sched1, sched2]);
		// and the usable balance has not changed.
		assert_eq!(usable_balance, Balances::usable_balance(&3));

		assert_ok!(Vesting::merge_schedules(Some(3).into(), 0, 2));

		// Create the merged schedule of sched0 & sched2.
		// The merged schedule will have the max possible starting block,
		let sched3_start = sched1.starting_block().max(sched2.starting_block());
		// `locked` equal to the sum of the two schedules locked through the current block,
		let sched3_locked = sched2.locked_at::<Identity>(cur_block, Some(sched2.starting_block())) +
			sched0.locked_at::<Identity>(cur_block, Some(sched0.starting_block()));
		// and will end at the max possible block.
		let sched3_end = sched2
			.ending_block_as_balance::<Identity>()
			.max(sched0.ending_block_as_balance::<Identity>());
		let sched3_duration = sched3_end - sched3_start;
		let sched3_per_block = sched3_locked / sched3_duration;
		let sched3 = VestingInfo::new(sched3_locked, sched3_per_block, sched3_start);

		// The not touched schedule moves left and the new merged schedule is appended.
		assert_eq!(vesting::<Test>::get(&3).unwrap(), vec![sched1, sched3]);
		// The usable balance hasn't changed since none of the schedules have started.
		assert_eq!(Balances::usable_balance(&3), usable_balance);
	});
}

#[test]
fn merge_ongoing_and_yet_to_be_started_schedules() {
	// Merge an ongoing schedule that has had `vest` called and a schedule that has not already
	// started.
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 0));
		// Account 2 should already have a vesting schedule.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // Vesting over 20 blocks
			10,
		);
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched0]);

		// Fast forward to half way through the life of sched1.
		let mut cur_block =
			(sched0.starting_block() + sched0.ending_block_as_balance::<Identity>()) / 2;
		assert_eq!(cur_block, 20);
		System::set_block_number(cur_block);

		// Prior to vesting there is no usable balance.
		let mut usable_balance = 0;
		assert_eq!(Balances::usable_balance(&2), usable_balance);
		// Vest the current schedules (which is just sched0 now).
		Vesting::vest(Some(2).into()).unwrap();

		// After vesting the usable balance increases by the unlocked amount.
		let sched0_vested_now = sched0.locked() -
			sched0.locked_at::<Identity>(cur_block, Some(sched0.starting_block()));
		usable_balance += sched0_vested_now;
		assert_eq!(Balances::usable_balance(&2), usable_balance);

		// Go forward a block.
		cur_block += 1;
		System::set_block_number(cur_block);

		// And add a schedule that starts after this block, but before sched0 finishes.
		let sched1 = VestingInfo::new(
			ED * 10,
			1, // Vesting over 256 * 10 (2560) blocks
			cur_block + 1,
		);
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 2, sched1));

		// Merge the schedules before sched1 starts.
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1));
		// After merging, the usable balance only changes by the amount sched0 vested since we
		// last called `vest` (which is just 1 block). The usable balance is not affected by
		// sched1 because it has not started yet.
		usable_balance += sched0.per_block();
		assert_eq!(Balances::usable_balance(&2), usable_balance);

		// The resulting schedule will have the later starting block of the two,
		let sched2_start = sched1.starting_block();
		// `locked` equal to the sum of the two schedules locked through the current block,
		let sched2_locked = sched0.locked_at::<Identity>(cur_block, Some(sched0.starting_block())) +
			sched1.locked_at::<Identity>(cur_block, Some(sched1.starting_block()));
		// and will end at the max possible block.
		let sched2_end = sched0
			.ending_block_as_balance::<Identity>()
			.max(sched1.ending_block_as_balance::<Identity>());
		let sched2_duration = sched2_end - sched2_start;
		let sched2_per_block = sched2_locked / sched2_duration;

		let sched2 = VestingInfo::new(sched2_locked, sched2_per_block, sched2_start);
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched2]);
	});
}

#[test]
fn merge_finished_and_ongoing_schedules() {
	// If a schedule finishes by the current block we treat the ongoing schedule,
	// without any alterations, as the merged one.
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 0));
		// Account 2 should already have a vesting schedule.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // Vesting over 20 blocks.
			10,
		);
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched0]);

		let sched1 = VestingInfo::new(
			ED * 40,
			ED, // Vesting over 40 blocks.
			10,
		);
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 2, sched1));

		// Transfer a 3rd schedule, so we can demonstrate how schedule indices change.
		// (We are not merging this schedule.)
		let sched2 = VestingInfo::new(
			ED * 30,
			ED, // Vesting over 30 blocks.
			10,
		);
		assert_ok!(Vesting::vested_transfer(Some(3).into(), 2, sched2));

		// The schedules are in expected order prior to merging.
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched0, sched1, sched2]);

		// Fast forward to sched0's end block.
		let cur_block = sched0.ending_block_as_balance::<Identity>();
		System::set_block_number(cur_block);
		assert_eq!(System::block_number(), 30);

		// Prior to `merge_schedules` and with no vest/vest_other called the user has no usable
		// balance.
		assert_eq!(Balances::usable_balance(&2), 0);
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1));

		// sched2 is now the first, since sched0 & sched1 get filtered out while "merging".
		// sched1 gets treated like the new merged schedule by getting pushed onto back
		// of the vesting schedules vec. Note: sched0 finished at the current block.
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched2, sched1]);

		// sched0 has finished, so its funds are fully unlocked.
		let sched0_unlocked_now = sched0.locked();
		// The remaining schedules are ongoing, so their funds are partially unlocked.
		let sched1_unlocked_now = sched1.locked() -
			sched1.locked_at::<Identity>(cur_block, Some(sched1.starting_block()));
		let sched2_unlocked_now = sched2.locked() -
			sched2.locked_at::<Identity>(cur_block, Some(sched2.starting_block()));

		// Since merging also vests all the schedules, the users usable balance after merging
		// includes all pre-existing schedules unlocked through the current block, including
		// schedules not merged.
		assert_eq!(
			Balances::usable_balance(&2),
			sched0_unlocked_now + sched1_unlocked_now + sched2_unlocked_now
		);
	});
}

#[test]
fn merge_finishing_schedules_does_not_create_a_new_one() {
	// If both schedules finish by the current block we don't create new one
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 0));
		// Account 2 should already have a vesting schedule.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // 20 block duration.
			10,
		);
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched0]);

		// Create sched1 and transfer it to account 2.
		let sched1 = VestingInfo::new(
			ED * 30,
			ED, // 30 block duration.
			10,
		);
		assert_ok!(Vesting::vested_transfer(Some(3).into(), 2, sched1));
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched0, sched1]);

		let all_scheds_end = sched0
			.ending_block_as_balance::<Identity>()
			.max(sched1.ending_block_as_balance::<Identity>());

		assert_eq!(all_scheds_end, 40);
		System::set_block_number(all_scheds_end);

		// Prior to merge_schedules and with no vest/vest_other called the user has no usable
		// balance.
		assert_eq!(Balances::usable_balance(&2), 0);

		// Merge schedule 0 and 1.
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1));
		// The user no longer has any more vesting schedules because they both ended at the
		// block they where merged,
		assert!(!<VestingStorage<Test>>::contains_key(&2));
		// and their usable balance has increased by the total amount locked in the merged
		// schedules.
		assert_eq!(Balances::usable_balance(&2), sched0.locked() + sched1.locked());
	});
}

#[test]
fn merge_finished_and_yet_to_be_started_schedules() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 0));
		// Account 2 should already have a vesting schedule.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // 20 block duration.
			10, // Ends at block 30
		);
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched0]);

		let sched1 = VestingInfo::new(
			ED * 30,
			ED * 2, // 30 block duration.
			35,
		);
		assert_ok!(Vesting::vested_transfer(Some(13).into(), 2, sched1));
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched0, sched1]);

		let sched2 = VestingInfo::new(
			ED * 40,
			ED, // 40 block duration.
			30,
		);
		// Add a 3rd schedule to demonstrate how sched1 shifts.
		assert_ok!(Vesting::vested_transfer(Some(13).into(), 2, sched2));
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched0, sched1, sched2]);

		System::set_block_number(30);

		// At block 30, sched0 has finished unlocking while sched1 and sched2 are still fully
		// locked,
		assert_eq!(Vesting::vesting_balance(&2), Some(sched1.locked() + sched2.locked()));
		// but since we have not vested usable balance is still 0.
		assert_eq!(Balances::usable_balance(&2), 0);

		// Merge schedule 0 and 1.
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1));

		// sched0 is removed since it finished, and sched1 is removed and then pushed on the back
		// because it is treated as the merged schedule
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched2, sched1]);

		// The usable balance is updated because merging fully unlocked sched0.
		assert_eq!(Balances::usable_balance(&2), sched0.locked());
	});
}

#[test]
fn merge_schedules_throws_proper_errors() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // 20 block duration.
			10,
		);
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched0]);

		// Account 2 only has 1 vesting schedule.
		assert_noop!(
			Vesting::merge_schedules(Some(2).into(), 0, 1),
			Error::<Test>::ScheduleIndexOutOfBounds
		);

		// Account 4 has 0 vesting schedules.
		assert_eq!(vesting::<Test>::get(&4), None);
		assert_noop!(Vesting::merge_schedules(Some(4).into(), 0, 1), Error::<Test>::NotVesting);

		// There are enough schedules to merge but an index is non-existent.
		Vesting::vested_transfer(Some(3).into(), 2, sched0).unwrap();
		assert_eq!(vesting::<Test>::get(&2).unwrap(), vec![sched0, sched0]);
		assert_noop!(
			Vesting::merge_schedules(Some(2).into(), 0, 2),
			Error::<Test>::ScheduleIndexOutOfBounds
		);

		// It is a storage noop with no errors if the indexes are the same.
		assert_storage_noop!(Vesting::merge_schedules(Some(2).into(), 0, 0).unwrap());
	});
}

#[test]
#[should_panic]
fn multiple_schedules_from_genesis_config_errors() {
	// MaxVestingSchedules is 3, but this config has 4 for account 12 so we panic when building
	// from genesis.
	let vesting_config =
		vec![(12, 10, 20, ED), (12, 10, 20, ED), (12, 10, 20, ED), (12, 10, 20, ED)];
	ExtBuilder::default()
		.existential_deposit(ED)
		.vesting_genesis_config(vesting_config)
		.build();
}

#[test]
fn build_genesis_has_storage_version_v1() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_eq!(StorageVersion::<Test>::get(), Releases::V1);
	});
}

#[test]
fn merge_vesting_handles_per_block_0() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		const ED: u64 = 256;
		assert_ok!(Vesting::init_vesting_start_at(RawOrigin::Root.into(), 0));
		let sched0 = VestingInfo::new(
			ED, 0, // Vesting over 256 blocks.
			1,
		);
		assert_eq!(sched0.ending_block_as_balance::<Identity>(), 257);
		let sched1 = VestingInfo::new(
			ED * 2,
			0, // Vesting over 512 blocks.
			10,
		);
		assert_eq!(sched1.ending_block_as_balance::<Identity>(), 512u64 + 10);

		let merged = VestingInfo::new(764, 1, 10);
		assert_eq!(Vesting::merge_vesting_info(5, sched0, sched1), Some(merged));
	});
}

#[test]
fn vesting_info_validate_works() {
	let min_transfer = <Test as Config>::MinVestedTransfer::get();
	// Does not check for min transfer.
	assert_eq!(VestingInfo::new(min_transfer - 1, 1u64, 10u64).is_valid(), true);

	// `locked` cannot be 0.
	assert_eq!(VestingInfo::new(0, 1u64, 10u64).is_valid(), false);

	// `per_block` cannot be 0.
	assert_eq!(VestingInfo::new(min_transfer + 1, 0u64, 10u64).is_valid(), false);

	// With valid inputs it does not error.
	assert_eq!(VestingInfo::new(min_transfer, 1u64, 10u64).is_valid(), true);
}

#[test]
fn vesting_info_ending_block_as_balance_works() {
	// Treats `per_block` 0 as 1.
	let per_block_0 = VestingInfo::new(256u32, 0u32, 10u32);
	assert_eq!(per_block_0.ending_block_as_balance::<Identity>(), 256 + 10);

	// `per_block >= locked` always results in a schedule ending the block after it starts
	let per_block_gt_locked = VestingInfo::new(256u32, 256 * 2u32, 10u32);
	assert_eq!(
		per_block_gt_locked.ending_block_as_balance::<Identity>(),
		1 + per_block_gt_locked.starting_block()
	);
	let per_block_eq_locked = VestingInfo::new(256u32, 256u32, 10u32);
	assert_eq!(
		per_block_gt_locked.ending_block_as_balance::<Identity>(),
		per_block_eq_locked.ending_block_as_balance::<Identity>()
	);

	// Correctly calcs end if `locked % per_block != 0`. (We need a block to unlock the remainder).
	let imperfect_per_block = VestingInfo::new(256u32, 250u32, 10u32);
	assert_eq!(
		imperfect_per_block.ending_block_as_balance::<Identity>(),
		imperfect_per_block.starting_block() + 2u32,
	);
	assert_eq!(
		imperfect_per_block.locked_at::<Identity>(
			imperfect_per_block.ending_block_as_balance::<Identity>(),
			Some(10u32)
		),
		0
	);
}

#[test]
fn per_block_works() {
	let per_block_0 = VestingInfo::new(256u32, 0u32, 10u32);
	assert_eq!(per_block_0.per_block(), 1u32);
	assert_eq!(per_block_0.raw_per_block(), 0u32);

	let per_block_1 = VestingInfo::new(256u32, 1u32, 10u32);
	assert_eq!(per_block_1.per_block(), 1u32);
	assert_eq!(per_block_1.raw_per_block(), 1u32);
}

// When an accounts free balance + schedule.locked is less than ED, the vested transfer will fail.
#[test]
fn vested_transfer_less_than_existential_deposit_fails() {
	ExtBuilder::default().existential_deposit(4 * ED).build().execute_with(|| {
		// MinVestedTransfer is less the ED.
		assert!(
			<Test as Config>::Currency::minimum_balance() >
				<Test as Config>::MinVestedTransfer::get()
		);

		let sched =
			VestingInfo::new(<Test as Config>::MinVestedTransfer::get() as u64, 1u64, 10u64);
		// The new account balance with the schedule's locked amount would be less than ED.
		assert!(
			Balances::free_balance(&99) + sched.locked() <
				<Test as Config>::Currency::minimum_balance()
		);

		// vested_transfer fails.
		assert_noop!(Vesting::vested_transfer(Some(3).into(), 99, sched), TokenError::BelowMinimum,);
		// force_vested_transfer fails.
		assert_noop!(
			Vesting::force_vested_transfer(RawOrigin::Root.into(), 3, 99, sched),
			TokenError::BelowMinimum,
		);
	});
}
