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

pub mod chain_spec;
#[cfg(feature = "with-asgard-runtime")]
pub mod collator_asgard;
#[cfg(feature = "with-bifrost-kusama-runtime")]
pub mod collator_kusama;
#[cfg(feature = "with-bifrost-polkadot-runtime")]
pub mod collator_polkadot;
#[cfg(feature = "with-asgard-runtime")]
pub mod dev;
pub use node_rpc as rpc;

/// Can be called for a `Configuration` to check if it is a configuration for the `Bifrost` network.
pub trait IdentifyVariant {
	/// Returns if this is a configuration for the `Asgard` network.
	fn is_asgard(&self) -> bool;

	/// Returns if this is a configuration for the `Bifrost-Kusama` network.
	fn is_bifrost_kusama(&self) -> bool;

	/// Returns if this is a configuration for the `Bifrost-Polkadot` network.
	fn is_bifrost_polkadot(&self) -> bool;

	/// Returns if this is a configuration for the `Dev` network.
	fn is_dev(&self) -> bool;
}

impl IdentifyVariant for Box<dyn sc_service::ChainSpec> {
	fn is_asgard(&self) -> bool {
		self.id().starts_with("asgard")
	}

	fn is_bifrost_kusama(&self) -> bool {
		self.id().starts_with("bifrost") && !self.id().starts_with("bifrost_polkadot")
	}

	fn is_bifrost_polkadot(&self) -> bool {
		self.id().starts_with("bifrost_polkadot")
	}

	fn is_dev(&self) -> bool {
		self.id().starts_with("dev")
	}
}

pub const ASGARD_RUNTIME_NOT_AVAILABLE: &str =
	"Asgard runtime is not available. Please compile the node with `--features with-asgard-runtime` to enable it.";
pub const BIFROST_KUSAMA_RUNTIME_NOT_AVAILABLE: &str =
	"Bifrost runtime is not available. Please compile the node with `--features with-bifrost-kusama-runtime` to enable it.";
pub const BIFROST_POLKADOT_RUNTIME_NOT_AVAILABLE: &str =
	"Bifrost-polkadot runtime is not available. Please compile the node with `--features with-bifrost-polkadot-runtime` to enable it.";
pub const UNKNOWN_RUNTIME: &str = "Unknown runtime";
