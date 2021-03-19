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

#![cfg_attr(not(feature = "std"), no_std)]
#[macro_use]
extern crate alloc;

use crate::transaction::IostTxOut;
use alloc::string::{String, ToString};
use codec::{Decode, Encode};
use core::{convert::TryFrom, fmt::Debug, iter::FromIterator, ops::Div, str::FromStr};
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::Get,
    weights::{DispatchClass, Pays, Weight},
    IterableStorageMap, Parameter, StorageValue,
};
use frame_system::{
    self as system, ensure_none, ensure_root, ensure_signed,
    offchain::{SendTransactionTypes, SubmitTransaction},
};
use iost_chain::{ActionTransfer, IostAction, Read};
use lite_json::{parse_json, JsonValue, Serialize};
use node_primitives::{
    AssetTrait, BlockchainType, BridgeAssetBalance, BridgeAssetFrom, BridgeAssetSymbol,
    BridgeAssetTo, FetchVtokenMintPool,
};
use sp_application_crypto::RuntimeAppPublic;
use sp_core::offchain::StorageKind;
use sp_runtime::print;
use sp_runtime::{
    traits::{
        AtLeast32Bit, MaybeSerializeDeserialize, Member, SaturatedConversion, Saturating, Zero,
    },
    transaction_validity::{
        InvalidTransaction, TransactionLongevity, TransactionPriority, TransactionSource,
        TransactionValidity, ValidTransaction,
    },
};
use sp_std::prelude::*;

mod transaction;

pub trait WeightInfo {
    fn bridge_enable() -> Weight;
    fn set_contract_accounts() -> Weight;
    fn grant_crosschain_privilege() -> Weight;
    fn remove_crosschain_privilege() -> Weight;
    fn prove_action() -> Weight;
    fn bridge_tx_report() -> Weight;
    fn cross_to_iost(weight: Weight) -> Weight;
}

impl WeightInfo for () {
    fn bridge_enable() -> Weight {
        Default::default()
    }
    fn set_contract_accounts() -> Weight {
        Default::default()
    }
    fn grant_crosschain_privilege() -> Weight {
        Default::default()
    }
    fn remove_crosschain_privilege() -> Weight {
        Default::default()
    }
    fn prove_action() -> Weight {
        Default::default()
    }
    fn bridge_tx_report() -> Weight {
        Default::default()
    }
    fn cross_to_iost(_: Weight) -> Weight {
        Default::default()
    }
}

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, Debug)]
enum TransactionType {
    Deposit,
    Withdraw,
}

pub type VersionId = u32;

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

const IOST_NODE_URL: &[u8] = b"IOST_NODE_URL";
const IOST_ACCOUNT_NAME: &[u8] = b"IOST_ACCOUNT_NAME";
const IOST_SECRET_KEY: &[u8] = b"IOST_SECRET_KEY";

decl_error! {
    pub enum Error for Module<T: Config> {
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
        /// Error from iost-chain crate
        IostChainError,
        /// Error from iost-key crate
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
        /// IOST or vIOST not existed
        TokenNotExist,
        /// Invalid token
        InvalidTokenForTrade,
        /// IOSTSymbolMismatch,
        IOSTSymbolMismatch,
        /// Bridge eos has been disabled
        CrossChainDisabled,
        /// Who hasn't the permission to sign a cross-chain trade
        NoPermissionSignCrossChainTrade,

        /// Cross transaction back enable or not
        CrossChainBackDisabled,
        // DebugReachable,
        // DebugReachable_1,
        DebugReachableOther,
    }
}

pub trait Config: SendTransactionTypes<Call<Self>> + pallet_authorship::Config {
    /// The identifier type for an authority.
    type AuthorityId: Member
        + Parameter
        + RuntimeAppPublic
        + Default
        + Ord
        + From<<Self as frame_system::Config>::AccountId>;

    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;

    /// The units in which we record balances.
    type Balance: Member
        + Parameter
        + AtLeast32Bit
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + core::fmt::Display;

    /// The arithmetic type of asset identifier.
    type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

    /// The units in which we record asset precision.
    type Precision: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

