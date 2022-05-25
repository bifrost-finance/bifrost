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
use crate::{integration_tests::*, kusama_test_net::*};
use frame_support::assert_ok;
// use xcm::latest::prelude::*;
use crate::slp_tests::para_account_2001;
use xcm_emulator::TestExt;

#[test]
fn kusama_treasury_propose_spend() {
	let amount_to_fund = 50_000_000_000_000_000;

	KusamaNet::execute_with(|| {
		assert_ok!(kusama_runtime::Treasury::propose_spend(
			kusama_runtime::Origin::signed(ALICE.into()),
			amount_to_fund,
			sp_runtime::MultiAddress::Id(para_account_2001()),
		));
	});
}

#[test]
fn bifrost_issuance_ksm_transfer_to_treasury() {
	Bifrost::execute_with(|| {
		let treasury_derivative_account_id = bifrost_kusama_runtime::Utility::derivative_account_id(
			bifrost_kusama_runtime::TreasuryPalletId::get().into_account(),
			0,
		);
		assert_ok!(bifrost_kusama_runtime::Tokens::set_balance(
			bifrost_kusama_runtime::Origin::root(),
			sp_runtime::MultiAddress::Id(treasury_derivative_account_id.clone()),
			RelayCurrencyId::get(),
			50_000_000_000_000_000,
			0,
		));
		assert_ok!(bifrost_kusama_runtime::Tokens::force_transfer(
			bifrost_kusama_runtime::Origin::root(),
			sp_runtime::MultiAddress::Id(treasury_derivative_account_id),
			sp_runtime::MultiAddress::Id(
				bifrost_kusama_runtime::TreasuryPalletId::get().into_account()
			),
			RelayCurrencyId::get(),
			50_000_000_000_000_000,
		));
	});
}
