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

use wasm_builder_runner::{build_current_project_with_rustflags, WasmBuilderSource};

fn main() {
	build_current_project_with_rustflags(
		"wasm_binary.rs",
		WasmBuilderSource::Crates("1.0.8"),
		// This instructs LLD to export __heap_base as a global variable, which is used by the
		// external memory allocator.
		"-Clink-arg=--export=__heap_base",
	);
}
