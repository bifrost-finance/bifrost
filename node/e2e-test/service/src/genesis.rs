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

use codec::Encode;
use cumulus_client_service::genesis::generate_genesis_block;
use cumulus_primitives_core::ParaId;
use cumulus_test_runtime::Block;
use polkadot_primitives::v0::HeadData;
use sp_runtime::traits::Block as BlockT;

/// Returns the initial head data for a parachain ID.
pub fn initial_head_data(_para_id: ParaId) -> HeadData {
	#[cfg(any(feature = "with-bifrost-kusama-test-runtime", feature = "with-all-runtime"))]
	let spec = Box::new(node_service::chain_spec::bifrost_kusama::local_testnet_config().unwrap());
	let block: Block = generate_genesis_block(&(spec as Box<_>)).unwrap();
	let genesis_state = block.header().encode();
	genesis_state.into()
}
