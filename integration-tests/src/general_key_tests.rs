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

#![allow(unused)]

use crate::integration_tests::*;
use xcm::latest::prelude::*;

/// Currency ID, it might be extended with more variants in the future.
const TEST_33: [u8; 33] = [1u8; 33];

#[test]
fn general_key_works() {

	GeneralKey(CurrencyId::ForeignAsset(3).encode().try_into().unwrap());
	GeneralKey(CurrencyId::ForeignAsset(3).encode().try_into().unwrap());
	GeneralKey(CurrencyId::ForeignAsset(3).encode().try_into().unwrap());
	GeneralKey(CurrencyId::ForeignAsset(3).encode().try_into().unwrap());
	GeneralKey(CurrencyId::ForeignAsset(3).encode().try_into().unwrap());
	GeneralKey(CurrencyId::ForeignAsset(3).encode().try_into().unwrap());
	GeneralKey(CurrencyId::ForeignAsset(3).encode().try_into().unwrap());
	GeneralKey(CurrencyId::ForeignAsset(3).encode().try_into().unwrap());
	GeneralKey(CurrencyId::ForeignAsset(3).encode().try_into().unwrap());
	GeneralKey(CurrencyId::ForeignAsset(3).encode().try_into().unwrap());
	GeneralKey(CurrencyId::ForeignAsset(3).encode().try_into().unwrap());

	GeneralKey(CurrencyId::LPToken(TokenSymbol::ASG,100,TokenSymbol::BNC,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::LPToken(TokenSymbol::BNC,100,TokenSymbol::ASG,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::LPToken(TokenSymbol::KUSD,100,TokenSymbol::ASG,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::LPToken(TokenSymbol::DOT,100,TokenSymbol::ASG,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::LPToken(TokenSymbol::KSM,100,TokenSymbol::ASG,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::LPToken(TokenSymbol::ETH,100,TokenSymbol::ASG,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::LPToken(TokenSymbol::KAR,100,TokenSymbol::ASG,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::LPToken(TokenSymbol::ZLK,100,TokenSymbol::ASG,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::LPToken(TokenSymbol::PHA,100,TokenSymbol::ASG,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::LPToken(TokenSymbol::RMRK,100,TokenSymbol::ASG,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::LPToken(TokenSymbol::MOVR,100,TokenSymbol::ASG,100).encode().try_into().unwrap());


	GeneralKey(CurrencyId::VSBond(TokenSymbol::ASG,2001,1,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSBond(TokenSymbol::BNC,2001,1,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSBond(TokenSymbol::KUSD,2001,1,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSBond(TokenSymbol::DOT,2001,1,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSBond(TokenSymbol::KSM,2001,1,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSBond(TokenSymbol::ETH,2001,1,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSBond(TokenSymbol::KAR,2001,1,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSBond(TokenSymbol::ZLK,2001,1,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSBond(TokenSymbol::PHA,2001,1,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSBond(TokenSymbol::RMRK,2001,1,100).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSBond(TokenSymbol::MOVR,2001,1,100).encode().try_into().unwrap());

	GeneralKey(CurrencyId::VSToken(TokenSymbol::ASG).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::BNC).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::KUSD).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::DOT).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::KSM).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::ETH).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::KAR).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::ZLK).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::PHA).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::RMRK).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::MOVR).encode().try_into().unwrap());

	GeneralKey(CurrencyId::VSToken(TokenSymbol::ASG).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::BNC).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::KUSD).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::DOT).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::KSM).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::ETH).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::KAR).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::ZLK).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::PHA).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::RMRK).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VSToken(TokenSymbol::MOVR).encode().try_into().unwrap());

	GeneralKey(CurrencyId::VToken(TokenSymbol::ASG).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VToken(TokenSymbol::BNC).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VToken(TokenSymbol::KUSD).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VToken(TokenSymbol::DOT).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VToken(TokenSymbol::KSM).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VToken(TokenSymbol::ETH).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VToken(TokenSymbol::KAR).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VToken(TokenSymbol::ZLK).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VToken(TokenSymbol::PHA).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VToken(TokenSymbol::RMRK).encode().try_into().unwrap());
	GeneralKey(CurrencyId::VToken(TokenSymbol::MOVR).encode().try_into().unwrap());

	GeneralKey(CurrencyId::Native(TokenSymbol::ASG).encode().try_into().unwrap());
	GeneralKey(CurrencyId::Native(TokenSymbol::BNC).encode().try_into().unwrap());
	GeneralKey(CurrencyId::Native(TokenSymbol::KUSD).encode().try_into().unwrap());
	GeneralKey(CurrencyId::Native(TokenSymbol::DOT).encode().try_into().unwrap());
	GeneralKey(CurrencyId::Native(TokenSymbol::KSM).encode().try_into().unwrap());
	GeneralKey(CurrencyId::Native(TokenSymbol::ETH).encode().try_into().unwrap());
	GeneralKey(CurrencyId::Native(TokenSymbol::KAR).encode().try_into().unwrap());
	GeneralKey(CurrencyId::Native(TokenSymbol::ZLK).encode().try_into().unwrap());
	GeneralKey(CurrencyId::Native(TokenSymbol::PHA).encode().try_into().unwrap());
	GeneralKey(CurrencyId::Native(TokenSymbol::RMRK).encode().try_into().unwrap());
	GeneralKey(CurrencyId::Native(TokenSymbol::MOVR).encode().try_into().unwrap());


	GeneralKey(CurrencyId::Native(TokenSymbol::MOVR).encode().try_into().unwrap());



	assert_eq!(CurrencyId::LPToken(TokenSymbol::ASG,254,TokenSymbol::BNC,100).encode().len(),5);
	assert_eq!(TEST_33.len(),33)

	// 'called `Result::unwrap()` on an `Err` value: ()'
	//GeneralKey(test_33.encode().try_into().unwrap());
}
