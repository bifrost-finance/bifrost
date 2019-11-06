// Copyright 2019 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

//! Tests for the module.

#![cfg(test)]

use super::*;
use crate::mock::*;
use sr_primitives::traits::OffchainWorker;
use sr_primitives::traits::OnFinalize;
use substrate_offchain::testing::TestOffchainExt;
use substrate_primitives::offchain::{OpaquePeerId, OffchainExt};
use srml_support::{assert_ok};

#[test]
fn relay_tx_should_work() {
	new_test_ext().execute_with(|| {
		Bridge::relay_tx(Origin::signed(1), 100, 100);
	});
}

#[test]
fn send_tx_gen_should_work() {
	let mut ext = new_test_ext();
	let (offchain, state) = TestOffchainExt::new();
	ext.register_extension(OffchainExt::new(offchain));

	ext.execute_with(|| {
		Bridge::send_tx_gen(0, 100, 123, b"testx".to_vec());
		Bridge::send_tx_sign();
		assert_eq!(<Bridge as Store>::UnsignedSends::decode_len().unwrap_or(0), 1);

		assert_ok!(Bridge::send_tx_update(Origin::NONE, 10));

		assert_eq!(<Bridge as Store>::UnsignedSends::decode_len().unwrap_or(0), 0);
	});
}
