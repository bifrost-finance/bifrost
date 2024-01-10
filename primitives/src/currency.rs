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

//! Low-level types used throughout the Bifrost code.

use bstringify::bstringify;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::{
	convert::{TryFrom, TryInto},
	prelude::*,
};
use zenlink_protocol::{AssetId, LOCAL, NATIVE};

use crate::{
	traits::{CurrencyIdExt, TokenInfo},
	LeasePeriod, ParaId, PoolId, TryConvertFrom,
};

pub const MOVR: CurrencyId = CurrencyId::Token(TokenSymbol::MOVR);
pub const VMOVR: CurrencyId = CurrencyId::VToken(TokenSymbol::MOVR);
pub const BNC: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
pub const VBNC: CurrencyId = CurrencyId::VToken(TokenSymbol::BNC);
pub const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
pub const VKSM: CurrencyId = CurrencyId::VToken(TokenSymbol::KSM);
pub const VSKSM: CurrencyId = CurrencyId::VSToken(TokenSymbol::KSM);
pub const PHA: CurrencyId = CurrencyId::Token(TokenSymbol::PHA);
pub const VPHA: CurrencyId = CurrencyId::VToken(TokenSymbol::PHA);
pub const ZLK: CurrencyId = CurrencyId::Token(TokenSymbol::ZLK);

pub const DOT_TOKEN_ID: u8 = 0u8;
pub const DOT: CurrencyId = CurrencyId::Token2(DOT_TOKEN_ID);
pub const VDOT: CurrencyId = CurrencyId::VToken2(DOT_TOKEN_ID);
pub const GLMR_TOKEN_ID: u8 = 1u8;
pub const GLMR: CurrencyId = CurrencyId::Token2(GLMR_TOKEN_ID);
pub const VGLMR: CurrencyId = CurrencyId::VToken2(GLMR_TOKEN_ID);
pub const DOT_U_TOKEN_ID: u8 = 2u8;
pub const DOT_U: CurrencyId = CurrencyId::Token2(DOT_U_TOKEN_ID);
pub const ASTR_TOKEN_ID: u8 = 3u8;
pub const ASTR: CurrencyId = CurrencyId::Token2(ASTR_TOKEN_ID);
pub const FIL_TOKEN_ID: u8 = 4u8;
pub const FIL: CurrencyId = CurrencyId::Token2(FIL_TOKEN_ID);
pub const VFIL: CurrencyId = CurrencyId::VToken2(FIL_TOKEN_ID);
pub const MANTA_TOKEN_ID: u8 = 8u8;
pub const MANTA: CurrencyId = CurrencyId::Token2(MANTA_TOKEN_ID);
pub const VMANTA: CurrencyId = CurrencyId::VToken2(MANTA_TOKEN_ID);
pub const VSBOND_BNC_2001_0_8: CurrencyId = CurrencyId::VSBond(TokenSymbol::BNC, 2001, 0, 8);

pub const LDOT: CurrencyId = CurrencyId::Lend(0);
pub const LKSM: CurrencyId = CurrencyId::Lend(1);
pub const LUSDT: CurrencyId = CurrencyId::Lend(2);
pub const LVDOT: CurrencyId = CurrencyId::Lend(3);

