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

//! Bifrost Node CLI

#![warn(missing_docs)]

fn main() -> sc_cli::Result<()> {
	let version = sc_cli::VersionInfo {
		name: "Liebi Bifrost",
		commit: env!("VERGEN_SHA_SHORT"),
		version: env!("CARGO_PKG_VERSION"),
		executable_name: "bifrost",
		author: "Liebi Technologies <bifrost@liebi.com>",
		description: "Bifrost Parachain Node",
		support_url: "https://github.com/bifrost-codes/bifrost/issues/new",
		copyright_start_year: 2019,
	};

	node_cli::run(std::env::args(), version)
}