    /// Bridge asset from another blockchain.
    type BridgeAssetFrom: BridgeAssetFrom<
        Self::AccountId,
        Self::AssetId,
        Self::Precision,
        Self::Balance,
    >;

    type AssetTrait: AssetTrait<Self::AssetId, Self::AccountId, Self::Balance>;

    /// Fetch convert pool from convert module
    type FetchVtokenMintPool: FetchVtokenMintPool<Self::AssetId, Self::Balance>;

    /// A dispatchable call type.
    type Call: From<Call<Self>>;

    /// Set default weight
    type WeightInfo: WeightInfo;
}

decl_event! {
    pub enum Event<T>
        where <T as system::Config>::AccountId,
    {
        InitSchedule(VersionId),
        ChangeSchedule(VersionId, VersionId), // ChangeSchedule(older, newer)
        ProveAction,
        RelayBlock,
        Deposit(Vec<u8>, AccountId), // IOST account => Bifrost AccountId
        DepositFail,
        Withdraw(AccountId, Vec<u8>), // Bifrost AccountId => IOST account
        WithdrawFail,
        SentCrossChainTransaction,
        FailToSendCrossChainTransaction,
        SendTransactionSuccess,
        SendTransactionFailure,
        GrantedCrossChainPrivilege(AccountId),
        RemovedCrossChainPrivilege(AccountId),
        UnsignedTrx,
    }
}

