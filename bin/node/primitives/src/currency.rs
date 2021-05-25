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

use codec::{Encode, Decode};
use core::{convert::TryFrom, ops::Deref};
use crate::traits::{CurrencyIdExt, GetDecimals};
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Bifrost Tokens list
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[non_exhaustive]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum TokenSymbol {
	// Native token
	ASG = 0_u8,
	// Acala
	aUSD = 1u8,
	// Polkadot
	DOT = 2u8,
	vDOT = 3u8,
	// Kusama
	KSM = 4u8,
	vKSM = 5u8,
	// Ethereum
	ETH = 6u8,
	vETH = 7u8,
	// EOS
	EOS = 8u8,
	vEOS = 9u8,
	// IOST
	IOST = 10u8,
	vIOST = 11u8,
}

impl Default for TokenSymbol {
	fn default() -> Self {
		Self::ASG
	}
}

/// TokenSymbol from a number 
impl From<u8> for TokenSymbol {
	fn from(n: u8) -> Self {
		match n {
			0 => Self::ASG,
			1 => Self::aUSD,
			2 => Self::DOT,
			3 => Self::vDOT,
			4 => Self::KSM,
			5 => Self::vKSM,
			6 => Self::ETH,
			7 => Self::vETH,
			8 => Self::EOS,
			9 => Self::vEOS,
			10 => Self::IOST,
			11 => Self::vIOST,
			_ => todo!("Not support now."),
		}
	}
}

impl From<CurrencyId> for TokenSymbol {
	fn from(currency_id: CurrencyId) -> Self {
		match currency_id {
			CurrencyId::Token(TokenSymbol::ASG) => Self::ASG,
			CurrencyId::Token(TokenSymbol::aUSD) => Self::aUSD,
			CurrencyId::Token(TokenSymbol::DOT) => Self::DOT,
			CurrencyId::Token(TokenSymbol::vDOT) => Self::vDOT,
			CurrencyId::Token(TokenSymbol::KSM) => Self::KSM,
			CurrencyId::Token(TokenSymbol::vKSM) => Self::vKSM,
			CurrencyId::Token(TokenSymbol::ETH) => Self::ETH,
			CurrencyId::Token(TokenSymbol::vETH) => Self::vETH,
			CurrencyId::Token(TokenSymbol::EOS) => Self::EOS,
			CurrencyId::Token(TokenSymbol::vEOS) => Self::vEOS,
			CurrencyId::Token(TokenSymbol::IOST) => Self::IOST,
			CurrencyId::Token(TokenSymbol::vIOST) => Self::vIOST,
			_ => todo!("Not support now."),
		}
	}
}

/// List tokens precision
impl GetDecimals for TokenSymbol {
	fn decimals(&self) -> u32 {
		match *self {
			Self::ASG => 12u32,
			Self::aUSD => 12u32,
			Self::DOT => 10u32,
			Self::vDOT => 10u32,
			Self::KSM => 12u32,
			Self::vKSM => 12u32,
			Self::ETH => 18u32,
			Self::vETH => 18u32,
			Self::EOS => 12u32,
			Self::vEOS => 12u32,
			Self::IOST => 12u32,
			Self::vIOST => 12u32,
		}
	}
}

impl From<TokenSymbol> for Vec<u8> {

	fn from(token_symbol: TokenSymbol) -> Self {
		match token_symbol {
			TokenSymbol::ASG => b"ASG".to_vec(),
			TokenSymbol::aUSD => b"aUSD".to_vec(),
			TokenSymbol::DOT => b"DOT".to_vec(),
			TokenSymbol::vDOT => b"vDOT".to_vec(),
			TokenSymbol::KSM => b"KSM".to_vec(),
			TokenSymbol::vKSM => b"vKSM".to_vec(),
			TokenSymbol::ETH => b"ETH".to_vec(),
			TokenSymbol::vETH => b"vETH".to_vec(),
			TokenSymbol::EOS => b"EOS".to_vec(),
			TokenSymbol::vEOS => b"vEOS".to_vec(),
			TokenSymbol::IOST => b"IOST".to_vec(),
			TokenSymbol::vIOST => b"vIOST".to_vec(),
			_ => todo!("Not support now."),
		}
	}
}



/// Currency ID, it might be extended with more variants in the future.
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum CurrencyId {
	Native(TokenSymbol),
	Stable(TokenSymbol),
	Token(TokenSymbol),
	VToken(TokenSymbol),
	VSToken(TokenSymbol),
	VSBond(TokenSymbol),
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
		matches!(self, CurrencyId::VSBond(_))
	}

	fn is_native(&self) -> bool {
		matches!(self, CurrencyId::Native(_))
	}

	fn is_stable(&self) -> bool {
		matches!(self, CurrencyId::Stable(_))
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
			Self::VSBond(ref symbol) => symbol,
		}
	}
}

/// CurrencyId from a number
impl From<u8> for CurrencyId {
	fn from(n: u8) -> Self {
		match n {
			0 => CurrencyId::Token(n.into()),
			1 => CurrencyId::Token(n.into()),
			2 => CurrencyId::Token(n.into()),
			3 => CurrencyId::Token(n.into()),
			4 => CurrencyId::Token(n.into()),
			5 => CurrencyId::Token(n.into()),
			6 => CurrencyId::Token(n.into()),
			7 => CurrencyId::Token(n.into()),
			8 => CurrencyId::Token(n.into()),
			9 => CurrencyId::Token(n.into()),
			10 => CurrencyId::Token(n.into()),
			11 => CurrencyId::Token(n.into()),
			_ => todo!("Not support now."),
		}
	}
}

impl TryFrom<Vec<u8>> for CurrencyId {
	type Error = ();

	fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
		match v.as_slice() {
			b"ASG" => Ok(CurrencyId::Token(TokenSymbol::ASG)),
			b"aUSD" => Ok(CurrencyId::Token(TokenSymbol::aUSD)),
			b"DOT" => Ok(CurrencyId::Token(TokenSymbol::DOT)),
			b"vDOT" => Ok(CurrencyId::Token(TokenSymbol::vDOT)),
			b"KSM" => Ok(CurrencyId::Token(TokenSymbol::KSM)),
			b"vKSM" => Ok(CurrencyId::Token(TokenSymbol::vKSM)),
			b"ETH" => Ok(CurrencyId::Token(TokenSymbol::ETH)),
			b"vETH" => Ok(CurrencyId::Token(TokenSymbol::vETH)),
			b"EOS" => Ok(CurrencyId::Token(TokenSymbol::EOS)),
			b"vEOS" => Ok(CurrencyId::Token(TokenSymbol::vEOS)),
			b"IOST" => Ok(CurrencyId::Token(TokenSymbol::IOST)),
			b"vIOST" => Ok(CurrencyId::Token(TokenSymbol::vIOST)),
			_ => Err(()),
		}
	}
}

