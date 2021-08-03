// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

#![cfg(test)]
use core::convert::TryFrom;

use super::*;

#[test]
fn currency_id_from_string_should_work() {
	let currency_id = CurrencyId::try_from("DOT".as_bytes().to_vec());
	assert!(currency_id.is_ok());
	assert_eq!(currency_id.unwrap(), CurrencyId::Token(TokenSymbol::DOT));
}

#[test]
fn currency_id_to_u64_should_work() {
	let e00 = CurrencyId::Native(TokenSymbol::ASG);
	let e01 = CurrencyId::Native(TokenSymbol::BNC);
	let e02 = CurrencyId::Native(TokenSymbol::AUSD);
	let e03 = CurrencyId::Native(TokenSymbol::DOT);
	let e04 = CurrencyId::Native(TokenSymbol::KSM);
	let e05 = CurrencyId::Native(TokenSymbol::ETH);

	assert_eq!(0x0000_0000_0000_0000, e00.currency_id());
	assert_eq!(0x0001_0000_0000_0000, e01.currency_id());
	assert_eq!(0x0002_0000_0000_0000, e02.currency_id());
	assert_eq!(0x0003_0000_0000_0000, e03.currency_id());
	assert_eq!(0x0004_0000_0000_0000, e04.currency_id());
	assert_eq!(0x0005_0000_0000_0000, e05.currency_id());

	let e10 = CurrencyId::VToken(TokenSymbol::ASG);
	let e11 = CurrencyId::VToken(TokenSymbol::BNC);
	let e12 = CurrencyId::VToken(TokenSymbol::AUSD);
	let e13 = CurrencyId::VToken(TokenSymbol::DOT);
	let e14 = CurrencyId::VToken(TokenSymbol::KSM);
	let e15 = CurrencyId::VToken(TokenSymbol::ETH);

	assert_eq!(0x0100_0000_0000_0000, e10.currency_id());
	assert_eq!(0x0101_0000_0000_0000, e11.currency_id());
	assert_eq!(0x0102_0000_0000_0000, e12.currency_id());
	assert_eq!(0x0103_0000_0000_0000, e13.currency_id());
	assert_eq!(0x0104_0000_0000_0000, e14.currency_id());
	assert_eq!(0x0105_0000_0000_0000, e15.currency_id());

	let e20 = CurrencyId::Token(TokenSymbol::ASG);
	let e21 = CurrencyId::Token(TokenSymbol::BNC);
	let e22 = CurrencyId::Token(TokenSymbol::AUSD);
	let e23 = CurrencyId::Token(TokenSymbol::DOT);
	let e24 = CurrencyId::Token(TokenSymbol::KSM);
	let e25 = CurrencyId::Token(TokenSymbol::ETH);

	assert_eq!(0x0200_0000_0000_0000, e20.currency_id());
	assert_eq!(0x0201_0000_0000_0000, e21.currency_id());
	assert_eq!(0x0202_0000_0000_0000, e22.currency_id());
	assert_eq!(0x0203_0000_0000_0000, e23.currency_id());
	assert_eq!(0x0204_0000_0000_0000, e24.currency_id());
	assert_eq!(0x0205_0000_0000_0000, e25.currency_id());

	let e30 = CurrencyId::Stable(TokenSymbol::ASG);
	let e31 = CurrencyId::Stable(TokenSymbol::BNC);
	let e32 = CurrencyId::Stable(TokenSymbol::AUSD);
	let e33 = CurrencyId::Stable(TokenSymbol::DOT);
	let e34 = CurrencyId::Stable(TokenSymbol::KSM);
	let e35 = CurrencyId::Stable(TokenSymbol::ETH);

	assert_eq!(0x0300_0000_0000_0000, e30.currency_id());
	assert_eq!(0x0301_0000_0000_0000, e31.currency_id());
	assert_eq!(0x0302_0000_0000_0000, e32.currency_id());
	assert_eq!(0x0303_0000_0000_0000, e33.currency_id());
	assert_eq!(0x0304_0000_0000_0000, e34.currency_id());
	assert_eq!(0x0305_0000_0000_0000, e35.currency_id());

	let e40 = CurrencyId::VSToken(TokenSymbol::ASG);
	let e41 = CurrencyId::VSToken(TokenSymbol::BNC);
	let e42 = CurrencyId::VSToken(TokenSymbol::AUSD);
	let e43 = CurrencyId::VSToken(TokenSymbol::DOT);
	let e44 = CurrencyId::VSToken(TokenSymbol::KSM);
	let e45 = CurrencyId::VSToken(TokenSymbol::ETH);

	assert_eq!(0x0400_0000_0000_0000, e40.currency_id());
	assert_eq!(0x0401_0000_0000_0000, e41.currency_id());
	assert_eq!(0x0402_0000_0000_0000, e42.currency_id());
	assert_eq!(0x0403_0000_0000_0000, e43.currency_id());
	assert_eq!(0x0404_0000_0000_0000, e44.currency_id());
	assert_eq!(0x0405_0000_0000_0000, e45.currency_id());

	let e50 = CurrencyId::VSBond(TokenSymbol::ASG, 0x07d0, 0x0000, 0x000f);
	let e51 = CurrencyId::VSBond(TokenSymbol::BNC, 0x07d1, 0x000f, 0x001f);
	let e52 = CurrencyId::VSBond(TokenSymbol::AUSD, 0x07d2, 0x001f, 0x002f);
	let e53 = CurrencyId::VSBond(TokenSymbol::DOT, 0x07d3, 0x002f, 0x003f);
	let e54 = CurrencyId::VSBond(TokenSymbol::KSM, 0x07d4, 0x003f, 0x004f);
	let e55 = CurrencyId::VSBond(TokenSymbol::ETH, 0x07d5, 0x004f, 0x005f);

	assert_eq!(0x0500_07d0_0000_000f, e50.currency_id());
	assert_eq!(0x0501_07d1_000f_001f, e51.currency_id());
	assert_eq!(0x0502_07d2_001f_002f, e52.currency_id());
	assert_eq!(0x0503_07d3_002f_003f, e53.currency_id());
	assert_eq!(0x0504_07d4_003f_004f, e54.currency_id());
	assert_eq!(0x0505_07d5_004f_005f, e55.currency_id());
}

