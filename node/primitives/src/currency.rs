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

//! Low-level types used throughout the Bifrost code.

use bstringify::bstringify;
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::{
	convert::{TryFrom, TryInto},
	prelude::*,
};
use zenlink_protocol::{AssetId, LOCAL, NATIVE};

use crate::{
	traits::{CurrencyIdExt, TokenInfo},
	LeasePeriod, ParaId,
};
pub const BIFROST_PARACHAIN_ID: u32 = 2001; // bifrost parachain id

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

		impl TryFrom<CurrencyId> for AssetId {
			// DATA LAYOUT
			//
			// Empty:					 2bytes
			// Currency Discriminant:    1byte
			// TokenSymbol Index:        1byte
			type Error = ();
			fn try_from(id: CurrencyId) -> Result<AssetId, ()> {
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
					}
					_ => Err(()),
				};
				let asset_index = ((_index?.0 << 8) & 0x0000_ff00) + (_index?.1 & 0x0000_00ff);
				if id.is_native() {
					Ok(AssetId { chain_id: BIFROST_PARACHAIN_ID, asset_type: NATIVE, asset_index: 0 })
				} else {
					Ok(AssetId {
						chain_id: BIFROST_PARACHAIN_ID,
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
					_ => Err(()),
				}
			}
		}


		impl TokenInfo for CurrencyId {
			// DATA LAYOUT
			//
			// ParaId:                      2byte
			// LeasePeriod:                 2byte
			// LeasePeriod:                 2byte
			// Currency Discriminant:       1byte
			// TokenSymbol Discriminant:    1byte
			//
			// If it is LPToken:
			// Empty:						2byte
			// Currency 2 Discriminant:     1byte
			// TokenSymbol 2 Discriminant:  1byte
			// Currency 1 Discriminant:     1byte
			// TokenSymbol 2 Discriminant:  1byte
			// Currency Discriminant:       1byte
			// TokenSymbol Discriminant:    1byte

			fn currency_id(&self) -> u64 {
				let c_discr = self.discriminant() as u64;

				let t_discr = match *self {
					Self::Token(ts)
					| Self::VToken(ts)
					| Self::Native(ts)
					| Self::Stable(ts)
					| Self::VSToken(ts)
					| Self::VSBond(ts, ..) => ts as u8,
					Self::ForeignAsset(..) => 0u8,
					Self::LPToken(..) => 0u8
				} as u64;

		 		let discr = (c_discr << 8) + t_discr;

				match &*self {
					Self::Token(..)
					| Self::VToken(..)
					| Self::Native(..)
					| Self::Stable(..)
					| Self::VSToken(..) => (0x0000_ffff & discr) as u64,
					Self::VSBond(_, pid, lp1, lp2) => {
						// NOTE: ParaId representation
						//
						// The current goal is for Polkadot to support up to 100 parachains which `u8` could hold.
						// But `paraId` be represented like 2001, 2002 and so on which exceeds the range which `u8`
						//  could be represented.
						// So `u16` is a choice better than `u8`.

						// NOTE: LeasePeriod representation
						//
						// `u16` can hold the range of `LeasePeriod`

						let pid = (0x0000_ffff & pid) as u64;
						let lp1 = (0x0000_ffff & lp1) as u64;
						let lp2 = (0x0000_ffff & lp2) as u64;

						(pid << 48) + (lp1 << 32) + (lp2 << 16) + discr
					},
					Self::LPToken(token_symbol_1, token_type_1, token_symbol_2, token_type_2) => {
						(((*token_symbol_1 as u64) << 16) & 0x0000_0000_00ff_0000) + (((*token_type_1 as u64) << 24) & 0x0000_0000_ff00_0000) +
						(((*token_symbol_2 as u64) << 32) & 0x0000_00ff_0000_0000) + (((*token_type_2 as u64) << 40) & 0x0000_ff00_0000_0000) + discr
					},
					Self::ForeignAsset(asset_token_id) => {
						(((*asset_token_id as u64) << 16) & 0x0000_ffff_ffff_0000) + discr
					}
				}
			}

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
	#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, TypeInfo, MaxEncodedLen)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
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
	}
}

impl Default for TokenSymbol {
	fn default() -> Self {
		Self::BNC
	}
}

pub type ForeignAssetId = u32;

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
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
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
			Self::VToken(symbol) => Ok(Self::Token(symbol.clone())),
			_ => Err(()),
		}
	}

	pub fn to_vtoken(&self) -> Result<Self, ()> {
		match self {
			Self::Token(symbol) => Ok(Self::VToken(symbol.clone())),
			_ => Err(()),
		}
	}

	pub fn to_vstoken(&self) -> Result<Self, ()> {
		match self {
			Self::Token(symbol) => Ok(Self::VSToken(symbol.clone())),
			_ => Err(()),
		}
	}

	pub fn discriminant(&self) -> u8 {
		match *self {
			Self::Native(..) => 0,
			Self::VToken(..) => 1,
			Self::Token(..) => 2,
			Self::Stable(..) => 3,
			Self::VSToken(..) => 4,
			Self::VSBond(..) => 5,
			Self::LPToken(..) => 6,
			Self::ForeignAsset(..) => 7,
		}
	}
}

impl CurrencyIdExt for CurrencyId {
	type TokenSymbol = TokenSymbol;

	fn is_vtoken(&self) -> bool {
		matches!(self, CurrencyId::VToken(_))
	}

	fn is_token(&self) -> bool {
		matches!(self, CurrencyId::Token(_))
	}

	fn is_vstoken(&self) -> bool {
		matches!(self, CurrencyId::VSToken(_))
	}

	fn is_vsbond(&self) -> bool {
		matches!(self, CurrencyId::VSBond(..))
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

		let token_symbol = TokenSymbol::try_from(t_discr)?;

		match c_discr {
			0 => Ok(Self::Native(token_symbol)),
			1 => Ok(Self::VToken(token_symbol)),
			2 => Ok(Self::Token(token_symbol)),
			3 => Ok(Self::Stable(token_symbol)),
			4 => Ok(Self::VSToken(token_symbol)),
			5 => Ok(Self::VSBond(token_symbol, pid, lp1, lp2)),
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
			_ => Err(()),
		}
	}
}
