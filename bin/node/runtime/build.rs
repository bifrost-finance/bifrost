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

use wasm_builder_runner::WasmBuilder;

fn main() {
	WasmBuilder::new()
		.with_current_project()
		.with_wasm_builder_from_git(
			"https://github.com/paritytech/substrate.git",
			"68e51d6e24862d499b5f042321cc87b172579e74"
		)
		.export_heap_base()
		.import_memory()
		.build()
}
