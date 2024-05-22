//                    :                     $$\   $$\                 $$\
// $$$$$$$\  $$\   $$\                  !YJJ^                   $$ |  $$ |                $$ |
// $$  __$$\ $$ |  $$ |                7B5. ~B5^                 $$ |  $$ |$$\   $$\  $$$$$$$ |
// $$$$$$\  $$$$$$\  $$ |  $$ |\$$\ $$  |             .?B@G    ~@@P~               $$$$$$$$ |$$ |
// $$ |$$  __$$ |$$  __$$\ \____$$\ $$ |  $$ | \$$$$  /           :?#@@@Y    .&@@@P!.            $$
// __$$ |$$ |  $$ |$$ /  $$ |$$ |  \__|$$$$$$$ |$$ |  $$ | $$  $$<         ^?J^7P&@@!  .5@@#Y~!J!.
// $$ |  $$ |$$ |  $$ |$$ |  $$ |$$ |     $$  __$$ |$$ |  $$ |$$  /\$$\       ^JJ!.   :!J5^ ?5?^
// ^?Y7.        $$ |  $$ |\$$$$$$$ |\$$$$$$$ |$$ |     \$$$$$$$ |$$$$$$$  |$$ /  $$ |     ~PP: 7#B5!
// .         :?P#G: 7G?.      \__|  \__| \____$$ | \_______|\__|      \_______|\_______/ \__|  \__|
//  .!P@G    7@@@#Y^    .!P@@@#.   ~@&J:              $$\   $$ |
//  !&@@J    :&@@@@P.   !&@@@@5     #@@P.             \$$$$$$  |
//   :J##:   Y@@&P!      :JB@@&~   ?@G!                \______/
//     .?P!.?GY7:   .. .    ^?PP^:JP~
//       .7Y7.  .!YGP^ ?BP?^   ^JJ^         This file is part of https://github.com/galacticcouncil/HydraDX-node
//         .!Y7Y#@@#:   ?@@@G?JJ^           Built with <3 for decentralisation.
//            !G@@@Y    .&@@&J:
//              ^5@#.   7@#?.               Copyright (C) 2021-2023  Intergalactic, Limited (GIB).
//                :5P^.?G7.                 SPDX-License-Identifier: Apache-2.0
//                  :?Y!                    Licensed under the Apache License, Version 2.0 (the
// "License");                                          you may not use this file except in
// compliance with the License.                                          http://www.apache.org/licenses/LICENSE-2.0

//! Utils related to Substrate features:
//! - Substrate call dispatch.
//! - Substrate DB read and write costs

use crate::evm::precompiles::{revert, EvmResult};
use core::marker::PhantomData;
use frame_support::{
	dispatch::{GetDispatchInfo, PostDispatchInfo},
	sp_runtime::traits::Dispatchable,
	traits::Get,
	weights::Weight,
};
use pallet_evm::{ExitError, GasWeightMapping, PrecompileFailure, PrecompileHandle};
use smallvec::alloc;

/// Helper functions requiring a Substrate runtime.
/// This runtime must of course implement `pallet_evm::Config`.
#[derive(Clone, Copy, Debug)]
pub struct RuntimeHelper<Runtime>(PhantomData<Runtime>);

impl<Runtime> RuntimeHelper<Runtime>
where
	Runtime: pallet_evm::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
{
	/// Try to dispatch a Substrate call.
	/// Return an error if there are not enough gas, or if the call fails.
	/// If successful returns the used gas using the Runtime GasWeightMapping.
	pub fn try_dispatch<RuntimeCall>(
		handle: &mut impl PrecompileHandle,
		origin: <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin,
		call: RuntimeCall,
	) -> EvmResult<()>
	where
		Runtime::RuntimeCall: From<RuntimeCall>,
	{
		let call = Runtime::RuntimeCall::from(call);
		let dispatch_info = call.get_dispatch_info();

		// Make sure there is enough gas.
		let remaining_gas = handle.remaining_gas();
		let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
		if required_gas > remaining_gas {
			return Err(PrecompileFailure::Error { exit_status: ExitError::OutOfGas });
		}

		// Dispatch call.
		// It may be possible to not record gas cost if the call returns Pays::No.
		// However while Substrate handle checking weight while not making the sender pay for it,
		// the EVM doesn't. It seems this safer to always record the costs to avoid unmetered
		// computations.
		let used_weight = call
			.dispatch(origin)
			.map_err(|e| revert(alloc::format!("Dispatched call failed with error: {:?}", e)))?
			.actual_weight;

		let used_gas =
			Runtime::GasWeightMapping::weight_to_gas(used_weight.unwrap_or(dispatch_info.weight));

		handle.record_cost(used_gas)?;

		Ok(())
	}
}

impl<Runtime> RuntimeHelper<Runtime>
where
	Runtime: pallet_evm::Config + frame_system::Config,
{
	/// Cost of a Substrate DB write in gas.
	pub fn db_write_gas_cost() -> u64 {
		<Runtime as pallet_evm::Config>::GasWeightMapping::weight_to_gas(Weight::from_parts(
			<Runtime as frame_system::Config>::DbWeight::get().write,
			0,
		))
	}

	/// Cost of a Substrate DB read in gas.
	pub fn db_read_gas_cost() -> u64 {
		<Runtime as pallet_evm::Config>::GasWeightMapping::weight_to_gas(Weight::from_parts(
			<Runtime as frame_system::Config>::DbWeight::get().read,
			0,
		))
	}
}
