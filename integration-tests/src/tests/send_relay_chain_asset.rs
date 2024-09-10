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

use crate::mock::{
	Bifrost, BifrostAssetRegistry, BifrostTokens, BifrostXTokens, Relay, RelayBalances,
	RelaySystem, TestNet, ALICE, BOB,
};
use bifrost_primitives::CurrencyId;
use cumulus_primitives_core::ParaId;
use frame_support::{assert_ok, traits::Currency};
use orml_traits::MultiCurrency;
use sp_runtime::traits::AccountIdConversion;
use xcm::v4::{Junction, Location, WeightLimit};
use xcm_simulator::TestExt;

#[test]
fn send_relay_chain_asset_to_relay_chain() {
	TestNet::reset();

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(
			&ParaId::from(2030).into_account_truncating(),
			100_000_000_000,
		);
	});

	Bifrost::execute_with(|| {
		assert_ok!(BifrostAssetRegistry::do_register_location(
			CurrencyId::Token2(0),
			&Location::parent()
		));
		assert_ok!(BifrostXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::Token2(0),
			50_000_000_000,
			Box::new(
				Location::new(1, [Junction::AccountId32 { network: None, id: BOB.into() }]).into()
			),
			WeightLimit::Unlimited
		));
		assert_eq!(BifrostTokens::free_balance(CurrencyId::Token2(0), &ALICE), 50_000_000_000);
	});

	Relay::execute_with(|| {
		println!("{:?}", RelaySystem::events());

		assert_eq!(
			RelayBalances::free_balance(&ParaId::from(2030).into_account_truncating()),
			50_000_000_000
		);
		assert_eq!(RelayBalances::free_balance(&BOB), 49999999960);
	});
}
