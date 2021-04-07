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

/// Currency ID, it might be extended with more variants in the future.
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum CurrencyId {
	Token(TokenSymbol),
}

impl Default for CurrencyId {
	fn default() -> Self {
		Self::Token(TokenSymbol::ASG)
	}
}

impl From<TokenSymbol> for CurrencyId {
	fn from(symbol: TokenSymbol) -> Self {
		CurrencyId::Token(symbol)
	}
}

impl CurrencyIdExt for CurrencyId {
	/// This pair means (EOS, vEOS), token is ahead of vtoken.
    type PairTokens = (TokenSymbol, TokenSymbol);
	type TokenSymbol = TokenSymbol;
	fn is_vtoken(&self) -> bool {
		matches!(
			self.as_ref(),
			TokenSymbol::vDOT | TokenSymbol::vKSM | TokenSymbol::vETH | TokenSymbol::vEOS | TokenSymbol::vIOST
		)
	}

	fn is_token(&self) -> bool {
		matches!(
			self.as_ref(),
			TokenSymbol::DOT | TokenSymbol::KSM | TokenSymbol::ETH | TokenSymbol::EOS | TokenSymbol::IOST
		)
	}

	fn is_native(&self) -> bool {
		matches!(self.as_ref(), TokenSymbol::ASG)
	}

	fn is_stable_token(&self) -> bool {
		matches!(self.as_ref(), TokenSymbol::aUSD)
	}

	fn get_native_token(&self) -> Option<Self::TokenSymbol> {
		match self.as_ref() {
			TokenSymbol::ASG => Some(TokenSymbol::ASG),
			_ => None,
		}
	}

	fn get_stable_token(&self) -> Option<Self::TokenSymbol> {
		match self.as_ref() {
			TokenSymbol::aUSD => Some(TokenSymbol::aUSD),
			_ => None,
		}
	}

	fn get_token_pair(&self) -> Option<Self::PairTokens> {
		match self.as_ref() {
			TokenSymbol::DOT | TokenSymbol::vDOT => Some((TokenSymbol::DOT, TokenSymbol::vDOT)),
			TokenSymbol::KSM | TokenSymbol::vKSM => Some((TokenSymbol::KSM, TokenSymbol::vKSM)),
			TokenSymbol::ETH | TokenSymbol::vETH => Some((TokenSymbol::ETH, TokenSymbol::vETH)),
			TokenSymbol::EOS | TokenSymbol::vEOS => Some((TokenSymbol::EOS, TokenSymbol::vEOS)),
			TokenSymbol::IOST | TokenSymbol::vIOST => Some((TokenSymbol::IOST, TokenSymbol::vIOST)),
			_ => None,
		}
	}

	fn into(symbol: Self::TokenSymbol) -> Self {
		CurrencyId::Token(symbol)
	}
}

impl Deref for CurrencyId {
	type Target = TokenSymbol;
	fn deref(&self) -> &Self::Target {
		match *self {
			Self::Token(ref symbol) => symbol
		}
	}
}

impl AsRef<TokenSymbol>  for CurrencyId {
	fn as_ref(&self) -> &TokenSymbol {
		match *self {
			Self::Token(ref symbol) => symbol
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
