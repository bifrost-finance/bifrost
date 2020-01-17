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

use futures::channel::oneshot;
use futures::{future, FutureExt};
use sc_cli::VersionInfo;

use std::cell::RefCell;

// handles ctrl-c
struct Exit;
impl sc_cli::IntoExit for Exit {
	type Exit = future::Map<oneshot::Receiver<()>, fn(Result<(), oneshot::Canceled>) -> ()>;
	fn into_exit(self) -> Self::Exit {
		// can't use signal directly here because CtrlC takes only `Fn`.
		let (exit_send, exit) = oneshot::channel();

		let exit_send_cell = RefCell::new(Some(exit_send));
		ctrlc::set_handler(move || {
			if let Some(exit_send) = exit_send_cell.try_borrow_mut().expect("signal handler not reentrant; qed").take() {
				exit_send.send(()).expect("Error sending exit notification");
			}
		}).expect("Error setting Ctrl-C handler");

		exit.map(|_| ())
	}
}

fn main() -> Result<(), sc_cli::error::Error> {
	let version = VersionInfo {
		name: "Liebi Bifrost",
		commit: env!("VERGEN_SHA_SHORT"),
		version: env!("CARGO_PKG_VERSION"),
		executable_name: "bifrost",
		author: "Liebi Technologies <bifrost@liebi.com>",
		description: "Bifrost Parachain Node",
		support_url: "https://github.com/bifrost-codes/bifrost/issues/new",
	};

	node_cli::run(std::env::args(), Exit, version)
}
