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

#![cfg(test)]

use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};
use node_primitives::currency::KSM;
use pallet_bcmp::fee::GasConfig;
use sp_core::{Hasher, Pair};
use sp_runtime::{traits::BlakeTwo256, DispatchError::BadOrigin, Percent};
use std::str::FromStr;

const FROM_CHAIN_ID: u32 = 31337;
const FILECOIN_NETWORK_ID: xcm::v3::NetworkId =
	NetworkId::Ethereum { chain_id: FROM_CHAIN_ID as u64 };
const ONE_FIL: u64 = 1_000_000_000_000_000_000;

#[test]
fn cross_in_and_cross_out_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::AccountKey20 {
				network: Some(FILECOIN_NETWORK_ID),
				key: [0u8; 20],
			}),
		};

		assert_noop!(
			CrossInOut::cross_in(
				RuntimeOrigin::signed(ALICE),
				Box::new(location.clone()),
				KSM,
				100,
				None,
				KSM
			),
			Error::<Runtime>::NotAllowed
		);

		let bounded_vector = BoundedVec::try_from(vec![ALICE]).unwrap();
		IssueWhiteList::<Runtime>::insert(KSM, bounded_vector);

		assert_noop!(
			CrossInOut::cross_in(
				RuntimeOrigin::signed(ALICE),
				Box::new(location.clone()),
				KSM,
				100,
				None,
				KSM
			),
			Error::<Runtime>::CurrencyNotSupportCrossInAndOut
		);

		CrossCurrencyRegistry::<Runtime>::insert(KSM, ());

		assert_noop!(
			CrossInOut::cross_in(
				RuntimeOrigin::signed(ALICE),
				Box::new(location.clone()),
				KSM,
				100,
				None,
				KSM
			),
			Error::<Runtime>::NoCrossingMinimumSet
		);

		CrossingMinimumAmount::<Runtime>::insert(KSM, (1000, 1000));

		assert_noop!(
			CrossInOut::cross_in(
				RuntimeOrigin::signed(ALICE),
				Box::new(location.clone()),
				KSM,
				100,
				None,
				KSM
			),
			Error::<Runtime>::AmountLowerThanMinimum
		);

		CrossingMinimumAmount::<Runtime>::insert(KSM, (1, 1));

		assert_noop!(
			CrossInOut::cross_in(
				RuntimeOrigin::signed(ALICE),
				Box::new(location.clone()),
				KSM,
				100,
				None,
				KSM
			),
			Error::<Runtime>::NoAccountIdMapping
		);

		AccountToOuterMultilocation::<Runtime>::insert(KSM, ALICE, location.clone());
		OuterMultilocationToAccount::<Runtime>::insert(KSM, location.clone(), ALICE);

		assert_eq!(Tokens::free_balance(KSM, &ALICE), 0);
		assert_ok!(CrossInOut::cross_in(
			RuntimeOrigin::signed(ALICE),
			Box::new(location),
			KSM,
			100,
			None,
			KSM
		));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 100);

		assert_ok!(CrossInOut::cross_out(RuntimeOrigin::signed(ALICE), KSM, 50, KSM));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 50);
	});
}

#[test]
fn add_to_and_remove_from_issue_whitelist_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_eq!(CrossInOut::get_issue_whitelist(KSM), None);

		assert_ok!(CrossInOut::add_to_issue_whitelist(RuntimeOrigin::signed(ALICE), KSM, ALICE));
		let bounded_vector = BoundedVec::try_from(vec![ALICE]).unwrap();
		assert_eq!(CrossInOut::get_issue_whitelist(KSM), Some(bounded_vector));

		assert_noop!(
			CrossInOut::remove_from_issue_whitelist(RuntimeOrigin::signed(ALICE), KSM, BOB),
			Error::<Runtime>::NotExist
		);

		assert_ok!(CrossInOut::remove_from_issue_whitelist(
			RuntimeOrigin::signed(ALICE),
			KSM,
			ALICE
		));
		let empty_vec = BoundedVec::default();
		assert_eq!(CrossInOut::get_issue_whitelist(KSM), Some(empty_vec));
	});
}

