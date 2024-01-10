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
fn u64_to_currency_id_should_work() {
	let e00 = CurrencyId::Native(TokenSymbol::ASG);
	let e01 = CurrencyId::Native(TokenSymbol::BNC);
	let e02 = CurrencyId::Native(TokenSymbol::KUSD);
	let e03 = CurrencyId::Native(TokenSymbol::DOT);
	let e04 = CurrencyId::Native(TokenSymbol::KSM);
	let e05 = CurrencyId::Native(TokenSymbol::ETH);
	let e06 = CurrencyId::Native(TokenSymbol::KAR);

	assert_eq!(e00, CurrencyId::try_from(0x0000_0000_0000_0000).unwrap());

	assert_eq!(e01, CurrencyId::try_from(0x0000_0000_0000_0001).unwrap());
	assert_eq!(e02, CurrencyId::try_from(0x0000_0000_0000_0002).unwrap());
	assert_eq!(e03, CurrencyId::try_from(0x0000_0000_0000_0003).unwrap());
	assert_eq!(e04, CurrencyId::try_from(0x0000_0000_0000_0004).unwrap());
	assert_eq!(e05, CurrencyId::try_from(0x0000_0000_0000_0005).unwrap());
	assert_eq!(e06, CurrencyId::try_from(0x0000_0000_0000_0006).unwrap());

	let e10 = CurrencyId::VToken(TokenSymbol::ASG);
	let e11 = CurrencyId::VToken(TokenSymbol::BNC);
	let e12 = CurrencyId::VToken(TokenSymbol::KUSD);
	let e13 = CurrencyId::VToken(TokenSymbol::DOT);
	let e14 = CurrencyId::VToken(TokenSymbol::KSM);
	let e15 = CurrencyId::VToken(TokenSymbol::ETH);
	let e16 = CurrencyId::VToken(TokenSymbol::KAR);

	assert_eq!(e10, CurrencyId::try_from(0x0000_0000_0000_0100).unwrap());
	assert_eq!(e11, CurrencyId::try_from(0x0000_0000_0000_0101).unwrap());
	assert_eq!(e12, CurrencyId::try_from(0x0000_0000_0000_0102).unwrap());
	assert_eq!(e13, CurrencyId::try_from(0x0000_0000_0000_0103).unwrap());
	assert_eq!(e14, CurrencyId::try_from(0x0000_0000_0000_0104).unwrap());
	assert_eq!(e15, CurrencyId::try_from(0x0000_0000_0000_0105).unwrap());
	assert_eq!(e16, CurrencyId::try_from(0x0000_0000_0000_0106).unwrap());

	let e20 = CurrencyId::Token(TokenSymbol::ASG);
	let e21 = CurrencyId::Token(TokenSymbol::BNC);
	let e22 = CurrencyId::Token(TokenSymbol::KUSD);
	let e23 = CurrencyId::Token(TokenSymbol::DOT);
	let e24 = CurrencyId::Token(TokenSymbol::KSM);
	let e25 = CurrencyId::Token(TokenSymbol::ETH);
	let e26 = CurrencyId::Token(TokenSymbol::KAR);

	assert_eq!(e20, CurrencyId::try_from(0x0000_0000_0000_0200).unwrap());
	assert_eq!(e21, CurrencyId::try_from(0x0000_0000_0000_0201).unwrap());
	assert_eq!(e22, CurrencyId::try_from(0x0000_0000_0000_0202).unwrap());
	assert_eq!(e23, CurrencyId::try_from(0x0000_0000_0000_0203).unwrap());
	assert_eq!(e24, CurrencyId::try_from(0x0000_0000_0000_0204).unwrap());
	assert_eq!(e25, CurrencyId::try_from(0x0000_0000_0000_0205).unwrap());
	assert_eq!(e26, CurrencyId::try_from(0x0000_0000_0000_0206).unwrap());

	let e30 = CurrencyId::Stable(TokenSymbol::ASG);
	let e31 = CurrencyId::Stable(TokenSymbol::BNC);
	let e32 = CurrencyId::Stable(TokenSymbol::KUSD);
	let e33 = CurrencyId::Stable(TokenSymbol::DOT);
	let e34 = CurrencyId::Stable(TokenSymbol::KSM);
	let e35 = CurrencyId::Stable(TokenSymbol::ETH);
	let e36 = CurrencyId::Stable(TokenSymbol::KAR);

	assert_eq!(e30, CurrencyId::try_from(0x0000_0000_0000_0300).unwrap());
	assert_eq!(e31, CurrencyId::try_from(0x0000_0000_0000_0301).unwrap());
	assert_eq!(e32, CurrencyId::try_from(0x0000_0000_0000_0302).unwrap());
	assert_eq!(e33, CurrencyId::try_from(0x0000_0000_0000_0303).unwrap());
	assert_eq!(e34, CurrencyId::try_from(0x0000_0000_0000_0304).unwrap());
	assert_eq!(e35, CurrencyId::try_from(0x0000_0000_0000_0305).unwrap());
	assert_eq!(e36, CurrencyId::try_from(0x0000_0000_0000_0306).unwrap());

	let e40 = CurrencyId::VSToken(TokenSymbol::ASG);
	let e41 = CurrencyId::VSToken(TokenSymbol::BNC);
	let e42 = CurrencyId::VSToken(TokenSymbol::KUSD);
	let e43 = CurrencyId::VSToken(TokenSymbol::DOT);
	let e44 = CurrencyId::VSToken(TokenSymbol::KSM);
	let e45 = CurrencyId::VSToken(TokenSymbol::ETH);
	let e46 = CurrencyId::VSToken(TokenSymbol::KAR);

	assert_eq!(e40, CurrencyId::try_from(0x0000_0000_0000_0400).unwrap());
	assert_eq!(e41, CurrencyId::try_from(0x0000_0000_0000_0401).unwrap());
	assert_eq!(e42, CurrencyId::try_from(0x0000_0000_0000_0402).unwrap());
	assert_eq!(e43, CurrencyId::try_from(0x0000_0000_0000_0403).unwrap());
	assert_eq!(e44, CurrencyId::try_from(0x0000_0000_0000_0404).unwrap());
	assert_eq!(e45, CurrencyId::try_from(0x0000_0000_0000_0405).unwrap());
	assert_eq!(e46, CurrencyId::try_from(0x0000_0000_0000_0406).unwrap());

	let e50 = CurrencyId::VSBond(TokenSymbol::ASG, 0x07d0, 0x0000, 0x000f);
	let e51 = CurrencyId::VSBond(TokenSymbol::BNC, 0x07d1, 0x000f, 0x001f);
	let e52 = CurrencyId::VSBond(TokenSymbol::KUSD, 0x07d2, 0x001f, 0x002f);
	let e53 = CurrencyId::VSBond(TokenSymbol::DOT, 0x07d3, 0x002f, 0x003f);
	let e54 = CurrencyId::VSBond(TokenSymbol::KSM, 0x07d4, 0x003f, 0x004f);
	let e55 = CurrencyId::VSBond(TokenSymbol::ETH, 0x07d5, 0x004f, 0x005f);
	let e56 = CurrencyId::VSBond(TokenSymbol::KAR, 0x07d6, 0x005f, 0x006f);

	assert_eq!(e50, CurrencyId::try_from(0x07d0_0000_000f_0500).unwrap());
	assert_eq!(e51, CurrencyId::try_from(0x07d1_000f_001f_0501).unwrap());
	assert_eq!(e52, CurrencyId::try_from(0x07d2_001f_002f_0502).unwrap());
	assert_eq!(e53, CurrencyId::try_from(0x07d3_002f_003f_0503).unwrap());
	assert_eq!(e54, CurrencyId::try_from(0x07d4_003f_004f_0504).unwrap());
	assert_eq!(e55, CurrencyId::try_from(0x07d5_004f_005f_0505).unwrap());
	assert_eq!(e56, CurrencyId::try_from(0x07d6_005f_006f_0506).unwrap());

	let e60 = CurrencyId::LPToken(TokenSymbol::ASG, 0u8, TokenSymbol::BNC, 0u8);
	let e61 = CurrencyId::LPToken(TokenSymbol::KUSD, 0u8, TokenSymbol::DOT, 1u8);
	let e62 = CurrencyId::LPToken(TokenSymbol::KSM, 1u8, TokenSymbol::ETH, 2u8);
	let e63 = CurrencyId::LPToken(TokenSymbol::ASG, 3u8, TokenSymbol::KAR, 4u8);

	assert_eq!(e60, CurrencyId::try_from(0x0000_0001_0000_0600).unwrap());
	assert_eq!(e61, CurrencyId::try_from(0x0000_0103_0002_0600).unwrap());
	assert_eq!(e62, CurrencyId::try_from(0x0000_0205_0104_0600).unwrap());
	assert_eq!(e63, CurrencyId::try_from(0x0000_0406_0300_0600).unwrap());

	let e70 = CurrencyId::ForeignAsset(0);
	let e71 = CurrencyId::ForeignAsset(1);
	let e72 = CurrencyId::ForeignAsset(255);
	let e73 = CurrencyId::ForeignAsset(256);
	let e74 = CurrencyId::ForeignAsset(ForeignAssetId::MAX);

	assert_eq!(e70, CurrencyId::try_from(0x0000_0000_0000_0700).unwrap());
	assert_eq!(e71, CurrencyId::try_from(0x0000_0000_0001_0700).unwrap());
	assert_eq!(e72, CurrencyId::try_from(0x0000_0000_00ff_0700).unwrap());
	assert_eq!(e73, CurrencyId::try_from(0x0000_0000_0100_0700).unwrap());
	assert_eq!(e74, CurrencyId::try_from(0x0000_ffff_ffff_0700).unwrap());

	let e80 = CurrencyId::Token2(0);
	let e81 = CurrencyId::Token2(1);
	let e82 = CurrencyId::Token2(255);
	let e83 = CurrencyId::Token2(TokenId::MAX);

	assert_eq!(e80, CurrencyId::try_from(0x0000_0000_0000_0800).unwrap());
	assert_eq!(e81, CurrencyId::try_from(0x0000_0000_0000_0801).unwrap());
	assert_eq!(e82, CurrencyId::try_from(0x0000_0000_0000_08ff).unwrap());
	assert_eq!(e83, CurrencyId::try_from(0x0000_0000_0000_08ff).unwrap());

	let e90 = CurrencyId::VToken2(0);
	let e91 = CurrencyId::VToken2(1);
	let e92 = CurrencyId::VToken2(255);
	let e93 = CurrencyId::VToken2(TokenId::MAX);

	assert_eq!(e90, CurrencyId::try_from(0x0000_0000_0000_0900).unwrap());
	assert_eq!(e91, CurrencyId::try_from(0x0000_0000_0000_0901).unwrap());
	assert_eq!(e92, CurrencyId::try_from(0x0000_0000_0000_09ff).unwrap());
	assert_eq!(e93, CurrencyId::try_from(0x0000_0000_0000_09ff).unwrap());

	let ea0 = CurrencyId::VSToken2(0);
	let ea1 = CurrencyId::VSToken2(1);
	let ea2 = CurrencyId::VSToken2(255);
	let ea3 = CurrencyId::VSToken2(TokenId::MAX);

	assert_eq!(ea0, CurrencyId::try_from(0x0000_0000_0000_0a00).unwrap());
	assert_eq!(ea1, CurrencyId::try_from(0x0000_0000_0000_0a01).unwrap());
	assert_eq!(ea2, CurrencyId::try_from(0x0000_0000_0000_0aff).unwrap());
	assert_eq!(ea3, CurrencyId::try_from(0x0000_0000_0000_0aff).unwrap());

	let eb0 = CurrencyId::VSBond2(0, 0x07d0, 0x0000, 0x000f);
	let eb1 = CurrencyId::VSBond2(1, 0x07d1, 0x000f, 0x001f);
	let eb2 = CurrencyId::VSBond2(2, 0x07d2, 0x001f, 0x002f);
	let eb3 = CurrencyId::VSBond2(3, 0x07d3, 0x002f, 0x003f);
	let eb4 = CurrencyId::VSBond2(4, 0x07d4, 0x003f, 0x004f);
	let eb5 = CurrencyId::VSBond2(5, 0x07d5, 0x004f, 0x005f);
	let eb6 = CurrencyId::VSBond2(6, 0x07d6, 0x005f, 0x006f);

	assert_eq!(eb0, CurrencyId::try_from(0x07d0_0000_000f_0b00).unwrap());
	assert_eq!(eb1, CurrencyId::try_from(0x07d1_000f_001f_0b01).unwrap());
	assert_eq!(eb2, CurrencyId::try_from(0x07d2_001f_002f_0b02).unwrap());
	assert_eq!(eb3, CurrencyId::try_from(0x07d3_002f_003f_0b03).unwrap());
	assert_eq!(eb4, CurrencyId::try_from(0x07d4_003f_004f_0b04).unwrap());
	assert_eq!(eb5, CurrencyId::try_from(0x07d5_004f_005f_0b05).unwrap());
	assert_eq!(eb6, CurrencyId::try_from(0x07d6_005f_006f_0b06).unwrap());
}

