// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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

use crate::evm::precompiles::{costs, revert, Address, Bytes, EvmResult};
use pallet_evm::{Context, Log, PrecompileHandle};
use primitive_types::{H160, H256, U256};
use smallvec::alloc;
use sp_std::{borrow::ToOwned, vec, vec::Vec};

/// Wrapper around an EVM input slice, helping to parse it.
/// Provide functions to parse common types.
#[derive(Clone, Copy, Debug)]
pub struct EvmDataReader<'a> {
	input: &'a [u8],
	cursor: usize,
}

#[derive(Clone, Debug)]
pub struct EvmDataWriter {
	pub(crate) data: Vec<u8>,
	offset_data: Vec<OffsetDatum>,
	selector: Option<u32>,
}

impl EvmDataWriter {
	/// Creates a new empty output builder (without selector).
	pub fn new() -> Self {
		Self { data: vec![], offset_data: vec![], selector: None }
	}

	/// Return the built data.
	pub fn build(mut self) -> Vec<u8> {
		Self::bake_offsets(&mut self.data, self.offset_data);

		if let Some(selector) = self.selector {
			let mut output = selector.to_be_bytes().to_vec();
			output.append(&mut self.data);
			output
		} else {
			self.data
		}
	}

	/// Add offseted data at the end of this writer's data, updating the offsets.
	fn bake_offsets(output: &mut Vec<u8>, offsets: Vec<OffsetDatum>) {
		for mut offset_datum in offsets {
			let offset_position = offset_datum.offset_position;
			let offset_position_end = offset_position + 32;

			// The offset is the distance between the start of the data and the
			// start of the pointed data (start of a struct, length of an array).
			// Offsets in inner data are relative to the start of their respective "container".
			// However in arrays the "container" is actually the item itself instead of the whole
			// array, which is corrected by `offset_shift`.
			let free_space_offset = output.len() - offset_datum.offset_shift;

			// Override dummy offset to the offset it will be in the final output.
			U256::from(free_space_offset)
				.to_big_endian(&mut output[offset_position..offset_position_end]);

			// Append this data at the end of the current output.
			output.append(&mut offset_datum.data);
		}
	}

	/// Write arbitrary bytes.
	/// Doesn't handle any alignement checks, prefer using `write` instead if possible.
	fn write_raw_bytes(mut self, value: &[u8]) -> Self {
		self.data.extend_from_slice(value);
		self
	}

	/// Write data of requested type.
	pub fn write<T: EvmData>(mut self, value: T) -> Self {
		T::write(&mut self, value);
		self
	}

	/// Writes a pointer to given data.
	/// The data will be appended when calling `build`.
	/// Initially write a dummy value as offset in this writer's data, which will be replaced by
	/// the correct offset once the pointed data is appended.
	///
	/// Takes `&mut self` since its goal is to be used inside `EvmData` impl and not in chains.
	pub fn write_pointer(&mut self, data: Vec<u8>) {
		let offset_position = self.data.len();
		H256::write(self, H256::repeat_byte(0xff));

		self.offset_data.push(OffsetDatum { offset_position, data, offset_shift: 0 });
	}
}

impl Default for EvmDataWriter {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Clone, Debug)]
struct OffsetDatum {
	// Offset location in the container data.
	offset_position: usize,
	// Data pointed by the offset that must be inserted at the end of container data.
	data: Vec<u8>,
	// Inside of arrays, the offset is not from the start of array data (length), but from the
	// start of the item. This shift allow to correct this.
	offset_shift: usize,
}

/// Data that can be converted from and to EVM data types.
pub trait EvmData: Sized {
	fn read(reader: &mut EvmDataReader) -> EvmResult<Self>;
	fn write(writer: &mut EvmDataWriter, value: Self);
	fn has_static_size() -> bool;
}

impl EvmData for U256 {
	fn read(reader: &mut EvmDataReader) -> EvmResult<Self> {
		let range = reader.move_cursor(32)?;

		let data = reader
			.input
			.get(range)
			.ok_or_else(|| revert("tried to parse U256 out of bounds"))?;

		Ok(U256::from_big_endian(data))
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		let mut buffer = [0u8; 32];
		value.to_big_endian(&mut buffer);
		writer.data.extend_from_slice(&buffer);
	}

	fn has_static_size() -> bool {
		true
	}
}

