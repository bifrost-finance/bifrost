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

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{benchmarks, vec, whitelisted_caller};
use frame_support::{assert_ok, sp_runtime::traits::UniqueSaturatedFrom};
use frame_system::{Pallet as System, RawOrigin};
use node_primitives::{CurrencyId, TokenSymbol};

use crate::{Pallet as SystemStaking, *};

benchmarks! {
  on_initialize {}:{SystemStaking::<T>::on_initialize(T::BlockNumber::from(10u32));}

  token_config {
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	}: _(RawOrigin::Root,
  KSM,
  Some(1),
  Some(Permill::from_percent(80)),
  Some(false),
  Some(100),
  None,
	)

  refresh_token_info {
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	}: _(RawOrigin::Root,
  KSM,
	)

  payout {
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	}: _(RawOrigin::Root,
  KSM,
	)

  on_redeem_success {
    const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
    const ALICE: AccountId = AccountId32::new([0u8; 32]);
    let amount = 100;
  }:{SystemStaking::<T>::on_redeem_success(
    KSM,
    ALICE,
    amount,
  );}
}