decl_storage! {
    trait Store for Module<T: Config> as BridgeIost {
        /// The current set of notary keys that may send bridge transactions to Iost chain.
        NotaryKeys get(fn notary_keys) config(): Vec<T::AccountId>;

        /// Config to enable/disable this runtime
        BridgeEnable get(fn is_bridge_enable): bool = true;

        /// Cross transaction back enable or not
        CrossChainBackEnable get(fn is_cross_back_enable): bool = true;

        /// Current pending schedule version
        PendingScheduleVersion: VersionId;

        /// Transaction sent to Eos blockchain
        BridgeTxOuts get(fn bridge_tx_outs): Vec<IostTxOut<T::AccountId, T::AssetId>>;

        /// Account where Eos bridge contract deployed, (Account, Signature threshold)
        BridgeContractAccount get(fn bridge_contract_account) config(): (Vec<u8>, u8);

        /// Who has the privilege to call transaction between Bifrost and EOS
        CrossChainPrivilege get(fn cross_chain_privilege) config(): map hasher(blake2_128_concat) T::AccountId => bool;
        /// How many address has the privilege sign transaction between EOS and Bifrost
        AllAddressesHaveCrossChainPrivilege get(fn all_crosschain_privilege) config(): Vec<T::AccountId>;

        /// Set IOST asset id
        IostAssetId get(fn iost_asset_id) config(): T::AssetId;
    }
    add_extra_genesis {
        build(|config: &GenesisConfig<T>| {
            BridgeContractAccount::put(config.bridge_contract_account.clone());

            NotaryKeys::<T>::put(config.notary_keys.clone());

            // grant privilege to sign transaction between IOST and Bifrost
            for (who, privilege) in config.cross_chain_privilege.iter() {
                <CrossChainPrivilege<T>>::insert(who, privilege);
            }
            // update to AllAddressesHaveCrossChainPrivilege
            let all_addresses: Vec<T::AccountId> = config.cross_chain_privilege.iter().map(|x| x.0.clone()).collect();
            <AllAddressesHaveCrossChainPrivilege<T>>::mutate(move |all| {
                all.extend(all_addresses.into_iter());
            });
            // set IOST asset id
            IostAssetId::<T>::put(config.iost_asset_id);
        });
    }

}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = T::WeightInfo::bridge_enable()]
        fn bridge_enable(origin, enable: bool) {
            ensure_root(origin)?;

            BridgeEnable::put(enable);
        }

        #[weight = T::DbWeight::get().writes(1)]
        fn cross_chain_back_enable(origin, enable: bool) {
            ensure_root(origin)?;

            CrossChainBackEnable::put(enable);
        }

        #[weight = (T::WeightInfo::set_contract_accounts(), DispatchClass::Normal, Pays::No)]
        fn set_contract_accounts(origin, account: Vec<u8>, threthold: u8) {
            ensure_root(origin)?;
            BridgeContractAccount::put((account, threthold));
        }

        #[weight = T::WeightInfo::grant_crosschain_privilege()]
        fn grant_crosschain_privilege(origin, target: T::AccountId) {
            ensure_root(origin)?;

            // grant privilege
            <CrossChainPrivilege<T>>::insert(&target, true);

            // update all addresses
            <AllAddressesHaveCrossChainPrivilege<T>>::mutate(|all| {
                if !all.contains(&target) {
                    all.push(target.clone());
                }
            });

            Self::deposit_event(RawEvent::GrantedCrossChainPrivilege(target));
        }

        #[weight = (T::WeightInfo::remove_crosschain_privilege(), DispatchClass::Normal, Pays::No)]
        fn remove_crosschain_privilege(origin, target: T::AccountId) {
            ensure_root(origin)?;

            // remove privilege
            <CrossChainPrivilege<T>>::insert(&target, false);

            // update all addresses
            <AllAddressesHaveCrossChainPrivilege<T>>::mutate(|all| {
                all.retain(|who| who.ne(&target));
            });

            Self::deposit_event(RawEvent::RemovedCrossChainPrivilege(target));
        }

        #[weight = (T::WeightInfo::prove_action(), DispatchClass::Normal, Pays::No)]
        fn prove_action(
            origin,
            action: IostAction,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;
            ensure!(CrossChainPrivilege::<T>::get(&origin), Error::<T>::NoPermissionSignCrossChainTrade);

            // ensure this transaction is unique, and ensure no duplicated transaction
            // ensure!(BridgeActionReceipt::get(&action_receipt).ne(&action), Error::<T>::DuplicatedCrossChainTransaction);

            // ensure action is what we want
            // ensure!(action.action_name == "transfer", "This is an invalid action to Bifrost");

            Self::deposit_event(RawEvent::ProveAction);

            let action_transfer = Self::get_action_transfer_from_action(&action)?;
            let cross_account = BridgeContractAccount::get().0;
            // withdraw operation, Bifrost => IOST
            if cross_account == action_transfer.from.to_string().into_bytes() {
                match Self::transaction_from_bifrost_to_iost(&action_transfer) {
                    Ok(target) => {
                        Self::deposit_event(RawEvent::Withdraw(target, action_transfer.to.to_string().into_bytes()));
                    }
                    Err(e) => {
                        log::info!("Bifrost => IOST failed due to {:?}", e);
                        Self::deposit_event(RawEvent::WithdrawFail);
                    }
                }
            }

            // deposit operation, EOS => Bifrost
            if cross_account == action_transfer.to.to_string().into_bytes() {
                match Self::transaction_from_iost_to_bifrost(&action_transfer) {
                    Ok(target) => {
                        Self::deposit_event(RawEvent::Deposit(action_transfer.from.to_string().into_bytes(), target));
                    }
                    Err(e) => {
                        log::info!("IOST => Bifrost failed due to {:?}", e);
                        Self::deposit_event(RawEvent::DepositFail);
                    }
                }
            }

            Ok(())
        }

        #[weight = (T::WeightInfo::bridge_tx_report(), DispatchClass::Normal, Pays::No)]
        fn bridge_tx_report(origin, tx_list: Vec<IostTxOut<T::AccountId, T::AssetId>>) -> DispatchResult {
            ensure_none(origin)?;

            BridgeTxOuts::<T>::put(tx_list);

            Ok(())
        }

        #[weight = (T::WeightInfo::cross_to_iost(memo.len() as Weight), DispatchClass::Normal)]
        fn cross_to_iost(
            origin,
            to: Vec<u8>,
            #[compact] amount: T::Balance,
            memo: Vec<u8>
        ) {
            let origin = system::ensure_signed(origin)?;

            ensure!(CrossChainBackEnable::get(), Error::<T>::CrossChainBackDisabled);

            let asset_id = Self::iost_asset_id();
            let token = T::AssetTrait::get_token(Self::iost_asset_id());
            let symbol_code = token.symbol;
            let symbol_precise = token.precision;

            //
            let balance = T::AssetTrait::get_account_asset(asset_id, &origin).balance;

            let asset_symbol = BridgeAssetSymbol::new(BlockchainType::IOST, symbol_code, T::Precision::from(symbol_precise as u32));
            let bridge_asset = BridgeAssetBalance {
                symbol: asset_symbol,
                amount: amount,
                memo,
                from: origin.clone(),
                asset_id
            };

            match Self::bridge_asset_to(to, bridge_asset) {
                Ok(_) => {
                   log::info!(target: "bridge-iost", "sent transaction to IOST node.");
                    // locked balance until trade is verified
                    T::AssetTrait::lock_asset(&origin, asset_id, amount);
                   Self::deposit_event(RawEvent::SendTransactionSuccess);
                }
                Err(e) => {
                    log::warn!(target: "bridge-iost", "failed to send transaction to IOST node.");
                    Self::deposit_event(RawEvent::SendTransactionFailure);
                }
            }
        }

        // Runs after every block.
        fn offchain_worker(now_block: T::BlockNumber) {
            log::RuntimeLogger::init();
            log::info!(target: "bridge-iost", "A offchain worker processing.");


            if now_block % T::BlockNumber::from(10u32) == T::BlockNumber::from(2u32) {
                match Self::offchain(now_block) {
                    Ok(_) => log::info!(target: "bridge-iost", "A offchain worker started."),
                    Err(e) => log::error!(target: "bridge-iost", "A offchain worker got error: {:?}", e),
                }
            }
            // It's no nessesary to start offchain worker if no any task in queue
            // if !BridgeTxOuts::<T>::get().is_empty() {
            //     // Only send messages if we are a potential validator.
            //     if sp_io::offchain::is_validator() {
            //         log::info!(target: "bridge-iost", "Is validator at {:?}.", now_block);
            //         match Self::offchain(now_block) {
            //             Ok(_) => log::info!("A offchain worker started."),
            //             Err(e) => log::error!("A offchain worker got error: {:?}", e),
            //         }
            //     } else {
            //         log::info!(target: "bridge-iost", "Skipping send tx at {:?}. Not a validator.",now_block)
            //     }
            // } else {
            //     log::info!(target: "bridge-iost", "There's no offchain worker started.");
            // }
        }
    }
}

