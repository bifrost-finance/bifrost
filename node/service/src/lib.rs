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
pub use client::*;
pub use collator::*;
use sc_executor::native_executor_instance;
pub mod chain_spec;
mod client;
pub mod collator;
#[cfg(feature = "with-asgard-runtime")]
pub use asgard_runtime;
#[cfg(feature = "with-bifrost-runtime")]
pub use bifrost_runtime;
#[cfg(feature = "with-dev-runtime")]
pub mod dev;
#[cfg(feature = "with-dev-runtime")]
pub use dev::*;

#[cfg(feature = "with-asgard-runtime")]
native_executor_instance!(
	pub AsgardExecutor,
	asgard_runtime::api::dispatch,
	asgard_runtime::native_version,
	frame_benchmarking::benchmarking::HostFunctions,
);

#[cfg(feature = "with-bifrost-runtime")]
native_executor_instance!(
	pub BifrostExecutor,
	bifrost_runtime::api::dispatch,
	bifrost_runtime::native_version,
	frame_benchmarking::benchmarking::HostFunctions,
);

/// Can be called for a `Configuration` to check if it is a configuration for the `Bifrost` network.
pub trait IdentifyVariant {
	/// Returns if this is a configuration for the `Asgard` network.
	fn is_asgard(&self) -> bool;

	/// Returns if this is a configuration for the `Bifrost` network.
	fn is_bifrost(&self) -> bool;

	/// Returns if this is a configuration for the `Dev` network.
	fn is_asgard_dev(&self) -> bool;
}

impl IdentifyVariant for Box<dyn sc_service::ChainSpec> {
	fn is_asgard(&self) -> bool {
		self.id().starts_with("asgard") || self.id().starts_with("asg")
	}

	fn is_bifrost(&self) -> bool {
		self.id().starts_with("bifrost") || self.id().starts_with("bnc")
	}

	fn is_asgard_dev(&self) -> bool {
		self.id().starts_with("asgard-dev") || self.id().starts_with("dev")
	}
}

pub const BIFROST_RUNTIME_NOT_AVAILABLE: &str =
	"Bifrost runtime is not available. Please compile the node with `--features with-bifrost-runtime` to enable it.";
pub const ASGARD_RUNTIME_NOT_AVAILABLE: &str =
	"Asgard runtime is not available. Please compile the node with `--features with-asgard-runtime` to enable it.";
pub const DEV_RUNTIME_NOT_AVAILABLE: &str =
	"Dev runtime is not available. Please compile the node with `--features with-dev-runtime` to enable it.";
