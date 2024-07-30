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

#[cfg(any(feature = "with-bifrost-kusama-runtime", feature = "with-bifrost-runtime"))]
pub mod bifrost_kusama;
#[cfg(any(feature = "with-bifrost-polkadot-runtime", feature = "with-bifrost-runtime"))]
pub mod bifrost_polkadot;

use bifrost_primitives::BlockNumber;
pub use bifrost_primitives::{AccountId, Block, Signature};
use sc_chain_spec::ChainSpecExtension;
use serde::{Deserialize, Serialize};
use sp_core::{Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

pub const TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

type AccountPublic = <Signature as Verify>::Signer;

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct RelayExtensions {
	/// The relay chain of the Parachain.
	pub relay_chain: String,
	/// The id of the Parachain.
	pub para_id: u32,
	/// EVM compatible starting block number
	pub evm_since: BlockNumber,
}

impl RelayExtensions {
	/// Try to get the extension from the given `ChainSpec`.
	#[allow(clippy::borrowed_box)]
	pub fn try_get(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}
