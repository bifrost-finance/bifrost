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

#![warn(unused_extern_crates)]

//! Service implementation. Specialized wrapper over substrate service.
pub use collator::*;
pub mod chain_spec;
pub mod collator;
#[cfg(feature = "with-asgard-runtime")]
pub use asgard_runtime;
#[cfg(feature = "with-bifrost-runtime")]
pub use bifrost_runtime;
use node_rpc as rpc;
mod client;
pub use client::RuntimeApiCollection;
use sc_executor::NativeElseWasmExecutor;
use sc_service::TFullBackend;

#[cfg(feature = "with-asgard-runtime")]
pub struct AsgardExecutor;
#[cfg(feature = "with-asgard-runtime")]
impl sc_executor::NativeExecutionDispatch for AsgardExecutor {
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		asgard_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		asgard_runtime::native_version()
	}
}

#[cfg(feature = "with-bifrost-runtime")]
pub struct BifrostExecutor;
#[cfg(feature = "with-bifrost-runtime")]
impl sc_executor::NativeExecutionDispatch for BifrostExecutor {
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		bifrost_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		bifrost_runtime::native_version()
	}
}

#[cfg(feature = "with-asgard-runtime")]
pub mod dev;

pub type FullBackend = TFullBackend<Block>;

pub type FullClient<RuntimeApi, ExecutorDispatch> =
	sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;

/// Can be called for a `Configuration` to check if it is a configuration for the `Bifrost` network.
pub trait IdentifyVariant {
	/// Returns if this is a configuration for the `Asgard` network.
	fn is_asgard(&self) -> bool;

	/// Returns if this is a configuration for the `Bifrost` network.
	fn is_bifrost(&self) -> bool;

	/// Returns if this is a configuration for the `Dev` network.
	fn is_dev(&self) -> bool;
}

impl IdentifyVariant for Box<dyn sc_service::ChainSpec> {
	fn is_asgard(&self) -> bool {
		self.id().starts_with("asgard") || self.id().starts_with("asg")
	}

	fn is_bifrost(&self) -> bool {
		self.id().starts_with("bifrost") || self.id().starts_with("bnc")
	}

	fn is_dev(&self) -> bool {
		self.id().starts_with("dev")
	}
}

pub const BIFROST_RUNTIME_NOT_AVAILABLE: &str =
	"Bifrost runtime is not available. Please compile the node with `--features with-bifrost-runtime` to enable it.";
pub const ASGARD_RUNTIME_NOT_AVAILABLE: &str =
	"Asgard runtime is not available. Please compile the node with `--features with-asgard-runtime` to enable it.";
pub const UNKNOWN_RUNTIME: &str = "Unknown runtime";