#[test]
fn to_vtoken_should_work() {
	let native_bnc = CurrencyId::Native(TokenSymbol::BNC);
	let native_asg = CurrencyId::Native(TokenSymbol::ASG);
	let vtoken_ksm = CurrencyId::VToken(TokenSymbol::KSM);
	let token_ksm = CurrencyId::Token(TokenSymbol::KSM);
	let stable_kusd = CurrencyId::Stable(TokenSymbol::KUSD);
	let vstoken_eth = CurrencyId::VSToken(TokenSymbol::ETH);
	let vsbond_ksm = CurrencyId::VSBond(TokenSymbol::KSM, 2001, 0, 1000);
	let token2_dot = CurrencyId::Token2(DOT_TOKEN_ID);
	let vtoken2_dot = CurrencyId::VToken2(DOT_TOKEN_ID);
	let vstoken2_dot = CurrencyId::VSToken2(DOT_TOKEN_ID);
	let vsbond2_dot = CurrencyId::VSBond2(DOT_TOKEN_ID, 2030, 0, 1000);
	let foreign_asset = CurrencyId::ForeignAsset(1984);
	let blp = CurrencyId::BLP(100);
	let stable_lp_token = CurrencyId::StableLpToken(100);
	let lp_token = CurrencyId::LPToken(TokenSymbol::BNC, 0, TokenSymbol::KSM, 2);

	assert_eq!(native_bnc.to_vtoken(), Ok(CurrencyId::VToken(TokenSymbol::BNC)));
	assert_eq!(native_asg.to_vtoken(), Err(()));
	assert_eq!(vtoken_ksm.to_vtoken(), Err(()));
	assert_eq!(token_ksm.to_vtoken(), Ok(CurrencyId::VToken(TokenSymbol::KSM)));
	assert_eq!(stable_kusd.to_vtoken(), Err(()));
	assert_eq!(vstoken_eth.to_vtoken(), Err(()));
	assert_eq!(vsbond_ksm.to_vtoken(), Err(()));
	assert_eq!(token2_dot.to_vtoken(), Ok(CurrencyId::VToken2(DOT_TOKEN_ID)));
	assert_eq!(vtoken2_dot.to_vtoken(), Err(()));
	assert_eq!(vstoken2_dot.to_vtoken(), Err(()));
	assert_eq!(vsbond2_dot.to_vtoken(), Err(()));
	assert_eq!(foreign_asset.to_vtoken(), Err(()));
	assert_eq!(blp.to_vtoken(), Err(()));
	assert_eq!(stable_lp_token.to_vtoken(), Err(()));
	assert_eq!(lp_token.to_vtoken(), Err(()));
}