#[test]
fn add_to_and_remove_from_register_whitelist_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_eq!(CrossInOut::get_register_whitelist(KSM), None);

		assert_ok!(CrossInOut::add_to_register_whitelist(RuntimeOrigin::signed(ALICE), KSM, ALICE));
		assert_eq!(CrossInOut::get_register_whitelist(KSM), Some(vec![ALICE]));

		assert_noop!(
			CrossInOut::remove_from_register_whitelist(RuntimeOrigin::signed(ALICE), KSM, BOB),
			Error::<Runtime>::NotExist
		);

		assert_ok!(CrossInOut::remove_from_register_whitelist(
			RuntimeOrigin::signed(ALICE),
			KSM,
			ALICE
		));
		assert_eq!(CrossInOut::get_register_whitelist(KSM), Some(vec![]));
	});
}

#[test]
fn register_linked_account_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::AccountKey20 {
				network: Some(FILECOIN_NETWORK_ID),
				key: [0u8; 20],
			}),
		};

		let location2 = MultiLocation {
			parents: 111,
			interior: X1(Junction::AccountKey20 {
				network: Some(FILECOIN_NETWORK_ID),
				key: [0u8; 20],
			}),
		};

		assert_noop!(
			CrossInOut::register_linked_account(
				RuntimeOrigin::signed(ALICE),
				KSM,
				BOB,
				Box::new(location.clone())
			),
			Error::<Runtime>::NotAllowed
		);

		RegisterWhiteList::<Runtime>::insert(KSM, vec![ALICE]);

		assert_noop!(
			CrossInOut::register_linked_account(
				RuntimeOrigin::signed(ALICE),
				KSM,
				BOB,
				Box::new(location.clone())
			),
			Error::<Runtime>::CurrencyNotSupportCrossInAndOut
		);

		CrossCurrencyRegistry::<Runtime>::insert(KSM, ());

		assert_ok!(CrossInOut::register_linked_account(
			RuntimeOrigin::signed(ALICE),
			KSM,
			ALICE,
			Box::new(location.clone())
		));

		assert_noop!(
			CrossInOut::register_linked_account(
				RuntimeOrigin::signed(ALICE),
				KSM,
				ALICE,
				Box::new(location2)
			),
			Error::<Runtime>::AlreadyExist
		);
	});
}

#[test]
fn register_and_deregister_currency_for_cross_in_out_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(CrossInOut::register_currency_for_cross_in_out(
			RuntimeOrigin::signed(ALICE),
			KSM,
		));

		assert_eq!(CrossCurrencyRegistry::<Runtime>::get(KSM), Some(()));

		assert_ok!(CrossInOut::deregister_currency_for_cross_in_out(
			RuntimeOrigin::signed(ALICE),
			KSM,
		));

		assert_eq!(CrossCurrencyRegistry::<Runtime>::get(KSM), None);
	});
}

#[test]
fn change_outer_linked_account_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::AccountKey20 {
				network: Some(FILECOIN_NETWORK_ID),
				key: [0u8; 20],
			}),
		};

		let location2 = MultiLocation {
			parents: 111,
			interior: X1(Junction::AccountKey20 {
				network: Some(FILECOIN_NETWORK_ID),
				key: [0u8; 20],
			}),
		};

		AccountToOuterMultilocation::<Runtime>::insert(KSM, BOB, location.clone());
		OuterMultilocationToAccount::<Runtime>::insert(KSM, location.clone(), BOB);

		assert_noop!(
			CrossInOut::change_outer_linked_account(
				RuntimeOrigin::signed(BOB),
				KSM,
				Box::new(location2.clone()),
				BOB
			),
			BadOrigin
		);

		assert_noop!(
			CrossInOut::change_outer_linked_account(
				RuntimeOrigin::signed(ALICE),
				KSM,
				Box::new(location.clone()),
				BOB
			),
			Error::<Runtime>::CurrencyNotSupportCrossInAndOut
		);

		CrossCurrencyRegistry::<Runtime>::insert(KSM, ());

		assert_noop!(
			CrossInOut::change_outer_linked_account(
				RuntimeOrigin::signed(ALICE),
				KSM,
				Box::new(location.clone()),
				BOB
			),
			Error::<Runtime>::AlreadyExist
		);

		assert_ok!(CrossInOut::change_outer_linked_account(
			RuntimeOrigin::signed(ALICE),
			KSM,
			Box::new(location2.clone()),
			BOB
		));
	});
}