macro_rules! create_currency_id {
	($(#[$meta:meta])*
	$vis:vis enum TokenSymbol {
		$($(#[$vmeta:meta])* $symbol:ident($name:expr, $deci:literal) = $val:literal,)*
	}) => {
		$(#[$meta])*
		$vis enum TokenSymbol {
			$($(#[$vmeta])* $symbol = $val,)*
		}

		impl TryFrom<u8> for TokenSymbol {
			type Error = ();

			fn try_from(v: u8) -> Result<Self, Self::Error> {
				match v {
					$($val => Ok(TokenSymbol::$symbol),)*
					_ => Err(()),
				}
			}
		}

		impl TryFrom<Vec<u8>> for CurrencyId {
			type Error = ();
			fn try_from(v: Vec<u8>) -> Result<CurrencyId, ()> {
				match v.as_slice() {
					$(bstringify!($symbol) => Ok(CurrencyId::Token(TokenSymbol::$symbol)),)*
					_ => Err(()),
				}
			}
		}

		impl TryConvertFrom<CurrencyId> for AssetId {
			// DATA LAYOUT
			//
			// Empty:					 2bytes
			// Currency Discriminant:    1byte
			// TokenSymbol Index:        1byte
			type Error = ();
			fn try_convert_from(id: CurrencyId, para_id: u32) -> Result<AssetId, ()> {
				let _index = match id {
					$(CurrencyId::Native(TokenSymbol::$symbol) => Ok((0_u64, TokenSymbol::$symbol as u64)),)*
					$(CurrencyId::VToken(TokenSymbol::$symbol) => Ok((1_u64, TokenSymbol::$symbol as u64)),)*
					$(CurrencyId::Token(TokenSymbol::$symbol) => Ok((2_u64, TokenSymbol::$symbol as u64)),)*
					$(CurrencyId::Stable(TokenSymbol::$symbol) => Ok((3_u64, TokenSymbol::$symbol as u64)),)*
					$(CurrencyId::VSToken(TokenSymbol::$symbol) => Ok((4_u64, TokenSymbol::$symbol as u64)),)*
					CurrencyId::LPToken(symbol0, index0, symbol1, index1) => {
						let currency_index0 =
							(((((index0 as u64) << 8) & 0x0000_ff00) + (symbol0 as u64 & 0x0000_00ff)) as u64) << 16;
						let currency_index1 =
							(((((index1 as u64) << 8) & 0x0000_ff00) + (symbol1 as u64 & 0x0000_00ff)) as u64) << 32;
						Ok((6 as u64, currency_index0 + currency_index1))
					},
					CurrencyId::Token2(token_id) => Ok((8_u64, token_id as u64)),
					CurrencyId::VToken2(token_id) => Ok((9_u64, token_id as u64)),
					CurrencyId::VSToken2(token_id) => Ok((10_u64, token_id as u64)),
					// ForeignAsset, vsbond and vsbond2 are not allowed to be transferred to zenlink pool(c_disc is 7, 5 and 11).
					_ => Err(()),
				};
				let asset_index = ((_index?.0 << 8) & 0x0000_ff00) + (_index?.1 & 0x0000_00ff);
				if id.is_native() {
					Ok(AssetId { chain_id: para_id, asset_type: NATIVE, asset_index: 0 })
				} else {
					Ok(AssetId {
						chain_id: para_id,
						asset_type: LOCAL,
						asset_index: asset_index as u64,
					})
				}
			}
		}

		impl TryInto<CurrencyId> for AssetId {
			// DATA LAYOUT
			//
			// Empty:					2bytes
			// Currency Discriminant:   1byte
			// TokenSymbol Index:       1byte
			type Error = ();
			fn try_into(self) -> Result<CurrencyId, Self::Error> {
				let id = self.asset_index;
				let c_discr = ((id & 0x0000_0000_0000_ff00) >> 8) as u32;
				let _index = (0x0000_00ff & id) as u8;

				match c_discr {
					0 => Ok(CurrencyId::Native(TokenSymbol::try_from(_index)?)),
					1 => Ok(CurrencyId::VToken(TokenSymbol::try_from(_index)?)),
					2 => Ok(CurrencyId::Token(TokenSymbol::try_from(_index)?)),
					3 => Ok(CurrencyId::Stable(TokenSymbol::try_from(_index)?)),
					4 => Ok(CurrencyId::VSToken(TokenSymbol::try_from(_index)?)),
					6 => Ok(CurrencyId::try_from(id)?),
					8 => Ok(CurrencyId::Token2(_index)),
					9 => Ok(CurrencyId::VToken2(_index)),
					10 => Ok(CurrencyId::VSToken2(_index)),
					_ => Err(()),
				}
			}
		}


		impl TokenInfo for CurrencyId {
			fn name(&self) -> Option<&str> {
				match self {
					$(CurrencyId::Native(TokenSymbol::$symbol) => Some($name),)*
					$(CurrencyId::Stable(TokenSymbol::$symbol) => Some($name),)*
					$(CurrencyId::Token(TokenSymbol::$symbol) => Some($name),)*
					$(CurrencyId::VToken(TokenSymbol::$symbol) => Some($name),)*
					$(CurrencyId::VSToken(TokenSymbol::$symbol) => Some($name),)*
					$(CurrencyId::VSBond(TokenSymbol::$symbol, ..) => Some($name),)*
					CurrencyId::LPToken(ts1, type1, ts2, type2) => {
						let c1_u64: u64 = (((*type1 as u64) << 8) & 0x0000_0000_0000_ff00) + ((*ts1 as u64) & 0x0000_0000_0000_00ff);
						let c2_u64: u64 = (((*type2 as u64) << 8) & 0x0000_0000_0000_ff00) + ((*ts2 as u64) & 0x0000_0000_0000_00ff);

						let _c1: CurrencyId = c1_u64.try_into().unwrap_or_default();
						let _c2: CurrencyId = c2_u64.try_into().unwrap_or_default();
						Some(stringify!(_c1.name(), ",", _c2.name()))
					},
					CurrencyId::StableLpToken(..) => Some(stringify!("stable_pool_lp",)),
					_ => None
				}
			}

			fn symbol(&self) -> Option<&str> {
				match self {
					$(CurrencyId::Native(TokenSymbol::$symbol) => Some(stringify!($symbol)),)*
					$(CurrencyId::Stable(TokenSymbol::$symbol) => Some(stringify!($symbol)),)*
					$(CurrencyId::Token(TokenSymbol::$symbol) => Some(stringify!($symbol)),)*
					$(CurrencyId::VToken(TokenSymbol::$symbol) => Some(stringify!($symbol)),)*
					$(CurrencyId::VSToken(TokenSymbol::$symbol) => Some(stringify!($symbol)),)*
					$(CurrencyId::VSBond(TokenSymbol::$symbol, ..) => Some(stringify!($symbol)),)*
					CurrencyId::LPToken(_ts1, _, _ts2, _) => Some(stringify!(_ts1, ",", _ts2)),
					CurrencyId::StableLpToken(..) => Some(stringify!("stable_pool_lp_")),
					_ => None
				}
			}

			fn decimals(&self) -> Option<u8> {
				match self {
					$(CurrencyId::Native(TokenSymbol::$symbol) => Some($deci),)*
					$(CurrencyId::Stable(TokenSymbol::$symbol) => Some($deci),)*
					$(CurrencyId::Token(TokenSymbol::$symbol) => Some($deci),)*
					$(CurrencyId::VToken(TokenSymbol::$symbol) => Some($deci),)*
					$(CurrencyId::VSToken(TokenSymbol::$symbol) => Some($deci),)*
					$(CurrencyId::VSBond(TokenSymbol::$symbol, ..) => Some($deci),)*
					CurrencyId::LPToken(..) => Some(1u8),
					CurrencyId::StableLpToken(..) => Some(1u8),
					_ => None
				}
			}
		}

		// $(pub const $symbol: CurrencyId = CurrencyId::Token(TokenSymbol::$symbol);)*

		impl TokenSymbol {
			pub fn get_info() -> Vec<(&'static str, u32)> {
				vec![
					$((stringify!($symbol), $deci),)*
				]
			}
		}
    }
}

// Bifrost Tokens list
create_currency_id! {
	// Represent a Token symbol with 8 bit
	// Bit 8 : 0 for Pokladot Ecosystem, 1 for Kusama Ecosystem
	// Bit 7 : Reserved
	// Bit 6 - 1 : The token ID
	#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, TypeInfo, MaxEncodedLen, Serialize, Deserialize)]
	#[repr(u8)]
	pub enum TokenSymbol {
		ASG("Asgard", 12) = 0,
		BNC("Bifrost", 12) = 1,
		KUSD("Karura Dollar", 12) = 2,
		DOT("Polkadot", 10) = 3,
		KSM("Kusama", 12) = 4,
		ETH("Ethereum", 18) = 5,
		KAR("Karura", 12) = 6,
		ZLK("Zenlink Network Token", 18) = 7,
		PHA("Phala Native Token", 12) = 8,
		RMRK("RMRK Token",10) = 9,
		MOVR("Moonriver Native Token",18) = 10,
	}
}

impl Default for TokenSymbol {
	fn default() -> Self {
		Self::BNC
	}
}

pub type ForeignAssetId = u32;
pub type TokenId = u8;

/// Currency ID, it might be extended with more variants in the future.
#[derive(
	Encode,
	Decode,
	MaxEncodedLen,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	Ord,
	TypeInfo,
	Serialize,
	Deserialize,
)]
#[non_exhaustive]
pub enum CurrencyId {
	Native(TokenSymbol),
	VToken(TokenSymbol),
	Token(TokenSymbol),
	Stable(TokenSymbol),
	VSToken(TokenSymbol),
	VSBond(TokenSymbol, ParaId, LeasePeriod, LeasePeriod),
	// [currency1 Tokensymbol, currency1 TokenType, currency2 TokenSymbol, currency2 TokenType]
	LPToken(TokenSymbol, u8, TokenSymbol, u8),
	ForeignAsset(ForeignAssetId),
	Token2(TokenId),
	VToken2(TokenId),
	VSToken2(TokenId),
	VSBond2(TokenId, ParaId, LeasePeriod, LeasePeriod),
	StableLpToken(PoolId),
	BLP(PoolId),
	Lend(TokenId),
}

impl Default for CurrencyId {
	fn default() -> Self {
		Self::Native(Default::default())
	}
}

impl From<TokenSymbol> for CurrencyId {
	fn from(symbol: TokenSymbol) -> Self {
		Self::Token(symbol)
	}
}

impl CurrencyId {
	#[allow(non_snake_case)]
	pub const fn vsAssets(
		symbol: TokenSymbol,
		index: ParaId,
		first_slot: LeasePeriod,
		last_slot: LeasePeriod,
	) -> (Self, Self) {
		let vsbond_origin = CurrencyId::VSBond(symbol, index, first_slot, last_slot);

		let vsbond_fixed = match vsbond_origin {
			Self::VSBond(TokenSymbol::KSM, 2001, 13, 20) =>
				Self::VSBond(TokenSymbol::BNC, 2001, 13, 20),
			_ => vsbond_origin,
		};

		(Self::VSToken(symbol), vsbond_fixed)
	}

	pub fn to_token(&self) -> Result<Self, ()> {
		match self {
			Self::VToken(TokenSymbol::BNC) => Ok(Self::Native(TokenSymbol::BNC)),
			Self::VToken(symbol) => Ok(Self::Token(*symbol)),
			Self::VToken2(id) => Ok(Self::Token2(*id)),
			_ => Err(()),
		}
	}

	pub fn to_vtoken(&self) -> Result<Self, ()> {
		match self {
			Self::Token(symbol) => Ok(Self::VToken(*symbol)),
			Self::Token2(id) => Ok(Self::VToken2(*id)),
			Self::Native(TokenSymbol::BNC) => Ok(Self::VToken(TokenSymbol::BNC)),
			_ => Err(()),
		}
	}

	pub fn to_vstoken(&self) -> Result<Self, ()> {
		match self {
			Self::Token(symbol) => Ok(Self::VSToken(*symbol)),
			Self::Token2(id) => Ok(Self::VSToken2(*id)),
			_ => Err(()),
		}
	}
}

impl CurrencyIdExt for CurrencyId {
	type TokenSymbol = TokenSymbol;

	fn is_vtoken(&self) -> bool {
		matches!(self, CurrencyId::VToken(_) | CurrencyId::VToken2(_))
	}

	fn is_token(&self) -> bool {
		matches!(self, CurrencyId::Token(_) | CurrencyId::Token2(_))
	}

	fn is_vstoken(&self) -> bool {
		matches!(self, CurrencyId::VSToken(_) | CurrencyId::VSToken2(_))
	}

	fn is_vsbond(&self) -> bool {
		matches!(self, CurrencyId::VSBond(..) | CurrencyId::VSBond2(..))
	}

	fn is_native(&self) -> bool {
		matches!(self, CurrencyId::Native(_))
	}

	fn is_stable(&self) -> bool {
		matches!(self, CurrencyId::Stable(_))
	}

	fn is_lptoken(&self) -> bool {
		matches!(self, CurrencyId::LPToken(..))
	}

	fn is_foreign_asset(&self) -> bool {
		matches!(self, CurrencyId::ForeignAsset(..))
	}

	fn into(symbol: Self::TokenSymbol) -> Self {
		CurrencyId::Token(symbol)
	}
}

impl TryFrom<u64> for CurrencyId {
	type Error = ();

	fn try_from(id: u64) -> Result<Self, Self::Error> {
		let c_discr = ((id & 0x0000_0000_0000_ff00) >> 8) as u8;

		let t_discr = ((id & 0x0000_0000_0000_00ff) >> 00) as u8;

		let pid = ((id & 0xffff_0000_0000_0000) >> 48) as u32;
		let lp1 = ((id & 0x0000_ffff_0000_0000) >> 32) as u32;
		let lp2 = ((id & 0x0000_0000_ffff_0000) >> 16) as u32;

		match c_discr {
			0 => Ok(Self::Native(TokenSymbol::try_from(t_discr)?)),
			1 => Ok(Self::VToken(TokenSymbol::try_from(t_discr)?)),
			2 => Ok(Self::Token(TokenSymbol::try_from(t_discr)?)),
			3 => Ok(Self::Stable(TokenSymbol::try_from(t_discr)?)),
			4 => Ok(Self::VSToken(TokenSymbol::try_from(t_discr)?)),
			5 => Ok(Self::VSBond(TokenSymbol::try_from(t_discr)?, pid, lp1, lp2)),
			6 => {
				let token_symbol_num_1 = ((id & 0x0000_0000_00ff_0000) >> 16) as u8;
				let token_type_1 = ((id & 0x0000_0000_ff00_0000) >> 24) as u8;
				let token_symbol_num_2 = ((id & 0x0000_00ff_0000_0000) >> 32) as u8;
				let token_type_2 = ((id & 0x0000_ff00_0000_0000) >> 40) as u8;

				let token_symbol_1 = TokenSymbol::try_from(token_symbol_num_1).unwrap_or_default();
				let token_symbol_2 = TokenSymbol::try_from(token_symbol_num_2).unwrap_or_default();

				Ok(Self::LPToken(token_symbol_1, token_type_1, token_symbol_2, token_type_2))
			},
			7 => {
				let foreign_asset_id = ((id & 0x0000_ffff_ffff_0000) >> 16) as ForeignAssetId;
				Ok(Self::ForeignAsset(foreign_asset_id))
			},
			8 => {
				let token_id = t_discr as TokenId;
				Ok(Self::Token2(token_id))
			},
			9 => {
				let token_id = t_discr as TokenId;
				Ok(Self::VToken2(token_id))
			},
			10 => {
				let token_id = t_discr as TokenId;
				Ok(Self::VSToken2(token_id))
			},
			11 => {
				let token_id = t_discr as TokenId;
				Ok(Self::VSBond2(token_id, pid, lp1, lp2))
			},
			_ => Err(()),
		}
	}
}

#[derive(Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, TypeInfo)]
pub enum AssetIds {
	ForeignAssetId(ForeignAssetId),
	NativeAssetId(CurrencyId),
}
