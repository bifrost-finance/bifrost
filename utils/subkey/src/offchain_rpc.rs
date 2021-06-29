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

use futures::Future;
use jsonrpc_core_client::transports::http;
use sc_rpc::offchain::OffchainClient;
use sp_core::{offchain::StorageKind, Bytes};

pub fn get_offchain_storage(url: &str, prefix: StorageKind, key: Bytes) {
	tokio::run(
		http::connect(&url)
			.and_then(move |client: OffchainClient| {
				client.get_local_storage(prefix, key.clone()).map(move |ret| match ret {
					Some(value) => println!(
						"Value of key(0x{}) is 0x{}",
						hex::encode(&*key),
						hex::encode(&*value),
					),
					None => println!("Value of key(0x{}) not exists", hex::encode(&*key)),
				})
			})
			.map_err(|e| {
				println!("Error getting local storage: {:?}", e);
			}),
	);
}

pub fn set_offchain_storage(url: &str, prefix: StorageKind, key: Bytes, value: Bytes) {
	tokio::run(
		http::connect(&url)
			.and_then(move |client: OffchainClient| {
				client.set_local_storage(prefix, key, value).map(|_| {
					println!("Set local storage successfully");
				})
			})
			.map_err(|e| {
				println!("Error setting local storage: {:?}", e);
			}),
	);
}