impl<T: Config> Module<T> {
    fn get_action_transfer_from_action(act: &IostAction) -> Result<ActionTransfer, Error<T>> {
        let data = core::str::from_utf8(&act.data).map_err(|_| Error::<T>::ParseUtf8Error)?;

        let mut action_transfer: ActionTransfer = Default::default();
        let node_info = parse_json(data).map_err(|_| Error::<T>::LiteJsonError)?;

        match node_info {
            JsonValue::Array(ref json) => {
                for (i, item) in json.iter().enumerate() {
                    match item {
                        JsonValue::String(ref chars) => {
                            let v = String::from_iter(chars.iter());
                            match i {
                                0 => action_transfer.token_type = v,
                                1 => action_transfer.from = v,
                                2 => action_transfer.to = v,
                                3 => action_transfer.amount = v,
                                4 => action_transfer.memo = v,
                                _ => (),
                            }
                        }
                        _ => return Err(Error::<T>::IOSTRpcError),
                    }
                }
            }
            _ => return Err(Error::<T>::IOSTRpcError),
        }
        Ok(action_transfer)
    }

    fn transaction_from_iost_to_bifrost(
        action_transfer: &ActionTransfer,
    ) -> Result<T::AccountId, Error<T>> {
        // check memo, example like "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY@bifrost:IOST", the formatter: {receiver}@{chain}:{token_symbol}
        let split_memo = action_transfer
            .memo
            .as_str()
            .split(|c| c == '@' || c == ':')
            .collect::<Vec<_>>();

        // the length should be 2, either 3.
        if split_memo.len().gt(&3) || split_memo.len().lt(&2) {
            return Err(Error::<T>::InvalidMemo);
        }

        // get account
        let account_data = Self::get_account_data(split_memo[0])?;
        let target = Self::into_account(account_data)?;

        let iost_id = Self::iost_asset_id();
        let v_iost_id = T::AssetTrait::get_pair(iost_id).ok_or(Error::<T>::TokenNotExist)?;

        let token_symbol = {
            match split_memo.len() {
                2 => v_iost_id,
                3 => match split_memo[2] {
                    "" | "vIOST" => v_iost_id,
                    "IOST" => iost_id,
                    _ => {
                        log::error!("A invalid token type, default token type will be vtoken");
                        return Err(Error::<T>::InvalidMemo);
                    }
                },
                _ => unreachable!("previous step checked he length of split_memo."),
            }
        };
        // todo, vIOST or IOST, all asset will be added to IOST asset, instead of vIOST or IOST
        // but in the future, we will support both token, let user to which token he wants to get
        // according to the convert price
        let token = T::AssetTrait::get_token(Self::iost_asset_id());
        let symbol_code = token.symbol;
        let symbol_precise = token.precision;
        let align_precision = 12 - symbol_precise;

        let transfer_amount = action_transfer
            .amount
            .parse::<f64>()
            .map_err(|_| Error::<T>::ConvertBalanceError)?;

        let token_balances =
            (transfer_amount * 10u128.pow(8) as f64) as u128 * 10u128.pow(align_precision as u32);

        let new_balance: T::Balance = TryFrom::<u128>::try_from(token_balances)
            .map_err(|_| Error::<T>::ConvertBalanceError)?;

        // todo, according convert price to save as vIOST
        // let vtoken_balances: T::Balance =
        //     TryFrom::<u128>::try_from(11).map_err(|_| Error::<T>::ConvertBalanceError)?;

        T::AssetTrait::asset_issue(iost_id, &target, new_balance);

        Ok(target)
    }

