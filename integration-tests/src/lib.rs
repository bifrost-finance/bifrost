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
#[cfg(feature = "with-bifrost-kusama-runtime")]
mod integration_tests;
#[cfg(feature = "with-bifrost-kusama-runtime")]
mod kusama_cross_chain_transact;
#[cfg(feature = "with-bifrost-kusama-runtime")]
mod kusama_cross_chain_transfer;
#[cfg(feature = "with-bifrost-kusama-runtime")]
mod kusama_test_net;
#[cfg(feature = "with-bifrost-kusama-runtime")]
mod slp_tests;
#[cfg(feature = "with-bifrost-kusama-runtime")]
mod statemine;
#[cfg(feature = "with-bifrost-kusama-runtime")]
mod treasury;
