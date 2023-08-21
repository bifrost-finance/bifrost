use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::Weight;
use frame_system::Config;
use primitives::{AccountId, Balance, BlockNumber, ParaId};
use scale_info::TypeInfo;
use sp_runtime::{traits::StaticLookup, MultiSignature, RuntimeDebug};
use sp_std::{boxed::Box, vec::Vec};
use xcm::latest::MultiLocation;

/// A destination account for payment.
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum RewardDestination<AccountId> {
	/// Pay into the stash account, increasing the amount at stake accordingly.
	Staked,
	/// Pay into the stash account, not increasing the amount at stake.
	Stash,
	/// Pay into the controller account.
	Controller,
	/// Pay into a specified account.
	Account(AccountId),
	/// Receive no reward.
	None,
}

/// Relaychain staking.bond call arguments
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct StakingBondCall<T: Config> {
	/// Controller account
	pub controller: <T::Lookup as StaticLookup>::Source,
	/// Bond amount
	#[codec(compact)]
	pub value: u128,
	/// A destination account for payment.
	pub payee: RewardDestination<T::AccountId>,
}

/// Relaychain staking.bond_extra call arguments
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct StakingBondExtraCall {
	/// Rebond amount
	#[codec(compact)]
	pub value: u128,
}

/// Relaychain staking.unbond call arguments
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct StakingUnbondCall {
	/// Unbond amount
	#[codec(compact)]
	pub value: u128,
}

/// Relaychain staking.rebond call arguments
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct StakingRebondCall {
	/// Rebond amount
	#[codec(compact)]
	pub value: u128,
}

/// Relaychain staking.withdraw_unbonded call arguments
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct StakingWithdrawUnbondedCall {
	/// Withdraw amount
	pub num_slashing_spans: u32,
}

/// Relaychain staking.nominate call arguments
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct StakingNominateCall<T: Config> {
	/// List of nominate `targets`
	pub targets: Vec<<T::Lookup as StaticLookup>::Source>,
}

/// Relaychain staking.payout_stakers call arguments
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct StakingPayoutStakersCall<T: Config> {
	/// Stash account of validator
	pub validator_stash: T::AccountId,
	/// EraIndex
	pub era: u32,
}

#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum StakingCall<T: Config> {
	#[codec(index = 0)]
	Bond(StakingBondCall<T>),
	#[codec(index = 1)]
	BondExtra(StakingBondExtraCall),
	#[codec(index = 2)]
	Unbond(StakingUnbondCall),
	#[codec(index = 3)]
	WithdrawUnbonded(StakingWithdrawUnbondedCall),
	#[codec(index = 5)]
	Nominate(StakingNominateCall<T>),
	#[codec(index = 18)]
	PayoutStakers(StakingPayoutStakersCall<T>),
	#[codec(index = 19)]
	Rebond(StakingRebondCall),
}

/// Relaychain balances.transfer_keep_alive call arguments
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct BalancesTransferKeepAliveCall<T: Config> {
	/// dest account
	pub dest: <T::Lookup as StaticLookup>::Source,
	/// transfer amount
	#[codec(compact)]
	pub value: u128,
}

/// Relaychain balances.transfer_all call arguments
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct BalancesTransferAllCall<T: Config> {
	/// dest account
	pub dest: <T::Lookup as StaticLookup>::Source,
	pub keep_alive: bool,
}

