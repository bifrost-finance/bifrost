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

use crate::mock::{bifrost, Bifrost, BifrostSlp, Relay, RelayBalances, RelayXcmPallet};
use bifrost_polkadot_runtime::xcm_config::ParaId;
use bifrost_primitives::CurrencyId;
use bifrost_slp::MinimumsMaximums;
use frame_support::{assert_ok, traits::Currency};
use sp_runtime::{
	traits::{AccountIdConversion, Convert},
	AccountId32,
};
use std::convert::Into;
use xcm::{v3::MultiLocation, v4::prelude::*, VersionedAssets, VersionedLocation};
use xcm_simulator::TestExt;

const ENTRANCE_ACCOUNT: [u8; 32] =
	hex_literal::hex!["6d6f646c62662f76746b696e0000000000000000000000000000000000000000"];
const BIFROST_TREASURY_ACCOUNT: [u8; 32] =
	hex_literal::hex!["6d6f646c62662f74727372790000000000000000000000000000000000000000"];
const BIFROST_TREASURY_MULTILOCATION: MultiLocation = MultiLocation {
	parents: 0,
	interior: xcm::v3::Junctions::X1(xcm::v3::Junction::AccountId32 {
		network: None,
		id: BIFROST_TREASURY_ACCOUNT,
	}),
};
pub const KUSAMA_ALICE_STASH_ACCOUNT: [u8; 32] =
	hex_literal::hex!["be5ddb1579b72e84524fc29e78609e3caf42e85aa118ebfe0b0ad404b5bdd25f"];
const KUSAMA_ALICE_STASH_MULTILOCATION: MultiLocation = MultiLocation {
	parents: 1,
	interior: xcm::v3::Junctions::X1(xcm::v3::Junction::AccountId32 {
		network: None,
		id: KUSAMA_ALICE_STASH_ACCOUNT,
	}),
};

const DOT_DECIMALS: u128 = 10_000_000_000;

fn cross_dot_to_bifrost(to: AccountId32, amount: u128) {
	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(
			&ParaId::from(2030).into_account_truncating(),
			100_000_000_000,
		);
		let _ = RelayBalances::deposit_creating(&to, amount);
		assert_ok!(RelayXcmPallet::limited_reserve_transfer_assets(
			Some(to.clone()).into(),
			Box::new(VersionedLocation::V4(Parachain(2030).into())),
			Box::new(VersionedLocation::V4(
				Junction::AccountId32 { id: to.into(), network: None }.into()
			)),
			Box::new(VersionedAssets::V4((Here, amount / 10).into())),
			0,
			Unlimited
		));
	});
}

fn slp_setup() {
	cross_dot_to_bifrost(BIFROST_TREASURY_ACCOUNT.into(), 10000 * DOT_DECIMALS);
	cross_dot_to_bifrost(ENTRANCE_ACCOUNT.into(), 10000 * DOT_DECIMALS);

	let mins_and_maxs = MinimumsMaximums {
		delegator_bonded_minimum: DOT_DECIMALS / 10,
		bond_extra_minimum: DOT_DECIMALS / 1000,
		unbond_minimum: DOT_DECIMALS / 1000,
		rebond_minimum: DOT_DECIMALS / 1000,
		unbond_record_maximum: 32,
		validators_back_maximum: 24,
		delegator_active_staking_maximum: 80000 * DOT_DECIMALS,
		validators_reward_maximum: 256,
		delegation_amount_minimum: DOT_DECIMALS / 1000,
		delegators_maximum: 100,
		validators_maximum: 300,
	};

	Bifrost::execute_with(|| {
		assert_ok!(BifrostSlp::set_minimums_and_maximums(
			bifrost::RuntimeOrigin::root(),
			CurrencyId::Token2(0),
			Some(mins_and_maxs)
		));

		// set fee_source for ksm to be treasury
		assert_ok!(BifrostSlp::set_fee_source(
			bifrost::RuntimeOrigin::root(),
			CurrencyId::Token2(0),
			Some((BIFROST_TREASURY_MULTILOCATION, 1 * DOT_DECIMALS))
		));

		assert_ok!(BifrostSlp::initialize_delegator(
			bifrost::RuntimeOrigin::root(),
			CurrencyId::Token2(0),
			None
		));

		assert_ok!(BifrostSlp::add_validator(
			bifrost::RuntimeOrigin::root(),
			CurrencyId::Token2(0),
			Box::new(KUSAMA_ALICE_STASH_MULTILOCATION),
		));
	})
}

#[test]
fn relaychain_staking_bond() {
	slp_setup();
	Bifrost::execute_with(|| {
		let delegator = bifrost_polkadot_runtime::SubAccountIndexMultiLocationConvertor::convert((
			0,
			CurrencyId::Token2(0),
		));
		assert_ok!(BifrostSlp::bond(
			bifrost::RuntimeOrigin::root(),
			CurrencyId::Token2(0),
			Box::new(delegator),
			50 * DOT_DECIMALS,
			None,
			Some((Weight::from_parts(10000000000u64, 65535u64), 100000000u128))
		));
	});
}
