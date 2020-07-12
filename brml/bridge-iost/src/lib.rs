// Copyright 2020 Liebi Technologies.
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
#![cfg_attr(not(feature = "std"), no_std)]
#[macro_use]
extern crate alloc;

use alloc::string::{String, ToString};
use core::{convert::TryFrom, fmt::Debug, ops::Div, str::FromStr};

use codec::{Decode, Encode};

use frame_support::traits::Get;
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
    weights::{DispatchClass, FunctionOf, Pays, Weight},
    Parameter,
};
use frame_system::{
    self as system, ensure_none, ensure_root, ensure_signed,
    offchain::{CreateSignedTransaction, SubmitTransaction},
};
use sp_core::offchain::StorageKind;
use sp_runtime::{
    traits::{AtLeast32Bit, Member, SaturatedConversion},
    transaction_validity::{
        InvalidTransaction, TransactionLongevity, TransactionPriority, TransactionSource,
        TransactionValidity, ValidTransaction,
    },
};
use sp_std::prelude::*;

use node_primitives::{
    AssetSymbol, AssetTrait, BlockchainType, BridgeAssetBalance, BridgeAssetFrom,
    BridgeAssetSymbol, BridgeAssetTo, TokenType,
};
use sp_application_crypto::RuntimeAppPublic;
use transaction::TxOut;

mod transaction;

// lazy_static::lazy_static! {
// 	pub static ref ACTION_NAMES: [ActionName; 1] = {
// 		let name = ActionName::from_str("transfer").unwrap();
// 		[name]
// 	};
// }

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, Debug)]
enum TransactionType {
    Deposit,
    Withdraw,
}

pub mod sr25519 {
    pub mod app_sr25519 {
        use sp_application_crypto::{app_crypto, key_types::ACCOUNT, sr25519};
        app_crypto!(sr25519, ACCOUNT);

        impl From<sp_runtime::AccountId32> for Public {
            fn from(acct: sp_runtime::AccountId32) -> Self {
                let mut data = [0u8; 32];
                let acct_data: &[u8; 32] = acct.as_ref();
                for (index, val) in acct_data.iter().enumerate() {
                    data[index] = *val;
                }
                Self(sp_core::sr25519::Public(data))
            }
        }
    }

    /// A bridge-iost keypair using sr25519 as its crypto.
    #[cfg(feature = "std")]
    pub type AuthorityPair = app_sr25519::Pair;

    /// A bridge-iost signature using sr25519 as its crypto.
    pub type AuthoritySignature = app_sr25519::Signature;

    /// A bridge-iost identifier using sr25519 as its crypto.
    pub type AuthorityId = app_sr25519::Public;
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Use has no privilege to execute cross transaction
        NoCrossChainPrivilege,
        /// EOS node address or private key is not set
        NoLocalStorage,
        /// The format of memo doesn't match the requirement
        InvalidMemo,
        /// This is an invalid hash value while hashing schedule
        InvalidScheduleHash,
        /// Sign a transaction more than twice
        AlreadySignedByAuthor,
        /// Fail to calculate merkle root tree
        CalculateMerkleError,
        /// Fail to get EOS block id from block header
        FailureOnGetBlockId,
        /// Invalid substrate account id
        InvalidAccountId,
        /// Fail to cast balance type
        ConvertBalanceError,
        /// Fail to append block id to merkel tree
        AppendIncreMerkleError,
        /// Fail to verify signature
        SignatureVerificationFailure,
        /// Fail to verify merkle tree
        MerkleRootVerificationFailure,
        /// The length of block headers don't meet the size 15
        InvalidBlockHeadersLength,
        /// Invalid transaction
        InvalidTxOutType,
        /// Error from eos-chain crate
        IostChainError,
        /// Error from eos-key crate
        IostKeysError,
        /// User hasn't enough balance to trade
        InsufficientBalance,
        /// Fail to parse utf8 array
        ParseUtf8Error,
        /// Error while decode hex text
        DecodeHexError,
        /// Fail to parse secret key
        ParseSecretKeyError,
        /// Fail to calcualte action hash
        ErrorOnCalculationActionHash,
        /// Fail to calcualte action receipt hash
        ErrorOnCalculationActionReceiptHash,
        /// Offchain http error
        OffchainHttpError,
        /// EOS node response a error after send a request
        IOSTRpcError,
        /// Error from lite-json while serializing or deserializing
        LiteJsonError,
        /// Invalid checksum
        InvalidChecksum256,
        /// Initialze producer schedule multiple times
        InitMultiTimeProducerSchedules,
    }
}

pub type VersionId = u32;

pub trait Trait: CreateSignedTransaction<Call<Self>> + pallet_authorship::Trait {
    /// The identifier type for an authority.
    type AuthorityId: Member
        + Parameter
        + RuntimeAppPublic
        + Default
        + Ord
        + From<<Self as frame_system::Trait>::AccountId>;

    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// The units in which we record balances.
    type Balance: Member + Parameter + AtLeast32Bit + Default + Copy;

    /// The arithmetic type of asset identifier.
    type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy;

    /// The units in which we record costs.
    type Cost: Member + Parameter + AtLeast32Bit + Default + Copy;

