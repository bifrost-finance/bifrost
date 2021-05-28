// Copyright 2019-2021 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

//! Low-level types used throughout the Bifrost code.

use crate::traits::{CurrencyIdExt, TokenInfo};
use crate::{LeasePeriod, ParaId};
use bstringify::bstringify;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{RuntimeDebug, SaturatedConversion};
use sp_std::{
	convert::{Into, TryFrom},
	ops::Deref,
	prelude::*,
};
use zenlink_protocol::{AssetId, LOCAL, NATIVE};

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

		impl Into<u8> for TokenSymbol {
			fn into(self) -> u8 {
				match self {
					$(TokenSymbol::$symbol => ($val),)*
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

		impl TokenInfo for CurrencyId {
			fn currency_id(&self) -> u8 {
				match self {
					$(CurrencyId::Native(TokenSymbol::$symbol) => $val,)*
					$(CurrencyId::Stable(TokenSymbol::$symbol) => $val,)*
					$(CurrencyId::Token(TokenSymbol::$symbol) => $val,)*
					$(CurrencyId::VToken(TokenSymbol::$symbol) => $val,)*
					$(CurrencyId::VSToken(TokenSymbol::$symbol) => $val,)*
					$(CurrencyId::VSBond(TokenSymbol::$symbol, ..) => $val,)*
				}
			}
			fn name(&self) -> &str {
				match self {
					$(CurrencyId::Native(TokenSymbol::$symbol) => $name,)*
					$(CurrencyId::Stable(TokenSymbol::$symbol) => $name,)*
					$(CurrencyId::Token(TokenSymbol::$symbol) => $name,)*
					$(CurrencyId::VToken(TokenSymbol::$symbol) => $name,)*
					$(CurrencyId::VSToken(TokenSymbol::$symbol) => $name,)*
					$(CurrencyId::VSBond(TokenSymbol::$symbol, ..) => $name,)*
				}
			}
			fn symbol(&self) -> &str {
				match self {
					$(CurrencyId::Native(TokenSymbol::$symbol) => stringify!($symbol),)*
					$(CurrencyId::Stable(TokenSymbol::$symbol) => stringify!($symbol),)*
					$(CurrencyId::Token(TokenSymbol::$symbol) => stringify!($symbol),)*
					$(CurrencyId::VToken(TokenSymbol::$symbol) => stringify!($symbol),)*
					$(CurrencyId::VSToken(TokenSymbol::$symbol) => stringify!($symbol),)*
					$(CurrencyId::VSBond(TokenSymbol::$symbol, ..) => stringify!($symbol),)*
				}
			}
			fn decimals(&self) -> u8 {
				match self {
					$(CurrencyId::Native(TokenSymbol::$symbol) => $deci,)*
					$(CurrencyId::Stable(TokenSymbol::$symbol) => $deci,)*
					$(CurrencyId::Token(TokenSymbol::$symbol) => $deci,)*
					$(CurrencyId::VToken(TokenSymbol::$symbol) => $deci,)*
					$(CurrencyId::VSToken(TokenSymbol::$symbol) => $deci,)*
					$(CurrencyId::VSBond(TokenSymbol::$symbol, ..) => $deci,)*
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

/// Bifrost Tokens list
create_currency_id! {
	// Represent a Token symbol with 8 bit
	// Bit 8 : 0 for Pokladot Ecosystem, 1 for Kusama Ecosystem
	// Bit 7 : Reserved
	// Bit 6 - 1 : The token ID
	#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	#[repr(u8)]
	pub enum TokenSymbol {
		ASG("Asgard", 12) = 0,
		BNC("Bifrost", 12) = 1,
		AUSD("Acala Dollar", 12) = 2,
		DOT("Polkadot", 10) = 3,
		KSM("Kusama", 12) = 4,
		ETH("Ethereum", 18) = 5,
	}
}

impl Default for TokenSymbol {
	fn default() -> Self {
		Self::BNC
	}
}

/// Currency ID, it might be extended with more variants in the future.
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum CurrencyId {
	Token(TokenSymbol),
	VToken(TokenSymbol),
	Native(TokenSymbol),
	Stable(TokenSymbol),
	VSToken(TokenSymbol),
	VSBond(TokenSymbol, ParaId, LeasePeriod, LeasePeriod),
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

	fn exist(&self) -> bool {
		matches!(
			self,
			CurrencyId::VToken(_)
				| CurrencyId::Token(_)
				| CurrencyId::VSToken(_)
				| CurrencyId::VSBond(..)
				| CurrencyId::Native(_)
				| CurrencyId::Stable(_)
		)
	}

	fn into(symbol: Self::TokenSymbol) -> Self {
		CurrencyId::Token(symbol)
	}
}

impl Deref for CurrencyId {
	type Target = TokenSymbol;
	fn deref(&self) -> &Self::Target {
		match *self {
			Self::Native(ref symbol) => symbol,
			Self::Stable(ref symbol) => symbol,
			Self::Token(ref symbol) => symbol,
			Self::VToken(ref symbol) => symbol,
			Self::VSToken(ref symbol) => symbol,
			Self::VSBond(ref symbol, ..) => symbol,
		}
	}
}

// This part is for EOS and ISOT bridge.
// impl TryFrom<Vec<u8>> for CurrencyId {
// 	type Error = ();

// 	fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
// 		match v.as_slice() {
// 			b"ASG" => Ok(CurrencyId::Native(TokenSymbol::ASG)),
// 			b"AUSD" => Ok(CurrencyId::Stable(TokenSymbol::AUSD)),
// 			b"DOT" => Ok(CurrencyId::Token(TokenSymbol::DOT)),
// 			b"vDOT" => Ok(CurrencyId::VToken(TokenSymbol::DOT)),
// 			b"KSM" => Ok(CurrencyId::Token(TokenSymbol::KSM)),
// 			b"vKSM" => Ok(CurrencyId::VToken(TokenSymbol::KSM)),
// 			b"ETH" => Ok(CurrencyId::Token(TokenSymbol::ETH)),
// 			b"vETH" => Ok(CurrencyId::VToken(TokenSymbol::ETH)),
// 			b"EOS" => Ok(CurrencyId::Token(TokenSymbol::EOS)),
// 			b"vEOS" => Ok(CurrencyId::VToken(TokenSymbol::EOS)),
// 			b"IOST" => Ok(CurrencyId::Token(TokenSymbol::IOST)),
// 			b"vIOST" => Ok(CurrencyId::VToken(TokenSymbol::IOST)),
// 			_ => Err(()),
// 		}
// 	}
// }

// Below is the trait which can convert between Zenlink AssetId type and Bifrost CurrencyId type
pub const BIFROST_PARACHAIN_ID: u32 = 2001; // bifrost parachain id

// Temporary solution for conversion from Bifrost CurrencyId to Zenlink AssetId
impl From<CurrencyId> for AssetId {
	fn from(id: CurrencyId) -> Self {
		if id.is_native() {
			Self {
				chain_id: BIFROST_PARACHAIN_ID,
				asset_type: NATIVE,
				asset_index: 0u32,
			}
		} else {
			let u8_id = match id {
				CurrencyId::Stable(TokenSymbol::AUSD) => 2,

				CurrencyId::Token(TokenSymbol::DOT) => 3,
				CurrencyId::Token(TokenSymbol::KSM) => 4,
				CurrencyId::Token(TokenSymbol::ETH) => 5,

				CurrencyId::VToken(TokenSymbol::DOT) => 6,
				CurrencyId::VToken(TokenSymbol::KSM) => 7,
				CurrencyId::VToken(TokenSymbol::ETH) => 8,
				_ => todo!("Not support now."),
			};
			Self {
				chain_id: BIFROST_PARACHAIN_ID,
				asset_type: LOCAL,
				asset_index: u8_id as u32,
			}
		}
	}
}

impl Into<CurrencyId> for AssetId {
	fn into(self) -> CurrencyId {
		let id: u8 = self.asset_index.saturated_into();
		match id {
			0 => CurrencyId::Native(TokenSymbol::ASG),
			2 => CurrencyId::Stable(TokenSymbol::AUSD),

			3 => CurrencyId::Token(TokenSymbol::DOT),
			4 => CurrencyId::Token(TokenSymbol::KSM),
			5 => CurrencyId::Token(TokenSymbol::ETH),

			6 => CurrencyId::VToken(TokenSymbol::DOT),
			7 => CurrencyId::VToken(TokenSymbol::KSM),
			8 => CurrencyId::VToken(TokenSymbol::ETH),
			_ => todo!("Not support now."),
		}
	}
}