#[test]
fn set_crossing_minimum_amount_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_noop!(
			CrossInOut::set_crossing_minimum_amount(
				RuntimeOrigin::signed(BOB),
				KSM,
				Some((100, 100))
			),
			BadOrigin
		);

		assert_ok!(CrossInOut::set_crossing_minimum_amount(
			RuntimeOrigin::signed(ALICE),
			KSM,
			Some((100, 100))
		));

		assert_eq!(CrossingMinimumAmount::<Runtime>::get(KSM), Some((100, 100)));
	});
}

/* unit tests for cross chain transfer */
fn initialize_pallet_bcmp(cmt_pair: sp_core::ed25519::Pair) -> (H256, H256) {
	let pk = cmt_pair.public().0.to_vec();

	let dst_anchor: H256 = BlakeTwo256::hash(&b"BIFROST_POLKADOT_CROSS_IN_OUT"[..]);
	let src_anchor =
		H256::from_str("000000000000000000000008942c628b66a34c2234928a4adc3ccc437baa0dd0").unwrap();

	assert_ok!(Bcmp::register_anchor(RuntimeOrigin::signed(ALICE), dst_anchor, pk.clone()));
	assert_ok!(Bcmp::set_chain_id(RuntimeOrigin::signed(ALICE), FROM_CHAIN_ID));
	assert_ok!(Bcmp::enable_path(
		RuntimeOrigin::signed(ALICE),
		FROM_CHAIN_ID,
		src_anchor,
		dst_anchor
	));

	(src_anchor, dst_anchor)
}

fn initialize_cross_in_out() {
	// cross-in-out pallet initialization
	assert_ok!(CrossInOut::set_chain_network_id(
		RuntimeOrigin::signed(ALICE),
		FIL,
		Some(FILECOIN_NETWORK_ID)
	));

	// register currency
	assert_ok!(CrossInOut::register_currency_for_cross_in_out(RuntimeOrigin::signed(ALICE), FIL,));
	assert_ok!(CrossInOut::register_currency_for_cross_in_out(RuntimeOrigin::signed(ALICE), VFIL,));

	// set crossing minimum amount
	assert_ok!(CrossInOut::set_crossing_minimum_amount(
		RuntimeOrigin::signed(ALICE),
		FIL,
		Some((ONE_FIL, ONE_FIL))
	));
	assert_ok!(CrossInOut::set_crossing_minimum_amount(
		RuntimeOrigin::signed(ALICE),
		VFIL,
		Some((ONE_FIL, ONE_FIL))
	));

	// register wihitelist
	assert_ok!(CrossInOut::add_to_issue_whitelist(RuntimeOrigin::signed(ALICE), FIL, ALICE));
	assert_ok!(CrossInOut::add_to_register_whitelist(RuntimeOrigin::signed(ALICE), FIL, ALICE));

	assert_ok!(CrossInOut::add_to_issue_whitelist(RuntimeOrigin::signed(ALICE), VFIL, ALICE));
	assert_ok!(CrossInOut::add_to_register_whitelist(RuntimeOrigin::signed(ALICE), VFIL, ALICE));

	// register account id mapping
	assert_ok!(CrossInOut::register_linked_account(
		RuntimeOrigin::signed(ALICE),
		FIL,
		ALICE,
		Box::new(MultiLocation {
			parents: 100,
			interior: X1(Junction::AccountKey20 {
				network: Some(FILECOIN_NETWORK_ID),
				key: [1u8; 20],
			}),
		})
	));
}

