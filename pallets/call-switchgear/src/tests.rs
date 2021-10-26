// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

//! Unit tests for the transaction pause module.

#![cfg(test)]

use frame_support::{assert_noop, assert_ok};
use mock::{Event, *};
use sp_runtime::traits::BadOrigin;

use super::*;

const BALANCE_TRANSFER: &<Runtime as frame_system::Config>::Call =
	&mock::Call::Balances(pallet_balances::Call::transfer { dest: ALICE, value: 10 });
const TOKENS_TRANSFER: &<Runtime as frame_system::Config>::Call =
	&mock::Call::Tokens(orml_tokens::Call::transfer { dest: ALICE, currency_id: KSM, amount: 10 });

#[test]
fn switchoff_transaction_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			CallSwitchgear::switchoff_transaction(
				Origin::signed(5),
				b"Balances".to_vec(),
				b"transfer".to_vec()
			),
			BadOrigin
		);

		assert_eq!(
			CallSwitchgear::get_switchoff_transactions((
				b"Balances".to_vec(),
				b"transfer".to_vec()
			)),
			None
		);
		assert_ok!(CallSwitchgear::switchoff_transaction(
			Origin::signed(1),
			b"Balances".to_vec(),
			b"transfer".to_vec()
		));
		System::assert_last_event(Event::CallSwitchgear(crate::Event::TransactionSwitchedoff(
			b"Balances".to_vec(),
			b"transfer".to_vec(),
		)));
		assert_eq!(
			CallSwitchgear::get_switchoff_transactions((
				b"Balances".to_vec(),
				b"transfer".to_vec()
			)),
			Some(())
		);

		assert_noop!(
			CallSwitchgear::switchoff_transaction(
				Origin::signed(1),
				b"CallSwitchgear".to_vec(),
				b"switchoff_transaction".to_vec()
			),
			Error::<Runtime>::CannotSwitchOff
		);
		assert_noop!(
			CallSwitchgear::switchoff_transaction(
				Origin::signed(1),
				b"CallSwitchgear".to_vec(),
				b"some_other_call".to_vec()
			),
			Error::<Runtime>::CannotSwitchOff
		);
		assert_ok!(CallSwitchgear::switchoff_transaction(
			Origin::signed(1),
			b"OtherPallet".to_vec(),
			b"switchoff_transaction".to_vec()
		));
	});
}

#[test]
fn switchon_transaction_transaction_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(CallSwitchgear::switchoff_transaction(
			Origin::signed(1),
			b"Balances".to_vec(),
			b"transfer".to_vec()
		));
		assert_eq!(
			CallSwitchgear::get_switchoff_transactions((
				b"Balances".to_vec(),
				b"transfer".to_vec()
			)),
			Some(())
		);

		assert_noop!(
			CallSwitchgear::switchoff_transaction(
				Origin::signed(5),
				b"Balances".to_vec(),
				b"transfer".to_vec()
			),
			BadOrigin
		);

		assert_ok!(CallSwitchgear::switchon_transaction(
			Origin::signed(1),
			b"Balances".to_vec(),
			b"transfer".to_vec()
		));
		System::assert_last_event(Event::CallSwitchgear(crate::Event::TransactionSwitchedOn(
			b"Balances".to_vec(),
			b"transfer".to_vec(),
		)));
		assert_eq!(
			CallSwitchgear::get_switchoff_transactions((
				b"Balances".to_vec(),
				b"transfer".to_vec()
			)),
			None
		);
	});
}

#[test]
fn paused_transaction_filter_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert!(!SwitchOffTransactionFilter::<Runtime>::contains(BALANCE_TRANSFER));
		assert!(!SwitchOffTransactionFilter::<Runtime>::contains(TOKENS_TRANSFER));
		assert_ok!(CallSwitchgear::switchoff_transaction(
			Origin::signed(1),
			b"Balances".to_vec(),
			b"transfer".to_vec()
		));
		assert_ok!(CallSwitchgear::switchoff_transaction(
			Origin::signed(1),
			b"Tokens".to_vec(),
			b"transfer".to_vec()
		));
		assert!(SwitchOffTransactionFilter::<Runtime>::contains(BALANCE_TRANSFER));
		assert!(SwitchOffTransactionFilter::<Runtime>::contains(TOKENS_TRANSFER));
		assert_ok!(CallSwitchgear::switchon_transaction(
			Origin::signed(1),
			b"Balances".to_vec(),
			b"transfer".to_vec()
		));
		assert_ok!(CallSwitchgear::switchon_transaction(
			Origin::signed(1),
			b"Tokens".to_vec(),
			b"transfer".to_vec()
		));
		assert!(!SwitchOffTransactionFilter::<Runtime>::contains(BALANCE_TRANSFER));
		assert!(!SwitchOffTransactionFilter::<Runtime>::contains(TOKENS_TRANSFER));
	});
}