    fn transaction_from_bifrost_to_iost(
        action_transfer: &ActionTransfer,
    ) -> Result<T::AccountId, Error<T>> {
        let bridge_tx_outs = BridgeTxOuts::<T>::get();
        let pending_tx_id = "".to_string();

        for trx in bridge_tx_outs.iter() {
            match trx {
                IostTxOut::Processing {
                    tx_id,
                    multi_sig_tx,
                } => {
                    let tx_id = String::from_utf8(tx_id.to_vec())
                        .map_err(|_| Error::<T>::ConvertBalanceError)?;
                    // if pending_tx_id.ne(tx_id.as_str()) {
                    //     continue;
                    // }
                    let target = &multi_sig_tx.from;
                    let token_symbol = multi_sig_tx.token_type;
                    let asset_id = Self::iost_asset_id();
                    let all_vtoken_balances =
                        T::AssetTrait::get_account_asset(asset_id, &target).balance;

                    // let amount = action_transfer.amount.clone();
                    let token_balances: u128 = action_transfer
                        .amount
                        .parse::<u128>()
                        .map_err(|_| Error::<T>::ConvertBalanceError)?;
                    let vtoken_balances = TryFrom::<u128>::try_from(token_balances)
                        .map_err(|_| Error::<T>::ConvertBalanceError)?;

                    if all_vtoken_balances.lt(&vtoken_balances) {
                        log::warn!("origin account balance must be greater than or equal to the transfer amount.");
                        return Err(Error::<T>::InsufficientBalance);
                    }

                    // the trade is verified, unlock asset
                    T::AssetTrait::unlock_asset(&target, asset_id, vtoken_balances);

                    return Ok(target.clone());
                }
                _ => continue,
            }
        }

        Err(Error::<T>::InvalidAccountId)
    }

    /// check receiver account format
    /// https://github.com/paritytech/substrate/wiki/External-Address-Format-(SS58)
    fn get_account_data(receiver: &str) -> Result<[u8; 32], Error<T>> {
        let decoded_ss58 = bs58::decode(receiver)
            .into_vec()
            .map_err(|_| Error::<T>::InvalidAccountId)?;

        // todo, decoded_ss58.first() == Some(&42) || Some(&6) || ...
        if decoded_ss58.len() == 35 {
            let mut data = [0u8; 32];
            data.copy_from_slice(&decoded_ss58[1..33]);
            Ok(data)
        } else {
            Err(Error::<T>::InvalidAccountId)
        }
    }

