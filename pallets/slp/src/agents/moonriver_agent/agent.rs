// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
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

use core::marker::PhantomData;

use codec::Encode;
use cumulus_primitives_core::relay_chain::HashT;
pub use cumulus_primitives_core::ParaId;
use frame_support::{ensure, traits::Get, weights::Weight};
use frame_system::pallet_prelude::BlockNumberFor;
use node_primitives::{CurrencyId, TokenSymbol, VtokenMintingOperator};
use orml_traits::MultiCurrency;
use sp_core::U256;
use sp_runtime::{
	traits::{
		CheckedAdd, CheckedSub, Convert, Saturating, StaticLookup, UniqueSaturatedFrom,
		UniqueSaturatedInto, Zero,
	},
	DispatchResult,
};
use sp_std::prelude::*;
use types::{
	MoonriverBalancesCall, MoonriverCall, MoonriverCurrencyId, MoonriverParachainStakingCall,
	MoonriverUtilityCall, MoonriverXtokensCall,
};
use xcm::{
	latest::prelude::*,
	opaque::latest::{
		Junction::{AccountId32, Parachain},
		Junctions::X1,
		MultiLocation,
	},
	VersionedMultiAssets, VersionedMultiLocation,
};

use crate::{
	agents::SystemCall,
	pallet::{Error, Event},
	primitives::{
		Ledger, SubstrateLedger, SubstrateLedgerUpdateEntry,
		SubstrateValidatorsByDelegatorUpdateEntry, UnlockChunk, ValidatorsByDelegatorUpdateEntry,
		XcmOperation, KSM,
	},
	traits::{QueryResponseManager, StakingAgent, XcmBuilder},
	AccountIdOf, BalanceOf, Config, CurrencyDelays, DelegatorLedgerXcmUpdateQueue,
	DelegatorLedgers, DelegatorNextIndex, DelegatorsIndex2Multilocation,
	DelegatorsMultilocation2Index, Hash, LedgerUpdateEntry, MinimumsAndMaximums, Pallet, QueryId,
	TimeUnit, Validators, ValidatorsByDelegator, ValidatorsByDelegatorXcmUpdateQueue,
	XcmDestWeightAndFee, TIMEOUT_BLOCKS,
};

/// StakingAgent implementation for Moonriver
pub struct MoonriverAgent<T>(PhantomData<T>);

impl<T> MoonriverAgent<T> {
	pub fn new() -> Self {
		MoonriverAgent(PhantomData::<T>)
	}
}

impl<T: Config>
	StakingAgent<
		MultiLocation,
		MultiLocation,
		BalanceOf<T>,
		TimeUnit,
		AccountIdOf<T>,
		MultiLocation,
		QueryId,
		LedgerUpdateEntry<BalanceOf<T>, MultiLocation>,
		ValidatorsByDelegatorUpdateEntry<MultiLocation, MultiLocation, Hash<T>>,
		Error<T>,
	> for KusamaAgent<T>
{
}