#[test]
fn cross_in_fil_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let amount = 12000000000000000000;
		let receiver = [1u8; 20].to_vec();

		// can only get the first 3 32 bytes
		let mut payload = CrossInOut::get_cross_out_payload(
			XcmOperationType::TransferBack,
			FIL,
			amount,
			Some(&receiver),
		)
		.unwrap();

		// following 32 bytes is receiver
		let fixed_address = CrossInOut::extend_to_bytes32(&receiver, 32);
		payload.extend_from_slice(&fixed_address);

		initialize_cross_in_out();

		let cmt_pair = sp_core::ed25519::Pair::generate().0;
		let (src_anchor, dst_anchor) = initialize_pallet_bcmp(cmt_pair);

		let message = Message {
			uid: H256::from_str("00007A6900000000000000000000000000000000000000000000000000000000")
				.unwrap(),
			cross_type: <Runtime as pallet_bcmp::Config>::PureMessage::get(),
			src_anchor,
			extra_feed: vec![],
			dst_anchor,
			payload,
		};

		let mock_sig = cmt_pair.sign(&message.encode()).0.to_vec();
		assert_ok!(Bcmp::receive_message(
			RuntimeOrigin::signed(ALICE),
			mock_sig.clone(),
			message.encode()
		));

		assert_eq!(Tokens::free_balance(FIL, &ALICE), amount);
	});
}

#[test]
fn cross_in_vfil_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let amount = 12000000000000000000;
		let receiver = [1u8; 20].to_vec();

		// can only get the first 3 32 bytes
		let mut payload = CrossInOut::get_cross_out_payload(
			XcmOperationType::TransferBack,
			VFIL,
			amount,
			Some(&receiver),
		)
		.unwrap();

		// following 32 bytes is receiver
		let fixed_address = CrossInOut::extend_to_bytes32(&receiver, 32);
		payload.extend_from_slice(&fixed_address);

		initialize_cross_in_out();

		let cmt_pair = sp_core::ed25519::Pair::generate().0;
		let (src_anchor, dst_anchor) = initialize_pallet_bcmp(cmt_pair);

		let message = Message {
			uid: H256::from_str("00007A6900000000000000000000000000000000000000000000000000000000")
				.unwrap(),
			cross_type: <Runtime as pallet_bcmp::Config>::PureMessage::get(),
			src_anchor,
			extra_feed: vec![],
			dst_anchor,
			payload,
		};

		let mock_sig = cmt_pair.sign(&message.encode()).0.to_vec();
		assert_ok!(Bcmp::receive_message(
			RuntimeOrigin::signed(ALICE),
			mock_sig.clone(),
			message.encode()
		));

		assert_eq!(Tokens::free_balance(VFIL, &ALICE), amount);
	});
}

#[test]
fn fil_send_message_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		// endow some FIL to ALICE
		let endowed_amount = 5 * ONE_FIL;
		assert_ok!(Tokens::deposit(FIL, &ALICE, endowed_amount));
		assert_eq!(Tokens::free_balance(FIL, &ALICE), endowed_amount);

		let amount = ONE_FIL;
		let receiver = [1u8; 20];
		let receiver_location = MultiLocation {
			parents: 100,
			interior: X1(Junction::AccountKey20 {
				network: Some(FILECOIN_NETWORK_ID),
				key: receiver,
			}),
		};

		// initialize cross-in-out pallet and bcmp pallet
		initialize_cross_in_out();
		let cmt_pair = sp_core::ed25519::Pair::generate().0;
		initialize_pallet_bcmp(cmt_pair);

		let dest_location =
			crate::AccountToOuterMultilocation::<Runtime>::get(FIL, &ALICE).unwrap();
		assert_eq!(dest_location, receiver_location);

		assert_ok!(CrossInOut::send_message(ALICE, FIL, amount, Box::new(dest_location), FIL));
	});
}