    fn into_account(data: [u8; 32]) -> Result<T::AccountId, Error<T>> {
        T::AccountId::decode(&mut &data[..]).map_err(|_| Error::<T>::InvalidAccountId)
    }

    /// generate transaction for transfer amount to
    fn tx_transfer_to<P, B: AtLeast32Bit + Copy + core::fmt::Display>(
        raw_to: Vec<u8>,
        bridge_asset: BridgeAssetBalance<T::AccountId, T::AssetId, P, B>,
    ) -> Result<IostTxOut<T::AccountId, T::AssetId>, Error<T>>
    where
        P: AtLeast32Bit + Copy,
        B: AtLeast32Bit + Copy,
    {
        let (raw_from, threshold) = BridgeContractAccount::get();
        let memo = core::str::from_utf8(&bridge_asset.memo)
            .map_err(|_| Error::<T>::ParseUtf8Error)?
            .to_string();
        // let amount = (bridge_asset.amount.saturated_into::<u128>() / (10u128.pow(12 - precision as u32))) as i64;
        let amount = (bridge_asset.amount.saturated_into::<u128>() / (10u128.pow(15))) as i64;
        let tx_out = IostTxOut::<T::AccountId, T::AssetId>::init(
            raw_from,
            raw_to,
            amount.to_string(),
            threshold,
            &memo,
            bridge_asset.from,
            bridge_asset.asset_id,
        )?;
        BridgeTxOuts::<T>::append(&tx_out);

        Ok(tx_out)
    }

