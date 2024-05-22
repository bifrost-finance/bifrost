// This file is an exact copy from the Acala project (https://github.com/AcalaNetwork/Acala/),
// which is licensed under the GNU General Public License.
//
// For more info of its license, please check the Acala project

#[cfg(test)]
mod tests {
	#[test]
	fn generate_function_selector_works() {
		#[module_evm_utility_macro::generate_function_selector]
		#[derive(Debug, Eq, PartialEq)]
		#[repr(u32)]
		pub enum Action {
			Name = "name()",
			Symbol = "symbol()",
			Decimals = "decimals()",
			TotalSupply = "totalSupply()",
			BalanceOf = "balanceOf(address)",
			Transfer = "transfer(address,uint256)",
		}

		assert_eq!(Action::Name as u32, 0x06fdde03_u32);
		assert_eq!(Action::Symbol as u32, 0x95d89b41_u32);
		assert_eq!(Action::Decimals as u32, 0x313ce567_u32);
		assert_eq!(Action::TotalSupply as u32, 0x18160ddd_u32);
		assert_eq!(Action::BalanceOf as u32, 0x70a08231_u32);
		assert_eq!(Action::Transfer as u32, 0xa9059cbb_u32);
	}

	#[test]
	fn keccak256_works() {
		assert_eq!(
			module_evm_utility_macro::keccak256!(""),
			&module_evm_utility::sha3_256("")
		);
		assert_eq!(
			module_evm_utility_macro::keccak256!("keccak256"),
			&module_evm_utility::sha3_256("keccak256")
		);
	}
}
