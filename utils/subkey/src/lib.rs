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

use structopt::StructOpt;
use sc_cli::{
	Error, VanityCmd, SignCmd, VerifyCmd, GenerateNodeKeyCmd, GenerateCmd, InspectKeyCmd,
	InspectNodeKeyCmd
};

mod offchain_rpc;

#[derive(Debug, StructOpt)]
#[structopt(
	name = "subkey",
	author = "Parity Team <admin@parity.io>",
	about = "Utility for generating and restoring with Substrate keys",
)]
pub enum Subkey {
	/// Generate a random node libp2p key, save it to file or print it to stdout
	/// and print its peer ID to stderr.
	GenerateNodeKey(GenerateNodeKeyCmd),

	/// Generate a random account
	Generate(GenerateCmd),

	/// Gets a public key and a SS58 address from the provided Secret URI
	Inspect(InspectKeyCmd),

	/// Print the peer ID corresponding to the node key in the given file
	InspectNodeKey(InspectNodeKeyCmd),

	/// Sign a message, with a given (secret) key.
	Sign(SignCmd),

	/// Generate a seed that provides a vanity address.
	Vanity(VanityCmd),

	/// Verify a signature for a message, provided on STDIN, with a given (public or secret) key.
	Verify(VerifyCmd),

	/// Get localstorage
	#[structopt(name = "localstorage-get")]
	GetLocalStorage(offchain_storage::GetLocalStorageCmd),

	/// Set localstorage
	#[structopt(name = "localstorage-set")]
	SetLocalStorage(offchain_storage::SetLocalStorageCmd),
}

/// Run the subkey command, given the apropriate runtime.
pub fn run() -> Result<(), Error> {
	match Subkey::from_args() {
		Subkey::GenerateNodeKey(cmd) => cmd.run(),
		Subkey::Generate(cmd) => cmd.run(),
		Subkey::Inspect(cmd) => cmd.run(),
		Subkey::InspectNodeKey(cmd) => cmd.run(),
		Subkey::Vanity(cmd) => cmd.run(),
		Subkey::Verify(cmd) => cmd.run(),
		Subkey::Sign(cmd) => cmd.run(),
		Subkey::GetLocalStorage(cmd) => cmd.run(),
		Subkey::SetLocalStorage(cmd) => cmd.run(),
	}
}

pub mod offchain_storage {
	use crate::offchain_rpc;
	use sp_core::{Bytes, offchain::StorageKind};
	use structopt::StructOpt;

	/// The `localstorage-get` command
	#[derive(Debug, StructOpt)]
	#[structopt(
		name = "localstorage-get",
		about = "Get local storage from current node itself."
	)]
	pub struct GetLocalStorageCmd {
		#[structopt(long)]
		key: String,
		#[structopt(default_value = "http://localhost:9933")]
		url: String,
	}

	impl GetLocalStorageCmd {
		pub fn run(&self) -> Result<(), sc_cli::Error> {
			let prefix = StorageKind::PERSISTENT;
			let key = Bytes(Vec::from(self.key.clone()));

			offchain_rpc::get_offchain_storage(&self.url, prefix, key);
			Ok(())
		}
	}

	/// The `localstorage-set` command
	#[derive(Debug, StructOpt)]
	#[structopt(
		name = "localstorage-set",
		about = "Set local storage for current node itself."
	)]
	pub struct SetLocalStorageCmd {
		#[structopt(long)]
		key: String,
		#[structopt(long)]
		value: String,
		#[structopt(default_value = "http://localhost:9933")]
		url: String,
	}

	impl SetLocalStorageCmd {
		pub fn run(&self) -> Result<(), sc_cli::Error> {
			let prefix = StorageKind::PERSISTENT;
			let key = Bytes(Vec::from(self.key.clone()));
			let value = Bytes(Vec::from(self.value.clone()));

			offchain_rpc::set_offchain_storage(&self.url, prefix, key, value);
			Ok(())
		}
	}
}