    fn offchain(_now_block: T::BlockNumber) -> Result<(), Error<T>> {
        //  avoid borrow checker issue if use has_change: bool
        let has_change = core::cell::Cell::new(false);
        let bridge_tx_outs = BridgeTxOuts::<T>::get();

        let node_url = Self::get_offchain_storage(IOST_NODE_URL)?;
        let account_name = Self::get_offchain_storage(IOST_ACCOUNT_NAME)?;
        let sk_str = Self::get_offchain_storage(IOST_SECRET_KEY)?;
        log::info!(target: "bridge-iost", "IOST_NODE_URL ------------- {:?}.", node_url.as_str());
        log::info!(target: "bridge-iost", "IOST_SECRET_KEY ------------- {:?}.", sk_str.as_str());
        let bridge_tx_outs = bridge_tx_outs.into_iter()
            .map(|bto| {
                match bto {
                    // generate raw transactions
                    IostTxOut::<T::AccountId, T::AssetId>::Initial(_) => {
                        match bto.clone().generate::<T>(node_url.as_str()) {
                            Ok(generated_bto) => {
                                has_change.set(true);
                                log::info!(target: "bridge-iost", "bto.generate {:?}",generated_bto);
                                log::info!("bto.generate");
                                generated_bto
                            }
                            Err(e) => {
                                log::info!("failed to get latest block due to: {:?}", e);
                                bto
                            }
                        }
                    },
                    _ => bto,
                }
            })
            .map(|bto| {
                match bto {
                    IostTxOut::<T::AccountId, T::AssetId>::Generated(_) => {
                        let author = <pallet_authorship::Module<T>>::author();
                        let mut ret = bto.clone();
                        let decoded_sk = bs58::decode(sk_str.as_str()).into_vec().map_err(|_| Error::<T>::IostKeysError).unwrap();

                        match bto.sign::<T>(decoded_sk, account_name.as_str()) {
                            Ok(signed_bto) => {
                                has_change.set(true);
                                log::info!(target: "bridge-iost", "bto.sign {:?}", signed_bto);
                                ret = signed_bto;
                            }
                            Err(e) => log::warn!("bto.sign with failure: {:?}", e),
                        }
                        ret
                    },
                    _ => bto,
                }
            })
            .map(|bto| {
                match bto {
                    IostTxOut::<T::AccountId, T::AssetId>::Signed(_) => {
                        match bto.clone().send::<T>(node_url.as_str()) {
                            Ok(sent_bto) => {
                                has_change.set(true);
                                log::info!(target: "bridge-iost", "bto.send {:?}", sent_bto,);
                                log::info!("bto.send");
                                sent_bto
                            }
                            Err(e) => {
                                log::warn!("error happened while pushing transaction: {:?}", e);
                                bto
                            }
                        }
                    },
                    _ => bto,
                }
            }).collect::<Vec<_>>();

        if has_change.get() {
            // BridgeTxOuts::<T>::put(bridge_tx_outs.clone()); // update transaction list

            let call = Call::bridge_tx_report(bridge_tx_outs.clone());
            match SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()) {
                Ok(_) => {
                    log::info!(target: "bridge-iost", "Call::bridge_tx_report {:?}", bridge_tx_outs)
                }
                Err(e) => log::warn!("submit transaction with failure: {:?}", e),
            }
        }
        Ok(())
    }
    // fn convert_to_iost_asset<A, P, B>(
    //     bridge_asset: &BridgeAssetBalance<A, P, B>
    // ) -> Result<Asset, Error<T>>
    //     where
    //         P: AtLeast32Bit + Copy,
    //         B: AtLeast32Bit + Copy
    // {
    //     let precision = bridge_asset.symbol.precision.saturated_into::<u8>();
    //     let symbol_str = core::str::from_utf8(&bridge_asset.symbol.symbol).map_err(|_| Error::<T>::ParseUtf8Error)?;
    //     let symbol_code = SymbolCode::try_from(symbol_str).map_err(|_| Error::<T>::ParseUtf8Error)?;
    //     let symbol = Symbol::new_with_code(precision, symbol_code);
    //
    //     let amount = (bridge_asset.amount.saturated_into::<u128>() / (10u128.pow(12 - precision as u32))) as i64;
    //
    //     Ok(Asset::new(amount, symbol))
    // }
    fn get_offchain_storage(key: &[u8]) -> Result<String, Error<T>> {
        let value = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, key)
            .ok_or(Error::<T>::NoLocalStorage)?;

        Ok(String::from_utf8(value).map_err(|_| Error::<T>::ParseUtf8Error)?)
    }

    fn local_authority_keys() -> impl Iterator<Item = T::AuthorityId> {
        let authorities = NotaryKeys::<T>::get();
        let mut local_keys = T::AuthorityId::all();
        local_keys.sort();

        authorities
            .into_iter()
            .enumerate()
            .filter_map(move |(_, authority)| {
                local_keys
                    .binary_search(&authority.into())
                    .ok()
                    .map(|location| local_keys[location].clone())
            })
    }
}

impl<T: Config> BridgeAssetTo<T::AccountId, T::AssetId, T::Precision, T::Balance> for Module<T> {
    type Error = crate::Error<T>;
    fn bridge_asset_to(
        target: Vec<u8>,
        bridge_asset: BridgeAssetBalance<T::AccountId, T::AssetId, T::Precision, T::Balance>,
    ) -> Result<(), Self::Error> {
        let _ = Self::tx_transfer_to(target, bridge_asset)?;

        Ok(())
    }

    fn redeem(_: T::AssetId, _: T::Balance, _: Vec<u8>) -> Result<(), Self::Error> {
        Ok(())
    }
    fn stake(_: T::AssetId, _: T::Balance, _: Vec<u8>) -> Result<(), Self::Error> {
        Ok(())
    }
    fn unstake(_: T::AssetId, _: T::Balance, _: Vec<u8>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[allow(deprecated)]
impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
        if let Call::bridge_tx_report(_) = call {
            let now_block = <frame_system::Module<T>>::block_number().saturated_into::<u64>();
            ValidTransaction::with_tag_prefix("BridgeIost")
                .priority(TransactionPriority::max_value())
                .and_provides(vec![(now_block).encode()])
                .longevity(TransactionLongevity::max_value())
                .propagate(true)
                .build()
        } else {
            InvalidTransaction::Call.into()
        }
    }
}
