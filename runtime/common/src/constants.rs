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

//! A set of constant values used for all runtimes in common.

pub mod parachains {
	pub mod kusama {
		pub mod karura {
			pub const ID: u32 = 2000;
			pub const KAR_KEY: &[u8] = &[0, 128];
			pub const KUSD_KEY: &[u8] = &[0, 129];
		}
	}

	pub mod polkadot {
		pub mod acala {
			pub const ID: u32 = 2000;
			pub const ACA_KEY: &[u8] = &[0, 0];
			pub const AUSD_KEY: &[u8] = &[0, 1];
		}
	}
}