#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum BalancesCall<T: Config> {
	#[codec(index = 3)]
	TransferKeepAlive(BalancesTransferKeepAliveCall<T>),
	#[codec(index = 4)]
	TransferAll(BalancesTransferAllCall<T>),
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct CrowdloansContributeCall {
	/// - `crowdloan`: The crowdloan who you are contributing to
	#[codec(compact)]
	pub index: ParaId,
	/// - `value`: The amount of tokens you want to contribute to a parachain.
	#[codec(compact)]
	pub value: u128,
	// `signature`: The signature if the fund has a verifier
	pub signature: Option<MultiSignature>,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct CrowdloansWithdrawCall<T: Config> {
	/// - `who`: The account whose contribution should be withdrawn.
	pub who: T::AccountId,
	/// - `index`: The parachain to whose crowdloan the contribution was made.
	#[codec(compact)]
	pub index: ParaId,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct CrowdloansAddMemoCall {
	/// - `index`: The parachain to whose crowdloan the contribution was made.
	pub index: ParaId,
	pub memo: Vec<u8>,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum CrowdloansCall<T: Config> {
	#[codec(index = 1)]
	Contribute(CrowdloansContributeCall),
	#[codec(index = 2)]
	Withdraw(CrowdloansWithdrawCall<T>),
	#[codec(index = 6)]
	AddMemo(CrowdloansAddMemoCall),
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SystemRemarkCall {
	pub remark: Vec<u8>,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum SystemCall {
	#[codec(index = 1)]
	Remark(SystemRemarkCall),
}

#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ProxyProxyCall<RelaychainCall> {
	pub real: AccountId,
	pub force_proxy_type: Option<ProxyType>,
	pub call: RelaychainCall,
}

#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ProxyAddProxyCall {
	pub delegate: AccountId,
	pub proxy_type: Option<ProxyType>,
	pub delay: BlockNumber,
}

#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ProxyRemoveProxyCall {
	pub delegate: AccountId,
	pub proxy_type: Option<ProxyType>,
	pub delay: BlockNumber,
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
	TypeInfo,
)]
pub enum ProxyType {
	Any = 0_isize,
	NonTransfer = 1_isize,
	Governance = 2_isize,
	Staking = 3_isize,
}

impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}

#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum ProxyCall<RelaychainCall> {
	#[codec(index = 0)]
	Proxy(ProxyProxyCall<RelaychainCall>),
	#[codec(index = 1)]
	AddProxy(ProxyAddProxyCall),
	#[codec(index = 2)]
	RemoveProxy(ProxyRemoveProxyCall),
}

/// Relaychain utility.as_derivative call arguments
#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct UtilityAsDerivativeCall<RelaychainCall> {
	/// derivative index
	pub index: u16,
	/// call
	pub call: RelaychainCall,
}

/// Relaychain utility.batch_all call arguments
#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct UtilityBatchAllCall<RelaychainCall> {
	/// calls
	pub calls: Vec<RelaychainCall>,
}

#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum UtilityCall<RelaychainCall> {
	#[codec(index = 1)]
	AsDerivative(UtilityAsDerivativeCall<RelaychainCall>),
	#[codec(index = 2)]
	BatchAll(UtilityBatchAllCall<RelaychainCall>),
}

#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum KusamaCall<T: Config> {
	#[codec(index = 0)]
	System(SystemCall),
	#[codec(index = 4)]
	Balances(BalancesCall<T>),
	#[codec(index = 6)]
	Staking(StakingCall<T>),
	#[codec(index = 22)]
	Proxy(Box<ProxyCall<Self>>),
	#[codec(index = 24)]
	Utility(Box<UtilityCall<Self>>),
	#[codec(index = 73)]
	Crowdloans(CrowdloansCall<T>),
}

#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum PolkadotCall<T: Config> {
	#[codec(index = 0)]
	System(SystemCall),
	#[codec(index = 5)]
	Balances(BalancesCall<T>),
	#[codec(index = 7)]
	Staking(StakingCall<T>),
	#[codec(index = 26)]
	Utility(Box<UtilityCall<Self>>),
	#[codec(index = 29)]
	Proxy(Box<ProxyCall<Self>>),
	#[codec(index = 73)]
	Crowdloans(CrowdloansCall<T>),
}

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct XcmWeightFeeMisc<Weight, Balance> {
	pub weight: Weight,
	pub fee: Balance,
}

impl Default for XcmWeightFeeMisc<Weight, Balance> {
	fn default() -> Self {
		let default_weight = 20_000_000_000u64;
		let default_fee = 15_000_000_000;
		let default_proof_size: u64 = 64 * 1024;
		XcmWeightFeeMisc {
			weight: Weight::from_parts(default_weight, default_proof_size),
			fee: default_fee,
		}
	}
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum XcmCall {
	Bond,
	BondExtra,
	Unbond,
	Rebond,
	WithdrawUnbonded,
	Nominate,
	Contribute,
	Withdraw,
	AddMemo,
	TransferToSiblingchain(Box<MultiLocation>),
	Proxy,
	AddProxy,
	RemoveProxy,
}

#[macro_export]
macro_rules! switch_relay {
    ({ $( $code:tt )* }) => {
        if <T as Config>::RelayNetwork::get() == NetworkId::Polkadot {
            use pallet_traits::ump::PolkadotCall as RelaychainCall;

            $( $code )*
        } else if <T as Config>::RelayNetwork::get() == NetworkId::Kusama {
            use pallet_traits::ump::KusamaCall as RelaychainCall;

            $( $code )*
        } else {
            unreachable!()
        }
    }
}
