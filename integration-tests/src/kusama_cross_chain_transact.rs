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

use frame_support::{assert_ok, traits::Currency};
use xcm::latest::prelude::*;
use xcm_emulator::TestExt;

use crate::{integration_tests::*, kusama_test_net::*};

pub type ParaBalances = pallet_balances::Pallet<Runtime>;

pub type ParaTokens = orml_tokens::Pallet<Runtime>;

pub type Salp = bifrost_salp::Pallet<Runtime>;

pub type RelayChainPalletXcm = pallet_xcm::Pallet<kusama_runtime::Runtime>;

pub use bifrost_kusama_runtime::SlotLength;
use frame_system::RawOrigin;

use crate::{integration_tests::*, kusama_test_net::*};

#[test]
fn transact_transfer_call_from_relaychain_works() {
	Bifrost::execute_with(|| {
		let _ = ParaBalances::deposit_creating(
			&AccountId::from(ALICE),
			1000 * dollar(NativeCurrencyId::get()),
		);
	});

	let alice = Junctions::X1(Junction::AccountId32 { network: NetworkId::Kusama, id: ALICE });
	let call = Call::Balances(pallet_balances::Call::<Runtime>::transfer {
		dest: MultiAddress::Id(AccountId::from(BOB)),
		value: 500 * dollar(NativeCurrencyId::get()),
	});
	let assets: MultiAsset = (Parent, dollar(RelayCurrencyId::get())).into();

	KusamaNet::execute_with(|| {
		let xcm = vec![
			WithdrawAsset(assets.clone().into()),
			BuyExecution {
				fees: assets,
				weight_limit: Limited(dollar(RelayCurrencyId::get()) as u64),
			},
			Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: (dollar(RelayCurrencyId::get()) as u64) / 10 as u64,
				call: call.encode().into(),
			},
			DepositAsset {
				assets: All.into(),
				max_assets: 1,
				beneficiary: { (1, alice.clone()).into() },
			},
		];
		assert_ok!(RelayChainPalletXcm::send_xcm(alice, Parachain(2001).into(), Xcm(xcm),));
	});

	Bifrost::execute_with(|| {
		use Event;
		use System;
		assert_eq!(
			9991920000000,
			ParaTokens::free_balance(RelayCurrencyId::get(), &AccountId::from(ALICE))
		);
		assert_eq!(
			500 * dollar(NativeCurrencyId::get()),
			ParaBalances::free_balance(&AccountId::from(ALICE))
		);
		assert_eq!(
			500 * dollar(NativeCurrencyId::get()),
			ParaBalances::free_balance(&AccountId::from(BOB))
		);
		System::assert_has_event(Event::Balances(pallet_balances::Event::Transfer {
			from: AccountId::from(ALICE),
			to: AccountId::from(BOB),
			amount: 500 * dollar(NativeCurrencyId::get()),
		}));
	});
}

#[test]
fn transact_salp_contribute_call_from_relaychain_works() {
	Bifrost::execute_with(|| {
		assert_ok!(Salp::create(
			RawOrigin::Root.into(),
			3_000,
			100 * dollar(RelayCurrencyId::get()),
			1,
			SlotLength::get()
		));
		assert_ok!(Salp::funds(3_000).ok_or(()));
	});

	let alice = Junctions::X1(Junction::AccountId32 { network: NetworkId::Kusama, id: ALICE });
	let call = Call::Salp(bifrost_salp::Call::<Runtime>::contribute {
		index: 3000,
		value: 1 * dollar(RelayCurrencyId::get()),
	});
	let assets: MultiAsset = (Parent, dollar(RelayCurrencyId::get())).into();

	KusamaNet::execute_with(|| {
		let xcm = vec![
			WithdrawAsset(assets.clone().into()),
			BuyExecution {
				fees: assets,
				weight_limit: Limited(dollar(RelayCurrencyId::get()) as u64),
			},
			Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: (dollar(RelayCurrencyId::get()) as u64) / 10 as u64,
				call: call.encode().into(),
			},
			DepositAsset {
				assets: All.into(),
				max_assets: 1,
				beneficiary: { (1, alice.clone()).into() },
			},
		];
		assert_ok!(RelayChainPalletXcm::send_xcm(alice, Parachain(2001).into(), Xcm(xcm),));
	});
}

#[test]
fn transact_to_relaychain_works() {
	let remark = kusama_runtime::Call::System(
		frame_system::Call::<kusama_runtime::Runtime>::remark_with_event {
			remark: "Hello from Bifrost!".as_bytes().to_vec(),
		},
	);

	let asset: MultiAsset =
		MultiAsset { id: Concrete(MultiLocation::here()), fun: Fungible(8000000000) };

	let msg = Xcm(vec![
		WithdrawAsset(asset.clone().into()),
		BuyExecution { fees: asset, weight_limit: WeightLimit::Limited(6000000000) },
		Transact {
			origin_type: OriginKind::SovereignAccount,
			require_weight_at_most: 2000000000 as u64,
			call: remark.encode().into(),
		},
	]);

	Bifrost::execute_with(|| {
		assert_ok!(pallet_xcm::Pallet::<Runtime>::send_xcm(Here, Parent, msg));
	});

	KusamaNet::execute_with(|| {
		use kusama_runtime::{Event, System};
		assert!(System::events().iter().any(|r| matches!(
			r.event,
			Event::System(frame_system::Event::Remarked { sender: _, hash: _ })
		)));
	});
}
