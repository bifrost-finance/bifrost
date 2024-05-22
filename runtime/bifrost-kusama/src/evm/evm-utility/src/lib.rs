// This file is an exact copy from the Acala project (https://github.com/AcalaNetwork/Acala/),
// which is licensed under the GNU General Public License.
//
// For more info of its license, please check the Acala project

//! # Evm utiltity Module
//!
//! A pallet provides some utility methods.

#![cfg_attr(not(feature = "std"), no_std)]

use sha3::{Digest, Keccak256};

pub use ethereum;
pub use evm::{self, backend::Basic as Account};
pub use evm_gasometer;
pub use evm_runtime;

pub fn sha3_256(s: &str) -> [u8; 32] {
	let mut result = [0u8; 32];

	// create a SHA3-256 object
	let mut hasher = Keccak256::new();
	// write input message
	hasher.update(s);
	// read hash digest
	result.copy_from_slice(&hasher.finalize()[..32]);

	result
}

pub fn get_function_selector(s: &str) -> u32 {
	let result = sha3_256(s);
	u32::from_be_bytes(result[..4].try_into().unwrap())
}