#[test]
fn to_token_should_work() {
	let native_bnc = CurrencyId::Native(TokenSymbol::BNC);
	let vtoken_bnc = CurrencyId::VToken(TokenSymbol::BNC);
	let vtoken_ksm = CurrencyId::VToken(TokenSymbol::KSM);
	let token_ksm = CurrencyId::Token(TokenSymbol::KSM);
	let stable_kusd = CurrencyId::Stable(TokenSymbol::KUSD);
	let vstoken_eth = CurrencyId::VSToken(TokenSymbol::ETH);
	let vsbond_ksm = CurrencyId::VSBond(TokenSymbol::KSM, 2001, 0, 1000);
	let token2_dot = CurrencyId::Token2(DOT_TOKEN_ID);
	let vtoken2_dot = CurrencyId::VToken2(DOT_TOKEN_ID);
	let vstoken2_dot = CurrencyId::VSToken2(DOT_TOKEN_ID);
	let vsbond2_dot = CurrencyId::VSBond2(DOT_TOKEN_ID, 2030, 0, 1000);
	let foreign_asset = CurrencyId::ForeignAsset(1984);
	let blp = CurrencyId::BLP(100);
	let stable_lp_token = CurrencyId::StableLpToken(100);
	let lp_token = CurrencyId::LPToken(TokenSymbol::BNC, 0, TokenSymbol::KSM, 2);

	assert_eq!(native_bnc.to_token(), Err(()));
	assert_eq!(vtoken_bnc.to_token(), Ok(CurrencyId::Native(TokenSymbol::BNC)));
	assert_eq!(vtoken_ksm.to_token(), Ok(CurrencyId::Token(TokenSymbol::KSM)));
	assert_eq!(token_ksm.to_token(), Err(()));
	assert_eq!(stable_kusd.to_token(), Err(()));
	assert_eq!(vstoken_eth.to_token(), Err(()));
	assert_eq!(vsbond_ksm.to_token(), Err(()));
	assert_eq!(token2_dot.to_token(), Err(()));
	assert_eq!(vtoken2_dot.to_token(), Ok(CurrencyId::Token2(DOT_TOKEN_ID)));
	assert_eq!(vstoken2_dot.to_token(), Err(()));
	assert_eq!(vsbond2_dot.to_token(), Err(()));
	assert_eq!(foreign_asset.to_token(), Err(()));
	assert_eq!(blp.to_token(), Err(()));
	assert_eq!(stable_lp_token.to_token(), Err(()));
	assert_eq!(lp_token.to_token(), Err(()));
}
