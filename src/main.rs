// Copyright 2019 Liebi Technologies.
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

//! Bifrost Node CLI library.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

mod chain_spec;
mod service;
mod cli;

pub use substrate_cli::{VersionInfo, IntoExit, error};

fn main() {
	let version = VersionInfo {
		name: "Liebi Bifrost",
		commit: env!("VERGEN_SHA_SHORT"),
		version: env!("CARGO_PKG_VERSION"),
		executable_name: "bifrost",
		author: "Liebi Technologies <bifrost@liebi.com>",
		description: "Bifrost Parachain Node",
		support_url: "https://github.com/bifrost-codes/bifrost/issues/new",
	};

	if let Err(e) = cli::run(::std::env::args(), cli::Exit, version) {
		eprintln!("Error starting the node: {}\n\n{:?}", e, e);
		std::process::exit(1)
	}
}