    /// The units in which we record incomes.
    type Income: Member + Parameter + AtLeast32Bit + Default + Copy;

    /// The units in which we record asset precision.
    type Precision: Member + Parameter + AtLeast32Bit + Default + Copy;

    /// Bridge asset from another blockchain.
    type BridgeAssetFrom: BridgeAssetFrom<Self::AccountId, Self::Precision, Self::Balance>;

    type AssetTrait: AssetTrait<
        Self::AssetId,
        Self::AccountId,
        Self::Balance,
        Self::Cost,
        Self::Income,
    >;

    /// A dispatchable call type.
    type Call: From<Call<Self>>;
}

decl_event! {
    pub enum Event<T>
        where <T as system::Trait>::AccountId,
    {
        InitSchedule(VersionId),
        ChangeSchedule(VersionId, VersionId), // ChangeSchedule(older, newer)
        ProveAction,
        RelayBlock,
        Deposit(Vec<u8>, AccountId), // IOST account => Bifrost AccountId
        DepositFail,
        Withdraw(AccountId, Vec<u8>), // Bifrost AccountId => IOST account
        WithdrawFail,
        SendTransactionSuccess,
        SendTransactionFailure,
        GrantedCrossChainPrivilege(AccountId),
        RemovedCrossChainPrivilege(AccountId),
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as BridgeIost {
        /// The current set of notary keys that may send bridge transactions to Eos chain.
        NotaryKeys get(fn notary_keys) config(): Vec<T::AccountId>;

        /// Config to enable/disable this runtime
        BridgeEnable get(fn is_bridge_enable): bool = true;

        /// Eos producer list and hash which in specific version id
        // ProducerSchedules: map hasher(blake2_128_concat) VersionId => (Vec<ProducerAuthority>, Checksum256);

        /// Initialize a producer schedule while starting a node.
        // InitializeSchedule get(fn producer_schedule): ProducerAuthoritySchedule;

        /// Save all unique transactions
        /// Every transaction has different action receipt, but can have the same action
        // BridgeActionReceipt: map hasher(blake2_128_concat) ActionReceipt => Action;

        /// Current pending schedule version
        PendingScheduleVersion: VersionId;

        /// Transaction sent to Eos blockchain
        BridgeTxOuts get(fn bridge_tx_outs): Vec<TxOut<T::AccountId>>;

        /// Account where Eos bridge contract deployed, (Account, Signature threshold)
        BridgeContractAccount get(fn bridge_contract_account) config(): (Vec<u8>, u8);

        /// Who has the privilege to call transaction between Bifrost and EOS
        CrossChainPrivilege get(fn cross_chain_privilege) config(): map hasher(blake2_128_concat) T::AccountId => bool;
        /// How many address has the privilege sign transaction between EOS and Bifrost
        AllAddressesHaveCrossChainPrivilege get(fn all_crosschain_privilege) config(): Vec<T::AccountId>;
    }
    add_extra_genesis {
        build(|config: &GenesisConfig<T>| {
            BridgeContractAccount::put(config.bridge_contract_account.clone());

            NotaryKeys::<T>::put(config.notary_keys.clone());
            //
            // let schedule = ProducerAuthoritySchedule::default();
            // let schedule_hash = schedule.schedule_hash();
            // assert!(schedule_hash.is_ok());
            // ProducerSchedules::insert(schedule.version, (schedule.producers, schedule_hash.unwrap()));
            // PendingScheduleVersion::put(schedule.version);
            //
            // // grant privilege to sign transaction between EOS and Bifrost
            // for (who, privilege) in config.cross_chain_privilege.iter() {
            // 	<CrossChainPrivilege<T>>::insert(who, privilege);
            // }
            // update to AllAddressesHaveCrossChainPrivilege
            let all_addresses: Vec<T::AccountId> = config.cross_chain_privilege.iter().map(|x| x.0.clone()).collect();
            <AllAddressesHaveCrossChainPrivilege<T>>::mutate(move |all| {
                all.extend(all_addresses.into_iter());
            });
        });
    }

}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = T::DbWeight::get().writes(1)]
        fn bridge_enable(origin, enable: bool) {
            ensure_root(origin)?;

            BridgeEnable::put(enable);
        }


    }
}

impl<T: Trait> Module<T> {}

impl<T: Trait> BridgeAssetTo<T::AccountId, T::Precision, T::Balance> for Module<T> {
    type Error = crate::Error<T>;
    fn bridge_asset_to(
        target: Vec<u8>,
        bridge_asset: BridgeAssetBalance<T::AccountId, T::Precision, T::Balance>,
    ) -> Result<(), Self::Error> {
        // let _ = Self::tx_transfer_to(target, bridge_asset)?;

        Ok(())
    }

    fn redeem(_: AssetSymbol, _: T::Balance, _: Vec<u8>) -> Result<(), Self::Error> {
        Ok(())
    }
    fn stake(_: AssetSymbol, _: T::Balance, _: Vec<u8>) -> Result<(), Self::Error> {
        Ok(())
    }
    fn unstake(_: AssetSymbol, _: T::Balance, _: Vec<u8>) -> Result<(), Self::Error> {
        Ok(())
    }
}
