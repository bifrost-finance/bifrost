pub trait InspectEvmAccounts<AccountId, EvmAddress> {
	/// get the EVM address from the substrate address.
	fn evm_address(account_id: &impl AsRef<[u8; 32]>) -> EvmAddress;

	/// Get the AccountId from the EVM address.
	fn convert_account_id(evm_address: EvmAddress) -> AccountId;

	/// Return the Substrate address bound to the EVM account. If not bound, returns `None`.
	fn bound_account_id(evm_address: EvmAddress) -> Option<AccountId>;

	/// Get the Substrate address from the EVM address.
	/// Returns the converted address if the address wasn't bind.
	fn account_id(evm_address: EvmAddress) -> AccountId;

	/// Returns `True` if the address is allowed to deploy smart contracts.
	fn can_deploy_contracts(evm_address: EvmAddress) -> bool;
}
