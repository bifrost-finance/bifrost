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

use bifrost_primitives::{CurrencyId, TokenSymbol};
use integration_tests_common::BifrostKusama;
use parity_scale_codec::Encode;
use sp_runtime::BoundedVec;
use xcm::prelude::*;
use xcm_emulator::TestExt;

#[test]
fn dollar_should_work() {
	BifrostKusama::execute_with(|| {
		let id = CurrencyId::Token(TokenSymbol::KSM);
		assert_eq!(
			Junction::from(BoundedVec::try_from(id.encode()).unwrap()),
			GeneralKey {
				length: 2,
				data: [
					2, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
					0, 0, 0, 0, 0, 0
				]
			}
		);
	});
}
