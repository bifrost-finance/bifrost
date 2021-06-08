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

// use sc_service::{TFullBackend, TFullClient};

pub use collator::*;
pub use client::*;

pub mod chain_spec;
pub mod collator;
mod client;


/// Can be called for a `Configuration` to check if it is a configuration for the `Bifrost` network.
pub trait IdentifyVariant {
    /// Returns if this is a configuration for the `Asgard` network.
    fn is_asgard(&self) -> bool;

    /// Returns if this is a configuration for the `Bifrost` network.
    fn is_bifrost(&self) -> bool;
}

impl IdentifyVariant for Box<dyn sc_service::ChainSpec> {
    fn is_asgard(&self) -> bool {
        self.id().starts_with("asgard") || self.id().starts_with("asg")
    }

    fn is_bifrost(&self) -> bool {
        self.id().starts_with("bifrost") || self.id().starts_with("bnc")
    }
}