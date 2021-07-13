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

#[cfg(test)]
mod tests {
	use codec::Encode;
	use frame_support::assert_ok;
	use xcm::v0::{
		Junction::{self, Parachain, Parent},
		MultiAsset::*,
		MultiLocation::*,
		NetworkId, OriginKind,
		Xcm::*,
	};

	use crate::mock::*;

	fn print_events<T: frame_system::Config>(context: &str) {
		println!("------ {:?} events ------", context);
		frame_system::Pallet::<T>::events().iter().for_each(|r| {
			println!("{:?}", r.event);
		});
	}

	#[test]
	fn reserve_transfer() {
		MockNet::reset();

		Relay::execute_with(|| {
			assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
				relay::Origin::signed(ALICE),
				X1(Parachain(1)),
				X1(Junction::AccountId32 { network: NetworkId::Any, id: ALICE.into() }),
				vec![ConcreteFungible { id: Null, amount: 123 }],
				123,
			));
		});

		ParaA::execute_with(|| {
			// free execution, full amount received
			assert_eq!(
				pallet_balances::Pallet::<para::Runtime>::free_balance(&ALICE),
				INITIAL_BALANCE + 123
			);

			print_events::<para::Runtime>("ParaA");
		});
	}

	#[test]
	fn dmp() {
		MockNet::reset();

		let remark =
			para::Call::System(frame_system::Call::<para::Runtime>::remark_with_event(vec![
				1, 2, 3,
			]));
		Relay::execute_with(|| {
			assert_ok!(RelayChainPalletXcm::send_xcm(
				Null,
				X1(Parachain(1)),
				Transact {
					origin_type: OriginKind::SovereignAccount,
					require_weight_at_most: INITIAL_BALANCE as u64,
					call: remark.encode().into(),
				},
			));
		});

		ParaA::execute_with(|| {
			print_events::<para::Runtime>("ParaA");
		});
	}

	#[test]
	fn ump() {
		MockNet::reset();

		let remark =
			relay::Call::System(frame_system::Call::<relay::Runtime>::remark_with_event(vec![
				1, 2, 3,
			]));
		ParaA::execute_with(|| {
			assert_ok!(ParachainPalletXcm::send_xcm(
				Null,
				X1(Parent),
				Transact {
					origin_type: OriginKind::SovereignAccount,
					require_weight_at_most: INITIAL_BALANCE as u64,
					call: remark.encode().into(),
				},
			));
		});

		Relay::execute_with(|| {
			print_events::<relay::Runtime>("RelayChain");
		});
	}

	#[test]
	fn xcmp() {
		MockNet::reset();

		let remark =
			para::Call::System(frame_system::Call::<para::Runtime>::remark_with_event(vec![
				1, 2, 3,
			]));
		ParaA::execute_with(|| {
			assert_ok!(ParachainPalletXcm::send_xcm(
				Null,
				X2(Parent, Parachain(2)),
				Transact {
					origin_type: OriginKind::SovereignAccount,
					require_weight_at_most: INITIAL_BALANCE as u64,
					call: remark.encode().into(),
				},
			));
		});

		ParaB::execute_with(|| {
			print_events::<para::Runtime>("ParaB");
		});
	}
}
