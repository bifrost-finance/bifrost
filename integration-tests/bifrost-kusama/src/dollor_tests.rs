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

use bifrost_kusama_runtime::Runtime;
use bifrost_primitives::{CurrencyId, TokenSymbol::*};
use bifrost_runtime_common::{cent, dollar, micro, microcent, milli, millicent};
use integration_tests_common::BifrostKusama;
use xcm_emulator::TestExt;

const DECIMAL_18: u128 = 1_000_000_000_000_000_000;
const DECIMAL_12: u128 = 1_000_000_000_000;
const DOT_DECIMALS: u128 = 10_000_000_000;

#[test]
fn dollar_should_work() {
	BifrostKusama::execute_with(|| {
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(ASG)), DECIMAL_12);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(BNC)), DECIMAL_12);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(KUSD)), DECIMAL_12);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(DOT)), DOT_DECIMALS);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(KSM)), DECIMAL_12);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(ETH)), DECIMAL_18);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(KAR)), DECIMAL_12);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(ZLK)), DECIMAL_18);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(PHA)), DECIMAL_12);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(RMRK)), DOT_DECIMALS);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(MOVR)), DECIMAL_18);
	});
}

#[test]
fn milli_should_work() {
	BifrostKusama::execute_with(|| {
		assert_eq!(milli::<Runtime>(CurrencyId::Token(ASG)), DECIMAL_12 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(BNC)), DECIMAL_12 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(KUSD)), DECIMAL_12 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(DOT)), DOT_DECIMALS / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(KSM)), DECIMAL_12 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(ETH)), DECIMAL_18 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(KAR)), DECIMAL_12 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(ZLK)), DECIMAL_18 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(PHA)), DECIMAL_12 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(RMRK)), DOT_DECIMALS / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(MOVR)), DECIMAL_18 / 1000);
	});
}

#[test]
fn micro_should_work() {
	BifrostKusama::execute_with(|| {
		assert_eq!(micro::<Runtime>(CurrencyId::Token(ASG)), DECIMAL_12 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(BNC)), DECIMAL_12 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(KUSD)), DECIMAL_12 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(DOT)), DOT_DECIMALS / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(KSM)), DECIMAL_12 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(ETH)), DECIMAL_18 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(KAR)), DECIMAL_12 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(ZLK)), DECIMAL_18 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(PHA)), DECIMAL_12 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(RMRK)), DOT_DECIMALS / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(MOVR)), DECIMAL_18 / 1_000_000);
	});
}

#[test]
fn cent_should_work() {
	BifrostKusama::execute_with(|| {
		assert_eq!(cent::<Runtime>(CurrencyId::Token(ASG)), DECIMAL_12 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(BNC)), DECIMAL_12 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(KUSD)), DECIMAL_12 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(DOT)), DOT_DECIMALS / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(KSM)), DECIMAL_12 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(ETH)), DECIMAL_18 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(KAR)), DECIMAL_12 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(ZLK)), DECIMAL_18 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(PHA)), DECIMAL_12 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(RMRK)), DOT_DECIMALS / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(MOVR)), DECIMAL_18 / 100);
	});
}

#[test]
fn millicent_should_work() {
	BifrostKusama::execute_with(|| {
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(ASG)), DECIMAL_12 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(BNC)), DECIMAL_12 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(KUSD)), DECIMAL_12 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(DOT)), DOT_DECIMALS / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(KSM)), DECIMAL_12 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(ETH)), DECIMAL_18 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(KAR)), DECIMAL_12 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(ZLK)), DECIMAL_18 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(PHA)), DECIMAL_12 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(RMRK)), DOT_DECIMALS / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(MOVR)), DECIMAL_18 / 100_000);
	});
}

#[test]
fn microcent_should_work() {
	BifrostKusama::execute_with(|| {
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(ASG)), DECIMAL_12 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(BNC)), DECIMAL_12 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(KUSD)), DECIMAL_12 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(DOT)), DOT_DECIMALS / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(KSM)), DECIMAL_12 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(ETH)), DECIMAL_18 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(KAR)), DECIMAL_12 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(ZLK)), DECIMAL_18 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(PHA)), DECIMAL_12 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(RMRK)), DOT_DECIMALS / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(MOVR)), DECIMAL_18 / 100_000_000);
	});
}
