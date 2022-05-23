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

//! Cross-chain transfer tests within Kusama network.

use bifrost_polkadot_runtime::PolkadotXcm;
use bifrost_slp::{
	primitives::{
		SubstrateLedgerUpdateEntry, SubstrateValidatorsByDelegatorUpdateEntry, UnlockChunk,
	},
	Delays, Ledger, LedgerUpdateEntry, MinimumsMaximums, SubstrateLedger,
	ValidatorsByDelegatorUpdateEntry, XcmOperation,
};
use cumulus_primitives_core::relay_chain::HashT;
use frame_support::{assert_ok, BoundedVec};
use node_primitives::TimeUnit;
use orml_traits::MultiCurrency;
use pallet_staking::{Nominations, StakingLedger};
use pallet_xcm::QueryStatus;
use sp_core::H160;
use xcm::{latest::prelude::*, VersionedMultiAssets, VersionedMultiLocation};
use xcm_emulator::TestExt;

use crate::{integration_tests::*, kusama_test_net::*};

/// ****************************************************
/// *********  Preparation section  ********************
/// ****************************************************
// parachain 2001, subaccount index 0 and index 1
fn para_h160_and_account_id_20_for_2001() -> (H160, [u8; 20]) {
	// 5Ec4AhPV91i9yNuiWuNunPf6AQCYDhFTTA4G5QCbtqYApH9E
	let para_account_2001: H160 =
		hex_literal::hex!["7369626cd1070000000000000000000000000000"].into();
	let account_id_20: [u8; 20] =
		hex_literal::hex!["7369626cd1070000000000000000000000000000"].into();

	(para_account_2001, account_id_20)
}

fn subaccount_0_h160_and_account_id_20() -> (H160, [u8; 20]) {
	// subaccountId0: 0x863c1faef3c3b8f8735ecb7f8ed18996356dd3de
	let subaccount_0: H160 = hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();
	let account_id_20_0: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	(subaccount_0, account_id_20_0)
}

fn subaccount_1_h160_and_account_id_20() -> (H160, [u8; 20]) {
	// subaccountId1: 0x3afe20b0c85801b74e65586fe7070df827172574
	let subaccount_1: H160 = hex_literal::hex!["3afe20b0c85801b74e65586fe7070df827172574"].into();
	let account_id_20_1: [u8; 20] =
		hex_literal::hex!["3afe20b0c85801b74e65586fe7070df827172574"].into();

	(subaccount_1, account_id_20_1)
}
