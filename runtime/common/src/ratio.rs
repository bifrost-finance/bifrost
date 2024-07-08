use core::cmp::{Ord, Ordering, PartialOrd};
use num_traits::Zero;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_arithmetic::helpers_128bit;

/// A rational number represented by a `n`umerator and `d`enominator.
#[derive(
	Clone,
	Copy,
	Default,
	PartialEq,
	Eq,
	Encode,
	Decode,
	Serialize,
	Deserialize,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct Ratio {
	pub n: u128,
	pub d: u128,
}

impl Ratio {
	/// Build from a raw `n/d`. Ensures that `d > 0`.
	pub const fn new(n: u128, d: u128) -> Self {
		// reimplement `.max(1)` so this can be `const`
		let d = if d > 0 { d } else { 1 };
		Self { n, d }
	}

	/// Build from a raw `n/d`. This could lead to / 0 if not properly handled.
	pub const fn new_unchecked(n: u128, d: u128) -> Self {
		Self { n, d }
	}

	/// Return a representation of one.
	///
	/// Note that more than one combination of `n` and `d` can be one.
	pub const fn one() -> Self {
		Self::new_unchecked(1, 1)
	}

	/// Return whether `self` is one.
	///
	/// Should a denominator of 0 happen, this function will return `false`.
	///
	/// Note that more than one combination of `n` and `d` can be one.
	pub const fn is_one(&self) -> bool {
		self.d > 0 && self.n == self.d
	}

	/// Return a representation of zero.
	///
	/// Note that any combination of `n == 0` and `d` represents zero.
	pub const fn zero() -> Self {
		Self::new_unchecked(0, 1)
	}

	/// Return whether `self` is zero.
	///
	/// Note that any combination of `n == 0` and `d` represents zero.
	pub const fn is_zero(&self) -> bool {
		self.n == 0
	}

	/// Invert `n/d` to `d/n`.
	///
	/// NOTE: Zero inverts to zero.
	pub const fn inverted(self) -> Self {
		if self.is_zero() {
			self
		} else {
			Self { n: self.d, d: self.n }
		}
	}
}

impl From<Ratio> for (u128, u128) {
	fn from(ratio: Ratio) -> (u128, u128) {
		(ratio.n, ratio.d)
	}
}

#[cfg(test)]
impl From<Ratio> for rug::Rational {
	fn from(ratio: Ratio) -> rug::Rational {
		rug::Rational::from((ratio.n, ratio.d))
	}
}

impl From<u128> for Ratio {
	fn from(n: u128) -> Self {
		Self::new(n, 1)
	}
}

impl From<(u128, u128)> for Ratio {
	fn from((n, d): (u128, u128)) -> Self {
		Self::new(n, d)
	}
}

impl PartialOrd for Ratio {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

// Taken from Substrate's `Rational128`.
impl Ord for Ratio {
	fn cmp(&self, other: &Self) -> Ordering {
		if self.d == other.d {
			self.n.cmp(&other.n)
		} else if self.d.is_zero() {
			Ordering::Greater
		} else if other.d.is_zero() {
			Ordering::Less
		} else {
			let self_n = helpers_128bit::to_big_uint(self.n) * helpers_128bit::to_big_uint(other.d);
			let other_n =
				helpers_128bit::to_big_uint(other.n) * helpers_128bit::to_big_uint(self.d);
			self_n.cmp(&other_n)
		}
	}
}

#[cfg(feature = "std")]
impl sp_std::fmt::Debug for Ratio {
	fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
		write!(f, "Ratio({} / {} ≈ {:.8})", self.n, self.d, self.n as f64 / self.d as f64)
	}
}

#[cfg(not(feature = "std"))]
impl sp_std::fmt::Debug for Ratio {
	fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
		write!(f, "Ratio({} / {})", self.n, self.d)
	}
}