#[test]
fn cross_out_fil_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let amount = ONE_FIL;

		// endow some FIL to ALICE
		let endowed_amount = 5 * ONE_FIL;
		assert_ok!(Tokens::deposit(FIL, &ALICE, endowed_amount));
		assert_eq!(Tokens::free_balance(FIL, &ALICE), endowed_amount);

		// endow some BNC to ALICE
		let endowed_amount = 5 * ONE_FIL;
		assert_ok!(Balances::deposit_into_existing(&ALICE, endowed_amount));

		// initialize cross-in-out pallet and bcmp pallet
		initialize_cross_in_out();
		let cmt_pair = sp_core::ed25519::Pair::generate().0;
		initialize_pallet_bcmp(cmt_pair);

		// Bcmp set fee standard
		let config = GasConfig {
			chain_id: FROM_CHAIN_ID,
			gas_per_byte: 1000,
			base_gas_amount: 100,
			gas_price: 100,
			price_ratio: Percent::from_percent(1),
			protocol_ratio: Percent::from_percent(0),
		};
		pallet_bcmp::ChainToGasConfig::<Runtime>::insert(FROM_CHAIN_ID, config);

		let dst_chain = FROM_CHAIN_ID;
		let receiver: [u8; 20] = [1u8; 20];
		let fee_standard = pallet_bcmp::ChainToGasConfig::<Runtime>::get(&dst_chain);
		let mut payload = CrossInOut::get_cross_out_payload(
			XcmOperationType::TransferTo,
			FIL,
			amount,
			Some(&receiver),
		)
		.unwrap();
		let total_fee = Bcmp::calculate_total_fee(payload.len() as u64, fee_standard);

		let src_anchor = <Runtime as crate::Config>::AnchorAddress::get();

		let fee_standard = pallet_bcmp::ChainToGasConfig::<Runtime>::get(&FROM_CHAIN_ID);

		assert_ok!(Bcmp::send_message(ALICE, total_fee, src_anchor, FROM_CHAIN_ID, payload));

		assert_ok!(CrossInOut::cross_out(RuntimeOrigin::signed(ALICE), FIL, amount, FIL));

		assert_eq!(Tokens::free_balance(FIL, &ALICE), endowed_amount - amount);
	});
}

#[test]
fn cross_out_vfil_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let amount = ONE_FIL;

		// endow some FIL to ALICE
		let endowed_amount = 5 * ONE_FIL;
		assert_ok!(Tokens::deposit(VFIL, &ALICE, endowed_amount));
		assert_eq!(Tokens::free_balance(VFIL, &ALICE), endowed_amount);

		// initialize cross-in-out pallet and bcmp pallet
		initialize_cross_in_out();
		let cmt_pair = sp_core::ed25519::Pair::generate().0;
		initialize_pallet_bcmp(cmt_pair);

		assert_ok!(CrossInOut::cross_out(RuntimeOrigin::signed(ALICE), VFIL, amount, FIL));

		assert_eq!(Tokens::free_balance(VFIL, &ALICE), endowed_amount - amount);
	});
}

#[test]
fn send_message_to_anchor_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let amount = ONE_FIL;

		// initialize cross-in-out pallet and bcmp pallet
		initialize_cross_in_out();
		let cmt_pair = sp_core::ed25519::Pair::generate().0;
		initialize_pallet_bcmp(cmt_pair);

		let receiver: [u8; 20] = [1u8; 20];

		// can only get the first 3 32 bytes
		let payload = CrossInOut::get_cross_out_payload(
			XcmOperationType::TransferTo,
			FIL,
			amount,
			Some(&receiver),
		)
		.unwrap();

		assert_ok!(CrossInOut::send_message_to_anchor(ALICE, FROM_CHAIN_ID, &payload));
	});
}