impl EvmData for H256 {
	fn read(reader: &mut EvmDataReader) -> EvmResult<Self> {
		let range = reader.move_cursor(32)?;

		let data = reader
			.input
			.get(range)
			.ok_or_else(|| revert("tried to parse H256 out of bounds"))?;

		Ok(H256::from_slice(data))
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		writer.data.extend_from_slice(value.as_bytes());
	}

	fn has_static_size() -> bool {
		true
	}
}

impl EvmData for Bytes {
	fn read(reader: &mut EvmDataReader) -> EvmResult<Self> {
		let mut inner_reader = reader.read_pointer()?;

		// Read bytes/string size.
		let array_size: usize = inner_reader
			.read::<U256>()
			.map_err(|_| revert("tried to parse bytes/string length out of bounds"))?
			.try_into()
			.map_err(|_| revert("bytes/string length is too large"))?;

		// Get valid range over the bytes data.
		let range = inner_reader.move_cursor(array_size)?;

		let data = inner_reader
			.input
			.get(range)
			.ok_or_else(|| revert("tried to parse bytes/string out of bounds"))?;

		let bytes = Self(data.to_owned());

		Ok(bytes)
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		let length = value.0.len();

		// Pad the data.
		// Leave it as is if a multiple of 32, otherwise pad to next
		// multiple or 32.
		let chunks = length / 32;
		let padded_size = match length % 32 {
			0 => chunks * 32,
			_ => (chunks + 1) * 32,
		};

		let mut value = value.0.to_vec();
		value.resize(padded_size, 0);

		writer.write_pointer(
			EvmDataWriter::new().write(U256::from(length)).write_raw_bytes(&value).build(),
		);
	}

	fn has_static_size() -> bool {
		false
	}
}

impl EvmData for Address {
	fn read(reader: &mut EvmDataReader) -> EvmResult<Self> {
		let range = reader.move_cursor(32)?;

		let data = reader
			.input
			.get(range)
			.ok_or_else(|| revert("tried to parse H160 out of bounds"))?;

		Ok(H160::from_slice(&data[12..32]).into())
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		H256::write(writer, value.0.into());
	}

	fn has_static_size() -> bool {
		true
	}
}

macro_rules! impl_evmdata_for_uints {
	($($uint:ty, )*) => {
		$(
			impl EvmData for $uint {
				fn read(reader: &mut EvmDataReader) -> EvmResult<Self> {
					let value256: U256 = reader.read()?;

					value256
						.try_into()
						.map_err(|_| revert(alloc::format!(
							"value too big for type",
						)))
				}

				fn write(writer: &mut EvmDataWriter, value: Self) {
					U256::write(writer, value.into());
				}

				fn has_static_size() -> bool {
					true
				}
			}
		)*
	};
}

impl_evmdata_for_uints!(u8, u16, u32, u64, u128,);

impl EvmData for bool {
	fn read(reader: &mut EvmDataReader) -> EvmResult<Self> {
		let h256 = H256::read(reader).map_err(|_| revert("tried to parse bool out of bounds"))?;

		Ok(!h256.is_zero())
	}

	fn write(writer: &mut EvmDataWriter, value: Self) {
		let mut buffer = [0u8; 32];
		if value {
			buffer[31] = 1;
		}

		writer.data.extend_from_slice(&buffer);
	}

	fn has_static_size() -> bool {
		true
	}
}

impl<'a> EvmDataReader<'a> {
	/// Create a new input parser.
	pub fn new(input: &'a [u8]) -> Self {
		Self { input, cursor: 0 }
	}

