// Copyright 2019-2020 Liebi Technologies.
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

//! Helper to run commands against current node RPC

use futures::Future;
use hyper::rt;
use jsonrpc_core_client::transports::http;
use node_primitives::Hash;
use sc_rpc::{author::AuthorClient, offchain::OffchainClient};
use sp_core::{offchain::StorageKind, Bytes};

pub struct RpcClient {
    url: String,
}

impl RpcClient {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    pub fn insert_key(&self, key_type: String, suri: String, public: Bytes) {
        let url = self.url.clone();

        rt::run(
            http::connect(&url)
                .and_then(|client: AuthorClient<Hash, Hash>| {
                    client.insert_key(key_type, suri, public).map(|_| ())
                })
                .map_err(|e| {
                    eprintln!("Error inserting key: {:?}", e);
                }),
        );
    }

    // bifrost code
    pub fn get_offchain_storage(&self, prefix: StorageKind, key: Bytes) {
        let url = self.url.clone();

        rt::run(
            http::connect(&url)
                .and_then(move |client: OffchainClient| {
                    client
                        .get_local_storage(prefix, key.clone())
                        .map(move |ret| match ret {
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

    // bifrost code
    pub fn set_offchain_storage(&self, prefix: StorageKind, key: Bytes, value: Bytes) {
        let url = self.url.clone();

        rt::run(
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
}
