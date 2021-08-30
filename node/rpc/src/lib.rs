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

//! A collection of node-specific RPC methods.
//!
//! Since `substrate` core functionality makes no assumptions
//! about the modules used inside the runtime, so do
//! RPC methods defined in `sc-rpc` crate.
//! It means that `client/rpc` can't have any methods that
//! need some strong assumptions about the particular runtime.
//!
//! The RPCs available in this crate however can make some assumptions
//! about how the runtime is constructed and what FRAME pallets
//! are part of it. Therefore all node-runtime-specific RPCs can
//! be placed here or imported from corresponding FRAME RPC definitions.

#![warn(missing_docs)]

use std::sync::Arc;

use bifrost_flexible_fee_rpc::{FeeRpcApi, FlexibleFeeStruct};
use bifrost_flexible_fee_rpc_runtime_api::FlexibleFeeRuntimeApi as FeeRuntimeApi;
use bifrost_salp_rpc_runtime_api::SalpRuntimeApi;
use node_primitives::{AccountId, Balance, Block, ParaId};
pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use zenlink_protocol_rpc::{ZenlinkProtocol, ZenlinkProtocolApi};
use zenlink_protocol_runtime_api::ZenlinkProtocolApi as ZenlinkProtocolRuntimeApi;

/// Full client dependencies.
pub struct FullDeps<C, P> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
}

/// A IO handler that uses all Full RPC extensions.
pub type RpcExtension = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// Instantiate all Full RPC extensions.
///
/// NOTE: It's a `PATCH` for the RPC of asgard runtime.
#[allow(non_snake_case)]
pub fn PATCH_FOR_ASGARD_create_full<C, P>(
	deps: FullDeps<C, P>,
) -> Result<jsonrpc_core::IoHandler<sc_rpc_api::Metadata>, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
	C: Send + Sync + 'static,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: FeeRuntimeApi<Block, AccountId>,
	C::Api: ZenlinkProtocolRuntimeApi<Block, AccountId>,
	C::Api: SalpRuntimeApi<Block, ParaId, AccountId, Balance>,
	P: TransactionPool + 'static,
{
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};

	let FullDeps { client, .. } = deps;

	let mut io = RpcExtension::default();

	io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(client.clone())));

	use bifrost_flexible_fee_rpc::{FeeRpcApi, FlexibleFeeStruct};
	use bifrost_salp_rpc_api::{SalpRpcApi, SalpRpcWrapper};
	use zenlink_protocol_rpc::{ZenlinkProtocol, ZenlinkProtocolApi};

	io.extend_with(FeeRpcApi::to_delegate(FlexibleFeeStruct::new(client.clone())));

	io.extend_with(ZenlinkProtocolApi::to_delegate(ZenlinkProtocol::new(client.clone())));

	io.extend_with(SalpRpcApi::to_delegate(SalpRpcWrapper::new(client.clone())));

	io
}