	/// Create a new input parser from a selector-initial input.
	pub fn read_selector<T>(input: &'a [u8]) -> EvmResult<T>
	where
		T: num_enum::TryFromPrimitive<Primitive = u32>,
	{
		if input.len() < 4 {
			return Err(revert("tried to parse selector out of bounds"));
		}

		let mut buffer = [0u8; 4];
		buffer.copy_from_slice(&input[0..4]);
		let selector = T::try_from_primitive(u32::from_be_bytes(buffer)).map_err(|_| {
			log::trace!(
				target: "precompile-utils",
				"Failed to match function selector"
				//TODO: add type in log
			);
			revert("unknown selector")
		})?;

		Ok(selector)
	}

	/// Create a new input parser from a selector-initial input.
	pub fn new_skip_selector(input: &'a [u8]) -> EvmResult<Self> {
		if input.len() < 4 {
			return Err(revert("input is too short"));
		}

		Ok(Self::new(&input[4..]))
	}

	/// Check the input has at least the correct amount of arguments before the end (32 bytes
	/// values).
	pub fn expect_arguments(&self, args: usize) -> EvmResult {
		if self.input.len() >= self.cursor + args * 32 {
			Ok(())
		} else {
			Err(revert("input doesn't match expected length"))
		}
	}

	/// Read data from the input.
	pub fn read<T: EvmData>(&mut self) -> EvmResult<T> {
		T::read(self)
	}

	/// Reads a pointer, returning a reader targetting the pointed location.
	pub fn read_pointer(&mut self) -> EvmResult<Self> {
		let offset: usize = self
			.read::<U256>()
			.map_err(|_| revert("tried to parse array offset out of bounds"))?
			.try_into()
			.map_err(|_| revert("array offset is too large"))?;

		if offset >= self.input.len() {
			return Err(revert("pointer points out of bounds"));
		}

		Ok(Self { input: &self.input[offset..], cursor: 0 })
	}

	/// Move the reading cursor with provided length, and return a range from the previous cursor
	/// location to the new one.
	/// Checks cursor overflows.
	fn move_cursor(&mut self, len: usize) -> EvmResult<sp_std::ops::Range<usize>> {
		let start = self.cursor;
		let end = self
			.cursor
			.checked_add(len)
			.ok_or_else(|| revert("data reading cursor overflow"))?;

		self.cursor = end;

		Ok(start..end)
	}
}

/// Represents modifiers a Solidity function can be annotated with.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum FunctionModifier {
	/// Function that doesn't modify the state.
	View,
	/// Function that modifies the state but refuse receiving funds.
	/// Correspond to a Solidity function with no modifiers.
	NonPayable,
	/// Function that modifies the state and accept funds.
	Payable,
}

pub trait PrecompileHandleExt: PrecompileHandle {
	/// Record cost of a log manually.
	/// This can be useful to record log costs early when their content have static size.
	fn record_log_costs_manual(&mut self, topics: usize, data_len: usize) -> EvmResult;

	/// Record cost of logs.
	fn record_log_costs(&mut self, logs: &[&Log]) -> EvmResult;

	/// Check that a function call is compatible with the context it is
	/// called into.
	fn check_function_modifier(&self, modifier: FunctionModifier) -> EvmResult;

	/// Read the selector from the input data.
	fn read_selector<T>(&self) -> EvmResult<T>
	where
		T: num_enum::TryFromPrimitive<Primitive = u32>;

	/// Returns a reader of the input, skipping the selector.
	fn read_input(&self) -> EvmResult<EvmDataReader>;
}

impl<T: PrecompileHandle> PrecompileHandleExt for T {
	/// Record cost of a log manualy.
	/// This can be useful to record log costs early when their content have static size.
	fn record_log_costs_manual(&mut self, topics: usize, data_len: usize) -> EvmResult {
		self.record_cost(costs::log_costs(topics, data_len)?)?;

		Ok(())
	}

	/// Record cost of logs.
	fn record_log_costs(&mut self, logs: &[&Log]) -> EvmResult {
		for log in logs {
			self.record_log_costs_manual(log.topics.len(), log.data.len())?;
		}

		Ok(())
	}

	/// Check that a function call is compatible with the context it is
	/// called into.
	fn check_function_modifier(&self, modifier: FunctionModifier) -> EvmResult {
		check_function_modifier(self.context(), self.is_static(), modifier)
	}

	/// Read the selector from the input data.
	fn read_selector<S>(&self) -> EvmResult<S>
	where
		S: num_enum::TryFromPrimitive<Primitive = u32>,
	{
		let input = self.input();
		EvmDataReader::read_selector(input)
	}

	/// Returns a reader of the input, skipping the selector.
	fn read_input(&self) -> EvmResult<EvmDataReader> {
		EvmDataReader::new_skip_selector(self.input())
	}
}

/// Check that a function call is compatible with the context it is
/// called into.
pub fn check_function_modifier(
	context: &Context,
	is_static: bool,
	modifier: FunctionModifier,
) -> EvmResult {
	if is_static && modifier != FunctionModifier::View {
		return Err(revert("can't call non-static function in static context"));
	}

	if modifier != FunctionModifier::Payable && context.apparent_value > U256::zero() {
		return Err(revert("function is not payable"));
	}

	Ok(())
}