#[test]
fn u64_to_currency_id_should_work() {
	let e00 = CurrencyId::Native(TokenSymbol::ASG);
	let e01 = CurrencyId::Native(TokenSymbol::BNC);
	let e02 = CurrencyId::Native(TokenSymbol::AUSD);
	let e03 = CurrencyId::Native(TokenSymbol::DOT);
	let e04 = CurrencyId::Native(TokenSymbol::KSM);
	let e05 = CurrencyId::Native(TokenSymbol::ETH);

	assert_eq!(e00, CurrencyId::try_from(0x0000_0000_0000_0000).unwrap());
	assert_eq!(e01, CurrencyId::try_from(0x0001_0000_0000_0000).unwrap());
	assert_eq!(e02, CurrencyId::try_from(0x0002_0000_0000_0000).unwrap());
	assert_eq!(e03, CurrencyId::try_from(0x0003_0000_0000_0000).unwrap());
	assert_eq!(e04, CurrencyId::try_from(0x0004_0000_0000_0000).unwrap());
	assert_eq!(e05, CurrencyId::try_from(0x0005_0000_0000_0000).unwrap());

	let e10 = CurrencyId::VToken(TokenSymbol::ASG);
	let e11 = CurrencyId::VToken(TokenSymbol::BNC);
	let e12 = CurrencyId::VToken(TokenSymbol::AUSD);
	let e13 = CurrencyId::VToken(TokenSymbol::DOT);
	let e14 = CurrencyId::VToken(TokenSymbol::KSM);
	let e15 = CurrencyId::VToken(TokenSymbol::ETH);

	assert_eq!(e10, CurrencyId::try_from(0x0100_0000_0000_0000).unwrap());
	assert_eq!(e11, CurrencyId::try_from(0x0101_0000_0000_0000).unwrap());
	assert_eq!(e12, CurrencyId::try_from(0x0102_0000_0000_0000).unwrap());
	assert_eq!(e13, CurrencyId::try_from(0x0103_0000_0000_0000).unwrap());
	assert_eq!(e14, CurrencyId::try_from(0x0104_0000_0000_0000).unwrap());
	assert_eq!(e15, CurrencyId::try_from(0x0105_0000_0000_0000).unwrap());

	let e20 = CurrencyId::Token(TokenSymbol::ASG);
	let e21 = CurrencyId::Token(TokenSymbol::BNC);
	let e22 = CurrencyId::Token(TokenSymbol::AUSD);
	let e23 = CurrencyId::Token(TokenSymbol::DOT);
	let e24 = CurrencyId::Token(TokenSymbol::KSM);
	let e25 = CurrencyId::Token(TokenSymbol::ETH);

	assert_eq!(e20, CurrencyId::try_from(0x0200_0000_0000_0000).unwrap());
	assert_eq!(e21, CurrencyId::try_from(0x0201_0000_0000_0000).unwrap());
	assert_eq!(e22, CurrencyId::try_from(0x0202_0000_0000_0000).unwrap());
	assert_eq!(e23, CurrencyId::try_from(0x0203_0000_0000_0000).unwrap());
	assert_eq!(e24, CurrencyId::try_from(0x0204_0000_0000_0000).unwrap());
	assert_eq!(e25, CurrencyId::try_from(0x0205_0000_0000_0000).unwrap());

	let e30 = CurrencyId::Stable(TokenSymbol::ASG);
	let e31 = CurrencyId::Stable(TokenSymbol::BNC);
	let e32 = CurrencyId::Stable(TokenSymbol::AUSD);
	let e33 = CurrencyId::Stable(TokenSymbol::DOT);
	let e34 = CurrencyId::Stable(TokenSymbol::KSM);
	let e35 = CurrencyId::Stable(TokenSymbol::ETH);

	assert_eq!(e30, CurrencyId::try_from(0x0300_0000_0000_0000).unwrap());
	assert_eq!(e31, CurrencyId::try_from(0x0301_0000_0000_0000).unwrap());
	assert_eq!(e32, CurrencyId::try_from(0x0302_0000_0000_0000).unwrap());
	assert_eq!(e33, CurrencyId::try_from(0x0303_0000_0000_0000).unwrap());
	assert_eq!(e34, CurrencyId::try_from(0x0304_0000_0000_0000).unwrap());
	assert_eq!(e35, CurrencyId::try_from(0x0305_0000_0000_0000).unwrap());

	let e40 = CurrencyId::VSToken(TokenSymbol::ASG);
	let e41 = CurrencyId::VSToken(TokenSymbol::BNC);
	let e42 = CurrencyId::VSToken(TokenSymbol::AUSD);
	let e43 = CurrencyId::VSToken(TokenSymbol::DOT);
	let e44 = CurrencyId::VSToken(TokenSymbol::KSM);
	let e45 = CurrencyId::VSToken(TokenSymbol::ETH);

	assert_eq!(e40, CurrencyId::try_from(0x0400_0000_0000_0000).unwrap());
	assert_eq!(e41, CurrencyId::try_from(0x0401_0000_0000_0000).unwrap());
	assert_eq!(e42, CurrencyId::try_from(0x0402_0000_0000_0000).unwrap());
	assert_eq!(e43, CurrencyId::try_from(0x0403_0000_0000_0000).unwrap());
	assert_eq!(e44, CurrencyId::try_from(0x0404_0000_0000_0000).unwrap());
	assert_eq!(e45, CurrencyId::try_from(0x0405_0000_0000_0000).unwrap());

	let e50 = CurrencyId::VSBond(TokenSymbol::ASG, 0x07d0, 0x0000, 0x000f);
	let e51 = CurrencyId::VSBond(TokenSymbol::BNC, 0x07d1, 0x000f, 0x001f);
	let e52 = CurrencyId::VSBond(TokenSymbol::AUSD, 0x07d2, 0x001f, 0x002f);
	let e53 = CurrencyId::VSBond(TokenSymbol::DOT, 0x07d3, 0x002f, 0x003f);
	let e54 = CurrencyId::VSBond(TokenSymbol::KSM, 0x07d4, 0x003f, 0x004f);
	let e55 = CurrencyId::VSBond(TokenSymbol::ETH, 0x07d5, 0x004f, 0x005f);

	assert_eq!(e50, CurrencyId::try_from(0x0500_07d0_0000_000f).unwrap());
	assert_eq!(e51, CurrencyId::try_from(0x0501_07d1_000f_001f).unwrap());
	assert_eq!(e52, CurrencyId::try_from(0x0502_07d2_001f_002f).unwrap());
	assert_eq!(e53, CurrencyId::try_from(0x0503_07d3_002f_003f).unwrap());
	assert_eq!(e54, CurrencyId::try_from(0x0504_07d4_003f_004f).unwrap());
	assert_eq!(e55, CurrencyId::try_from(0x0505_07d5_004f_005f).unwrap());
}
