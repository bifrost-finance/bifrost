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
use frame_support::{assert_noop, assert_ok, sp_io};
use hex_literal::hex;
use sp_core::{bounded::BoundedVec, ConstU32};

const EVM_ADDR: [u8; 20] = hex!["573394b77fC17F91E9E67F147A9ECe24d67C5073"];

#[test]
fn test_xcm_action_util() {
	sp_io::TestExternalities::default().execute_with(|| {
		let address = H160::from_slice(&EVM_ADDR);
		let account_id: AccountId = XcmAction::h160_to_account_id(address);
		assert_eq!(
			account_id,
			sp_runtime::AccountId32::new(hex!(
				"b1c2dde9e562a738e264a554e467b30e5cd58e95ab98459946fb8e518cfe71c2"
			))
		);
		let public_key: [u8; 32] = account_id.encode().try_into().unwrap();
		assert_eq!(
			public_key,
			hex!("b1c2dde9e562a738e264a554e467b30e5cd58e95ab98459946fb8e518cfe71c2")
		);

		assert_ok!(XcmAction::add_whitelist(
			RuntimeOrigin::signed(ALICE),
			SupportChain::Astar,
			ALICE
		));
		assert_eq!(
			XcmAction::whitelist_account_ids(SupportChain::Astar),
			BoundedVec::<AccountId, ConstU32<10>>::try_from(vec![ALICE]).unwrap()
		);
		assert_noop!(
			XcmAction::add_whitelist(RuntimeOrigin::signed(ALICE), SupportChain::Astar, ALICE),
			Error::<Test>::AccountIdAlreadyInWhitelist
		);
		assert_ok!(XcmAction::remove_whitelist(
			RuntimeOrigin::signed(ALICE),
			SupportChain::Astar,
			ALICE
		));
		assert_eq!(
			XcmAction::whitelist_account_ids(SupportChain::Astar),
			BoundedVec::<AccountId, ConstU32<10>>::default()
		);

		assert_ok!(XcmAction::set_execution_fee(
			RuntimeOrigin::signed(ALICE),
			CurrencyId::Token2(0),
			10
		));
		assert_eq!(XcmAction::execution_fee(CurrencyId::Token2(0)), Some(10));

		assert_ok!(XcmAction::set_transfer_to_fee(
			RuntimeOrigin::signed(ALICE),
			SupportChain::Moonbeam,
			10
		));
		assert_eq!(XcmAction::transfer_to_fee(SupportChain::Moonbeam), Some(10));
	});
}
