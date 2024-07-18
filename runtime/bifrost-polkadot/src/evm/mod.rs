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

// use crate::evm::runner::WrapRunner;
pub use crate::evm::accounts_conversion::{ExtendedAddressMapping, FindAuthorTruncated};
use crate::{
	evm::runner::WrapRunner, governance::TechAdminOrCouncil, Aura, ConstU32, DynamicFee,
	EVMChainId, Runtime, RuntimeEvent, Timestamp, Weight, EVM, MAXIMUM_BLOCK_WEIGHT,
	NORMAL_DISPATCH_RATIO, WEIGHT_REF_TIME_PER_SECOND,
};
use bifrost_flexible_fee::FeeAssetBalanceInCurrency;
use bifrost_primitives::{CurrencyId, CurrencyId::Token2};
use frame_support::{pallet_prelude::Get, parameter_types, traits::FindAuthor, ConsensusEngineId};
use orml_tokens::CurrencyAdapter;
use pallet_ethereum::PostLogContent;
use pallet_evm::EnsureAddressTruncated;
use pallet_transaction_payment::Multiplier;
use primitive_types::U256;

mod accounts_conversion;
mod evm_fee;
pub mod precompiles;
mod runner;

// Current approximation of the gas per second consumption considering
// EVM execution over compiled WASM (on 4.4Ghz CPU).
// Given the 500ms Weight, from which 75% only are used for transactions,
// the total EVM execution gas limit is: GAS_PER_SECOND * 0.500 * 0.75 ~=
// 15_000_000.
pub const GAS_PER_SECOND: u64 = 40_000_000;
// Approximate ratio of the amount of Weight per Gas.
const WEIGHT_PER_GAS: u64 = WEIGHT_REF_TIME_PER_SECOND / GAS_PER_SECOND;

// Fixed gas price of 0.015 gwei per gas
pub const DEFAULT_BASE_FEE_PER_GAS: u128 = 15_000_000;

parameter_types! {
	// We allow for a 75% fullness of a 0.5s block
	pub BlockGasLimit: U256 = U256::from(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT.ref_time() /
WEIGHT_PER_GAS);
	pub PrecompilesValue: precompiles::BifrostPrecompiles<crate::Runtime> =
precompiles::BifrostPrecompiles::<_>::new();
	pub WeightPerGas: Weight = Weight::from_parts(WEIGHT_PER_GAS, 0);
}

pub struct WethAssetId;
impl Get<CurrencyId> for WethAssetId {
	fn get() -> CurrencyId {
		Token2(0)
	}
}

type WethCurrency = CurrencyAdapter<Runtime, WethAssetId>;

pub struct TransactionPaymentMultiplier;

impl Get<Multiplier> for TransactionPaymentMultiplier {
	fn get() -> Multiplier {
		crate::TransactionPayment::next_fee_multiplier()
	}
}

parameter_types! {
	/// The amount of gas per pov. A ratio of 4 if we convert ref_time to gas and we compare
	/// it with the pov_size for a block. E.g.
	/// ceil(
	///     (max_extrinsic.ref_time() / max_extrinsic.proof_size()) / WEIGHT_PER_GAS
	/// )
	pub const GasLimitPovSizeRatio: u64 = 4;
	/// The amount of gas per storage (in bytes): BLOCK_GAS_LIMIT / BLOCK_STORAGE_LIMIT
	/// The current definition of BLOCK_STORAGE_LIMIT is 40 KB, resulting in a value of 366.
	pub GasLimitStorageGrowthRatio: u64 = 366;

	pub const SuicideQuickClearLimit: u32 = 0;
}

impl pallet_evm::Config for Runtime {
	type FeeCalculator = DynamicFee;
	type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
	type WeightPerGas = WeightPerGas;
	type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
	type CallOrigin = EnsureAddressTruncated;
	type WithdrawOrigin = EnsureAddressTruncated;
	type AddressMapping = ExtendedAddressMapping;
	type Currency = WethCurrency;
	type RuntimeEvent = RuntimeEvent;
	type PrecompilesType = precompiles::BifrostPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = EVMChainId;
	type BlockGasLimit = BlockGasLimit;
	type Runner = WrapRunner<
		Self,
		pallet_evm::runner::stack::Runner<Self>, // Evm runner that we wrap
		FeeAssetBalanceInCurrency<
			crate::Runtime,
			crate::FlexibleFee, // Get account's fee payment asset
			crate::Currencies,  // Account balance inspector
		>,
	>;
	type OnChargeTransaction = evm_fee::TransferEvmFees<
		evm_fee::DepositEvmFeeToTreasury,
		crate::FlexibleFee, // Get account's fee payment asset
		WethAssetId,
		crate::Currencies, // Multi currency support
	>;
	type OnCreate = ();
	type FindAuthor = FindAuthorTruncated<Aura>;
	type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
	type SuicideQuickClearLimit = SuicideQuickClearLimit;
	type Timestamp = Timestamp;
	type WeightInfo = pallet_evm::weights::SubstrateWeight<Self>;
}

impl pallet_evm_chain_id::Config for Runtime {}

parameter_types! {
	pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
	type PostLogContent = PostBlockAndTxnHashes;
	type ExtraDataLength = ConstU32<30>;
}

pub struct EvmNonceProvider;
impl pallet_evm_accounts::EvmNonceProvider for EvmNonceProvider {
	fn get_nonce(evm_address: sp_core::H160) -> U256 {
		EVM::account_basic(&evm_address).0.nonce
	}
}

impl pallet_evm_accounts::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type FeeMultiplier = ConstU32<50>;
	type EvmNonceProvider = EvmNonceProvider;
	type ControllerOrigin = TechAdminOrCouncil;
	type WeightInfo = ();
}

parameter_types! {
	pub BoundDivision: U256 = U256::from(1024);
}

impl pallet_dynamic_fee::Config for Runtime {
	type MinGasPriceBoundDivisor = BoundDivision;
}

parameter_types! {
	pub const DefaultBaseFeePerGas: u128 = DEFAULT_BASE_FEE_PER_GAS;
	pub const MinBaseFeePerGas: u128 = DEFAULT_BASE_FEE_PER_GAS.saturating_div(10);
	pub const MaxBaseFeePerGas: u128 = 14415000000; //To reach 10 dollar per omnipool trade
}

// impl pallet_dynamic_evm_fee::Config for crate::Runtime {
// 	type AssetId = AssetId;
// 	type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
// 	type MinBaseFeePerGas = MinBaseFeePerGas;
// 	type MaxBaseFeePerGas = MaxBaseFeePerGas;
// 	type FeeMultiplier = TransactionPaymentMultiplier;
// 	type NativePriceOracle = AssetFeeOraclePriceProvider<
// 		NativeAssetId,
// 		crate::MultiTransactionPayment,
// 		crate::Router,
// 		OraclePriceProvider<AssetId, crate::EmaOracle, LRNA>,
// 		crate::MultiTransactionPayment,
// 		OracleEvmPeriod,
// 	>;
// 	type WethAssetId = WethAssetId;
// 	type WeightInfo = crate::weights::dynamic_evm_fee::HydraWeight<crate::Runtime>;
// }
