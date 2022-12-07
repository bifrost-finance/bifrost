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

use frame_support::assert_ok;
use xcm::latest::prelude::*;
use xcm_emulator::TestExt;

use crate::{kusama_integration_tests::*, kusama_test_net::*};

#[test]
fn relaychain_transact_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		let remark = kusama_runtime::RuntimeCall::System(frame_system::Call::<
			kusama_runtime::Runtime,
		>::remark_with_event {
			remark: "Hello from Bifrost!".as_bytes().to_vec(),
		});

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
			use kusama_runtime::{RuntimeEvent, System};
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::System(frame_system::Event::Remarked { sender: _, hash: _ })
			)));
		});
	})
}
