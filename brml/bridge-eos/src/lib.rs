// Copyright 2019-2020 Liebi Technologies.
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
use core::{convert::TryFrom, ops::Div, str::FromStr, fmt::Debug};
use crate::transaction::{TxOut, TxOutV1};
use codec::{Decode, Encode};
use eos_chain::{
	Action, ActionTransfer, ActionReceipt, Asset, Checksum256, Digest, IncrementalMerkle,
	ProducerSchedule, SignedBlockHeader, Symbol, SymbolCode, Read, verify_proof, ActionName,
	ProducerAuthoritySchedule, ProducerAuthority,
};
use eos_keys::secret::SecretKey;
use sp_std::prelude::*;
use sp_core::offchain::StorageKind;
use sp_runtime::{
	traits::{Member, SaturatedConversion, Saturating, AtLeast32Bit, MaybeSerializeDeserialize, Zero},
	transaction_validity::{
		InvalidTransaction, TransactionLongevity, TransactionPriority,
		TransactionValidity, ValidTransaction, TransactionSource
	},
};
use frame_support::{
	decl_event, decl_module, decl_storage, decl_error, debug, ensure, Parameter, traits::Get,
	dispatch::DispatchResult, weights::{DispatchClass, Weight, Pays}, IterableStorageMap, StorageValue,
};
use frame_system::{
	self as system, ensure_root, ensure_none, ensure_signed, offchain::{SubmitTransaction, SendTransactionTypes}
};
use node_primitives::{
	AssetTrait, BridgeAssetBalance, BridgeAssetFrom, BridgeAssetTo, BridgeAssetSymbol, BlockchainType, TokenSymbol,
	FetchConvertPool,
};
use sp_application_crypto::RuntimeAppPublic;

mod transaction;
mod mock;
mod tests;

lazy_static::lazy_static! {
	pub static ref ACTION_NAMES: [ActionName; 1] = {
		let name = ActionName::from_str("transfer").unwrap();
		[name]
	};
}

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
				let mut data =  [0u8;32];
				let acct_data: &[u8;32] = acct.as_ref();
				for (index, val) in acct_data.iter().enumerate() {
					data[index] = *val;
				}
				Self(sp_core::sr25519::Public(data))
			}
		}
	}

	sp_application_crypto::with_pair! {
		/// An bridge-eos keypair using sr25519 as its crypto.
		pub type AuthorityPair = app_sr25519::Pair;
	}

	/// An bridge-eos signature using sr25519 as its crypto.
	pub type AuthoritySignature = app_sr25519::Signature;

	/// An bridge-eos identifier using sr25519 as its crypto.
	pub type AuthorityId = app_sr25519::Public;
}

pub mod ed25519 {
	mod app_ed25519 {
		use sp_application_crypto::{app_crypto, key_types::ACCOUNT, ed25519};
		app_crypto!(ed25519, ACCOUNT);
	}

	sp_application_crypto::with_pair! {
		/// An bridge-eos keypair using ed25519 as its crypto.
		pub type AuthorityPair = app_ed25519::Pair;
	}

	/// An bridge-eos signature using ed25519 as its crypto.
	pub type AuthoritySignature = app_ed25519::Signature;

	/// An bridge-eos identifier using ed25519 as its crypto.
	pub type AuthorityId = app_ed25519::Public;
}

const EOS_NODE_URL: &[u8] = b"EOS_NODE_URL";
const EOS_SECRET_KEY: &[u8] = b"EOS_SECRET_KEY";

#[derive(Encode, Decode, Clone, PartialEq, Debug, Copy)]
#[non_exhaustive]
pub enum TrxStatus {
	Initial,
	Generated,
	Signed,
	Processing,
	Success,
	Fail,
}

impl Default for TrxStatus {
	fn default() -> Self {
		Self::Initial
	}
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
		InvalidGeneratedTxOutType,
		InvalidSignedTxOutType,
		InvalidSendTxOutType,
		/// Error from eos-chain crate
		EosChainError,
		/// Error from eos-key crate
		EosKeysError,
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
		EOSRpcError,
		/// Error from lite-json while serializing or deserializing
		LiteJsonError,
		/// Invalid checksum
		InvalidChecksum256,
		/// Initialze producer schedule multiple times
		InitMultiTimeProducerSchedules,
		/// EOS or vEOS not existed
		TokenNotExist,
		/// Invalid token
		InvalidTokenForTrade,
		/// EOSSymbolMismatch,
		EOSSymbolMismatch,
		/// Bridge eos has been disabled
		CrossChainDisabled,
		/// Who hasn't the permission to sign a cross-chain trade
		NoPermissionSignCrossChainTrade,
		/// There's no any blockheaders for verifying
		InvalidDataForVerifyingBlockheaders,
		/// Blockheaders length are not equal to Id list
		/// These Id list are for verifying blockheaders
		BlockHeaderLengthMismatchWithIdList,
		/// Fail to verify blockheaders
		FailureOnVerifyingBlockheaders,
		/// Duplicated trade because this trade has been on bifrost
		DuplicatedCrossChainTransaction,
		/// This action is invalid
		InvalidAction,
		/// Fail to verify transaction action
		FailureOnVerifyingTransactionAction,
		/// Send duplicated transaction to EOS node
		SendingDuplicatedTransaction,
		/// Transaction expired
		TransactionExpired,
		/// Cross transaction back enable or not
		CrossChainBackDisabled,
	}
}

pub type VersionId = u32;

pub trait Trait: SendTransactionTypes<Call<Self>> + pallet_authorship::Trait {
	/// The identifier type for an authority.
	type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord
		+ From<<Self as frame_system::Trait>::AccountId>;

	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// The units in which we record balances.
	type Balance: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy + From<TokenSymbol> + Into<TokenSymbol> + MaybeSerializeDeserialize;

	/// The units in which we record costs.
	type Cost: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The units in which we record incomes.
	type Income: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The units in which we record asset precision.
	type Precision: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// Bridge asset from another blockchain.
	type BridgeAssetFrom: BridgeAssetFrom<Self::AccountId, Self::Precision, Self::Balance>;

	type AssetTrait: AssetTrait<Self::AssetId, Self::AccountId, Self::Balance, Self::Cost, Self::Income>;

	/// Fetch convert pool from convert module
	type FetchConvertPool: FetchConvertPool<TokenSymbol, Self::Balance>;

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
		Deposit(Vec<u8>, AccountId), // EOS account => Bifrost AccountId
		DepositFail,
		Withdraw(AccountId, Vec<u8>), // Bifrost AccountId => EOS account
		WithdrawFail,
		SentCrossChainTransaction,
		FailToSendCrossChainTransaction,
		GrantedCrossChainPrivilege(AccountId),
		RemovedCrossChainPrivilege(AccountId),
		UnsignedTrx,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as BridgeEos {
		/// The current set of notary keys that may send bridge transactions to Eos chain.
		NotaryKeys get(fn notary_keys) config(): Vec<T::AccountId>;

		/// Config to enable/disable this runtime
		BridgeEnable get(fn is_bridge_enable): bool = true;

		/// Cross transaction back enable or not
		CrossChainBackEnable get(fn is_cross_back_enable): bool = true;

		/// Eos producer list and hash which in specific version id
		ProducerSchedules: map hasher(blake2_128_concat) VersionId => (Vec<ProducerAuthority>, Checksum256);

		/// Initialize a producer schedule while starting a node.
		InitializeSchedule get(fn producer_schedule): ProducerAuthoritySchedule;

		/// Save all unique transactions
		/// Every transaction has different action receipt, but can have the same action
		BridgeActionReceipt: map hasher(blake2_128_concat) ActionReceipt => Action;
		/// Transaction ID is unique on EOS
		BridgeTransactionID: map hasher(blake2_128_concat) Checksum256 => ActionReceipt;

		/// Current pending schedule version
		PendingScheduleVersion: VersionId;

		/// Transaction sent to Eos blockchain
		BridgeTrxStatus get(fn trx_status): map hasher(blake2_128_concat) TxOut<T::AccountId> => TrxStatus;
		BridgeTrxStatusV1 get(fn trx_status_v1): map hasher(blake2_128_concat) (TxOut<T::AccountId>, u64) => TrxStatus;
		CrossTradeIndex get(fn cross_trade_index): u64 = 0;
		CrossTradeStatus get(fn cross_status): map hasher(blake2_128_concat) u64 => bool;
		CrossIndexRelatedEOSBalance get(fn index_with_eos_balance): map hasher(blake2_128_concat) u64 => (T::Balance, T::AccountId, TokenSymbol);
		/// According trx id to find processing trx
		ProcessingBridgeTrx: map hasher(blake2_128_concat) Checksum256 => TxOut<T::AccountId>;

		/// V2 storage
		/// Transaction sent to Eos blockchain
		BridgeTrxStatusV2 get(fn trx_status_v2): map hasher(blake2_128_concat) (TxOutV1<T::AccountId>, u64) => TrxStatus;
		CrossTradeIndexV2 get(fn cross_trade_index_v2): map hasher(blake2_128_concat) T::AccountId => u64 = 0;
		CrossTradeStatusV2 get(fn cross_status_v2): map hasher(blake2_128_concat) u64 => bool;
		EOSNodeAddress get(fn eos_node): Vec<u8> = b"http://122.51.241.19:8080".to_vec();
		CrossIndexRelatedEOSBalanceV2 get(fn index_with_eos_balance_v2): map hasher(blake2_128_concat) u64 => (T::Balance, T::AccountId, TokenSymbol);
		/// According trx id to find processing trx
		ProcessingBridgeTrxV2: map hasher(blake2_128_concat) Checksum256 => (TxOutV1<T::AccountId>, u64);

		/// Account where Eos bridge contract deployed, (Account, Signature threshold)
		BridgeContractAccount get(fn bridge_contract_account) config(): (Vec<u8>, u8);

		/// Who has the privilege to call transaction between Bifrost and EOS
		CrossChainPrivilege get(fn cross_chain_privilege) config(): map hasher(blake2_128_concat) T::AccountId => bool;
		/// How many address has the privilege sign transaction between EOS and Bifrost
		AllAddressesHaveCrossChainPrivilege get(fn all_crosschain_privilege) config(): Vec<T::AccountId>;

		/// Record times of cross-chain trade, (EOS => Bifrost, Bifrost => EOS)
		TimesOfCrossChainTrade get(fn trade_times): map hasher(blake2_128_concat) T::AccountId => (u32, u32) = (0u32, 0u32);
		/// Set low limit amount of EOS for cross transaction, if it's bigger than this, count one.
		LowLimitOnCrossChain get(fn cross_trade_eos_limit) config(): T::Balance;
	}
	add_extra_genesis {
		build(|config: &GenesisConfig<T>| {
			BridgeContractAccount::put(config.bridge_contract_account.clone());

			NotaryKeys::<T>::put(config.notary_keys.clone());

			let schedule = ProducerAuthoritySchedule::default();
			let schedule_hash = schedule.schedule_hash();
			assert!(schedule_hash.is_ok());
			ProducerSchedules::insert(schedule.version, (schedule.producers, schedule_hash.unwrap()));
			PendingScheduleVersion::put(schedule.version);

			// grant privilege to sign transaction between EOS and Bifrost
			for (who, privilege) in config.cross_chain_privilege.iter() {
				<CrossChainPrivilege<T>>::insert(who, privilege);
			}
			// update to AllAddressesHaveCrossChainPrivilege
			let all_addresses: Vec<T::AccountId> = config.cross_chain_privilege.iter().map(|x| x.0.clone()).collect();
			<AllAddressesHaveCrossChainPrivilege<T>>::mutate(move |all| {
				all.extend(all_addresses.into_iter());
			});

			// setting limit on how many eos counts one
			LowLimitOnCrossChain::<T>::put(config.cross_trade_eos_limit);
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		#[weight = T::DbWeight::get().writes(1)]
		fn clear_cross_trade_times(origin) {
			ensure_root(origin)?;

			// clear cross trade times
			for (who, _) in TimesOfCrossChainTrade::<T>::iter() {
				TimesOfCrossChainTrade::<T>::mutate(&who, |pool| {
					pool.0 = Zero::zero();
					pool.1 = Zero::zero();
				});
			}
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn clear_unused_cross_back_transaction_data(origin) {
			ensure_root(origin)?;

			// clear cross trade times
			for (key, val) in BridgeTrxStatusV2::<T>::iter() {
				BridgeTrxStatusV2::<T>::remove(key);
			}
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn bridge_enable(origin, enable: bool) {
			ensure_root(origin)?;

			BridgeEnable::put(enable);
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn change_eos_node_address(origin, address: Vec<u8>) {
			ensure_root(origin)?;

			EOSNodeAddress::put(address);
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn cross_chain_back_enable(origin, enable: bool) {
			ensure_root(origin)?;

			CrossChainBackEnable::put(enable);
		}

		#[weight = T::DbWeight::get().reads_writes(1, 2)]
		fn save_producer_schedule(origin, ps: ProducerAuthoritySchedule) -> DispatchResult {
			ensure_root(origin)?;

			let schedule_hash = ps.schedule_hash().map_err(|_| Error::<T>::InvalidScheduleHash)?;

			// calculate schedule hash just one time, instead of calculating it multiple times.
			ProducerSchedules::insert(ps.version, (ps.producers, schedule_hash));
			PendingScheduleVersion::put(ps.version);

			Self::deposit_event(RawEvent::InitSchedule(ps.version));

			Ok(())
		}

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn init_schedule(origin, ps: ProducerAuthoritySchedule) {
			ensure_root(origin)?;

			ensure!(!ProducerSchedules::contains_key(ps.version), Error::<T>::InitMultiTimeProducerSchedules);
			ensure!(!PendingScheduleVersion::exists(), Error::<T>::InitMultiTimeProducerSchedules);

			let schedule_hash = ps.schedule_hash().map_err(|_| Error::<T>::InvalidScheduleHash)?;

			// calculate schedule hash just one time, instead of calculating it multiple times.
			ProducerSchedules::insert(ps.version, (ps.producers, schedule_hash));
			PendingScheduleVersion::put(ps.version);

			Self::deposit_event(RawEvent::InitSchedule(ps.version));
		}

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
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

		#[weight = (0, DispatchClass::Normal, Pays::No)]
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

		#[weight = (0, DispatchClass::Normal, Pays::No)]
		fn set_contract_accounts(origin, account: Vec<u8>, threthold: u8) {
			ensure_root(origin)?;
			BridgeContractAccount::put((account, threthold));
		}

		// 1. block_headers length must be 15.
		// 2. the first block_header's new_producers cannot be none.
		// 3. compare current schedules version with pending_schedules'.
		// 4. verify incoming 180 block_headers to prove this new_producers list is valid.
		// 5. save the new_producers list.
		#[weight = (10000, DispatchClass::Normal, Pays::Yes)]
		fn change_schedule(
			origin,
			legacy_schedule_hash: Checksum256,
			new_schedule: ProducerAuthoritySchedule,
			merkle: IncrementalMerkle,
			block_headers: Vec<SignedBlockHeader>,
			block_ids_list: Vec<Vec<Checksum256>>
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(CrossChainPrivilege::<T>::get(&origin), Error::<T>::NoPermissionSignCrossChainTrade);

			// check the data is valid for verifying these blockheaders
			ensure!(BridgeEnable::get(), Error::<T>::CrossChainDisabled);
			ensure!(!block_headers.is_empty(), Error::<T>::InvalidDataForVerifyingBlockheaders);
			ensure!(block_ids_list.len() == block_headers.len(), Error::<T>::InvalidDataForVerifyingBlockheaders);
			ensure!(block_ids_list.len() == 15, Error::<T>::InvalidDataForVerifyingBlockheaders);
			ensure!(block_ids_list[0].is_empty(), Error::<T>::InvalidDataForVerifyingBlockheaders);
			ensure!(block_ids_list[1].len() ==  10, Error::<T>::InvalidDataForVerifyingBlockheaders);

			let current_schedule_version = PendingScheduleVersion::get();

			let (schedule_hash, producer_schedule) = {
				let schedule_hash = new_schedule.schedule_hash().map_err(|_| Error::<T>::InvalidScheduleHash)?;
				(schedule_hash, new_schedule)
			};

			ensure!(
				Self::verify_block_headers(merkle, &schedule_hash, &producer_schedule, &block_headers, block_ids_list).is_ok(),
				Error::<T>::FailureOnVerifyingBlockheaders
			);

			// if verification is successful, save the new producers schedule.
			ProducerSchedules::insert(producer_schedule.version, (&producer_schedule.producers, schedule_hash));
			PendingScheduleVersion::put(producer_schedule.version);

			Self::deposit_event(RawEvent::ChangeSchedule(current_schedule_version, producer_schedule.version));

			Ok(())
		}

		#[weight = (10000, DispatchClass::Normal, Pays::Yes)]
		fn prove_action(
			origin,
			action: Action,
			action_receipt: ActionReceipt,
			action_merkle_paths: Vec<Checksum256>,
			merkle: IncrementalMerkle,
			block_headers: Vec<SignedBlockHeader>,
			block_ids_list: Vec<Vec<Checksum256>>,
			trx_id: Checksum256
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(CrossChainPrivilege::<T>::get(&origin), Error::<T>::NoPermissionSignCrossChainTrade);

			// ensure this transaction is unique, and ensure no duplicated transaction
			ensure!(BridgeActionReceipt::get(&action_receipt).ne(&action), Error::<T>::DuplicatedCrossChainTransaction);

			// ensure action is what we want
			ensure!(action.name == ACTION_NAMES[0], Error::<T>::InvalidAction);
			let action_hash = action.digest().map_err(|_| Error::<T>::ErrorOnCalculationActionHash)?;
			ensure!(action_hash == action_receipt.act_digest, Error::<T>::InvalidAction);

			// check the data is valid for verifying these blockheaders
			ensure!(BridgeEnable::get(), Error::<T>::CrossChainDisabled);
			ensure!(block_ids_list.len() == block_headers.len(), Error::<T>::InvalidDataForVerifyingBlockheaders);
			ensure!(block_ids_list.len() == 15, Error::<T>::InvalidDataForVerifyingBlockheaders);
			ensure!(block_ids_list[0].is_empty(), Error::<T>::InvalidDataForVerifyingBlockheaders);
			ensure!(block_ids_list[1].len() ==  10, Error::<T>::InvalidDataForVerifyingBlockheaders);

			let leaf = action_receipt.digest().map_err(|_| Error::<T>::ErrorOnCalculationActionReceiptHash)?;

			let block_under_verification = &block_headers[0];
			// verify transaction and related action from this transaction
			ensure!(
				verify_proof(&action_merkle_paths, leaf, block_under_verification.block_header.action_mroot),
				Error::<T>::FailureOnVerifyingTransactionAction
			);

			let (schedule_hash, producer_schedule) = Self::get_schedule_hash_and_public_key(block_headers[0].block_header.new_producers.as_ref())?;
			// this is for testing due to there's a default producer schedule on standalone eos node.
			let schedule_hash = {
				if producer_schedule.version == 0 {
					ProducerSchedule::default().schedule_hash().map_err(|_| Error::<T>::InvalidScheduleHash)?
				} else {
					schedule_hash
				}
			};

			// verify EOS block headers
			ensure!(
				Self::verify_block_headers(merkle, &schedule_hash, &producer_schedule, &block_headers, block_ids_list).is_ok(),
				Error::<T>::FailureOnVerifyingBlockheaders
			);

			// save proves for this transaction
			BridgeActionReceipt::insert(&action_receipt, &action);

			Self::deposit_event(RawEvent::ProveAction);

			let action_transfer = Self::get_action_transfer_from_action(&action)?;

			let cross_account = BridgeContractAccount::get().0;
			// withdraw operation, Bifrost => EOS
			if cross_account == action_transfer.from.to_string().into_bytes() {
				match Self::transaction_from_bifrost_to_eos(trx_id, &action_transfer) {
					Ok(target) => {
						Self::deposit_event(RawEvent::Withdraw(target, action_transfer.to.to_string().into_bytes()));
					}
					Err(e) => {
						debug::warn!("Bifrost => EOS failed due to {:?}", e);
						Self::deposit_event(RawEvent::WithdrawFail);
					}
				}
			}

			// deposit operation, EOS => Bifrost
			if cross_account == action_transfer.to.to_string().into_bytes() {
				match Self::transaction_from_eos_to_bifrost(&action_transfer) {
					Ok((target, eos_amount)) => {
						// update times of trade from EOS => Bifrost
						if LowLimitOnCrossChain::<T>::get() <= eos_amount {
							TimesOfCrossChainTrade::<T>::mutate(&target, |times| {
								times.0 = times.0.saturating_add(1);
							});
						}
						Self::deposit_event(RawEvent::Deposit(action_transfer.from.to_string().into_bytes(), target));
					}
					Err(e) => {
						debug::info!("EOS => Bifrost failed due to {:?}", e);
						Self::deposit_event(RawEvent::DepositFail);
					}

				}
			}

			Ok(())
		}

		#[weight = (0, DispatchClass::Normal, Pays::No)]
		fn update_bridge_trx_status(
			origin,
			changed_trxs: Vec::<((TxOutV1<T::AccountId>, u64), TrxStatus, (TxOutV1<T::AccountId>, u64), Option<Checksum256>)>
		) -> DispatchResult {
			ensure_none(origin)?;

			for changed_trx in changed_trxs.iter() {
				let trade_index = changed_trx.0.1;

				// delete last changed status of transaction
				BridgeTrxStatusV2::<T>::remove(&changed_trx.2);

				if let Some(id) = changed_trx.3 {
					// insert the processing transaction to processing storage
					ProcessingBridgeTrxV2::<T>::insert(id, (&changed_trx.0.0, trade_index));
					// means that this transaction has been sent to EOS node
					CrossTradeStatusV2::mutate(trade_index, |sent| {
						*sent = true;
					});
					// due to it becoming processing status, delete it from BridgeTrxStatusV1
					BridgeTrxStatusV2::<T>::remove(&changed_trx.0);
				}

				// this transaction has been not in processing or signed status
				if !CrossTradeStatusV2::get(trade_index) {
					BridgeTrxStatusV2::<T>::insert(&changed_trx.0, changed_trx.1);
				} else {
					BridgeTrxStatusV2::<T>::remove(&changed_trx.0);
				}
			}

			Self::deposit_event(RawEvent::UnsignedTrx);
			Ok(())
		}

		#[weight = (weight_for::cross_to_eos::<T>(memo.len() as Weight), DispatchClass::Normal)]
		fn cross_to_eos(
			origin,
			to: Vec<u8>,
			token_symbol: TokenSymbol,
			#[compact] amount: T::Balance,
			memo: Vec<u8>
		) {
			let origin = system::ensure_signed(origin)?;
			let eos_amount = amount;

			ensure!(CrossChainBackEnable::get(), Error::<T>::CrossChainBackDisabled);

			// check vtoken id exist or not
			ensure!(T::AssetTrait::token_exists(token_symbol), Error::<T>::TokenNotExist);
			// ensure redeem EOS instead of any others tokens like vEOS, DOT, KSM etc
			ensure!(token_symbol == TokenSymbol::EOS, Error::<T>::InvalidTokenForTrade);

			let token = T::AssetTrait::get_token(token_symbol);
			let symbol_code = token.symbol;
			let symbol_precise = token.precision;

			let balance = T::AssetTrait::get_account_asset(token_symbol, &origin).balance;
			ensure!(symbol_precise <= 12, Error::<T>::EOSSymbolMismatch);
			let amount = amount.div(T::Balance::from(10u32.pow(12u32 - symbol_precise as u32)));
			ensure!(balance >= eos_amount, Error::<T>::InsufficientBalance);

			let asset_symbol = BridgeAssetSymbol::new(BlockchainType::EOS, symbol_code, T::Precision::from(symbol_precise.into()));
			let memo = CrossTradeIndexV2::<T>::get(&origin).to_string().into_bytes();
			let bridge_asset = BridgeAssetBalance {
				symbol: asset_symbol,
				amount: eos_amount,
				memo,
				from: origin.clone(),
				token_symbol
			};

			match Self::bridge_asset_to(to, bridge_asset) {
				Ok(_) => {
					debug::info!("sent transaction to EOS node.");
					// locked balance until trade is verified
					T::AssetTrait::lock_asset(&origin, token_symbol, eos_amount);

					Self::deposit_event(RawEvent::SentCrossChainTransaction);
				}
				Err(e) => {
					debug::warn!("failed to send transaction to EOS node, due to {:?}", e);
					Self::deposit_event(RawEvent::FailToSendCrossChainTransaction);
				}
			}
		}

		// Runs after every block.
		fn offchain_worker(now_block: T::BlockNumber) {
			debug::RuntimeLogger::init();

			// trigger offchain worker by each two block
			if now_block % T::BlockNumber::from(2u32) == T::BlockNumber::from(0u32) {
				if BridgeTrxStatusV2::<T>::iter()
					.any(|(_, status)|
						status == TrxStatus::Initial ||
						status == TrxStatus::Generated ||
						status == TrxStatus::Signed
					)
				{
					match Self::offchain(now_block) {
						Ok(_) => debug::info!("A offchain worker started."),
						Err(e) => (),
					}
				}
			}
			let author = <pallet_authorship::Module<T>>::author();
			debug::debug!(target: "bridge-eos", "current node is: {:?}", <pallet_authorship::Module<T>>::author());
		}
	}
}

impl<T: Trait> Module<T> {
	fn verify_block_headers(
		mut merkle: IncrementalMerkle,
		schedule_hash: &Checksum256,
		producer_schedule: &ProducerAuthoritySchedule,
		block_headers: &[SignedBlockHeader],
		block_ids_list: Vec<Vec<Checksum256>>,
	) -> Result<(), Error<T>> {
		ensure!(block_headers.len() == 15, Error::<T>::InvalidBlockHeadersLength);
		ensure!(block_ids_list.len() == 15, Error::<T>::InvalidBlockHeadersLength);

		for (block_header, block_ids) in block_headers.iter().zip(block_ids_list.iter()) {
			// calculate merkle root
			Self::calculate_block_header_merkle_root(&mut merkle, &block_header, &block_ids)?;

			// verify block header signature
			Self::verify_block_header_signature(schedule_hash, producer_schedule, block_header, &merkle.get_root()).map_err(|_| Error::<T>::SignatureVerificationFailure)?;

			// append current block id
			let block_id = block_header.id().map_err(|_| Error::<T>::FailureOnGetBlockId)?;
			merkle.append(block_id).map_err(|_| Error::<T>::AppendIncreMerkleError)?;
		}

		Ok(())
	}

	fn verify_block_header_signature(
		schedule_hash: &Checksum256,
		producer_schedule: &ProducerAuthoritySchedule,
		block_header: &SignedBlockHeader,
		expected_mroot: &Checksum256,
	) -> Result<(), Error<T>> {
		let pk = producer_schedule.get_producer_key(block_header.block_header.producer);
		block_header.verify(*expected_mroot, *schedule_hash, pk).map_err(|_| Error::<T>::SignatureVerificationFailure)?;

		Ok(())
	}

	fn calculate_block_header_merkle_root(
		merkle: &mut IncrementalMerkle,
		block_header: &SignedBlockHeader,
		block_ids: &[Checksum256],
	) -> Result<(), Error<T>> {
		for id in block_ids {
			merkle.append(*id).map_err(|_| Error::<T>::AppendIncreMerkleError)?;
		}

		// append previous block id
		merkle.append(block_header.block_header.previous).map_err(|_| Error::<T>::AppendIncreMerkleError)?;

		Ok(())
	}

	fn get_schedule_hash_and_public_key(
		new_producers: Option<&ProducerSchedule>
	) -> Result<(Checksum256, ProducerAuthoritySchedule), Error<T>> {
		let ps = match new_producers {
			Some(producers) => {
				let schedule_version = PendingScheduleVersion::get();
				if schedule_version != producers.version {
					return Err(Error::<T>::InvalidScheduleHash)
				}
				let producers = ProducerSchedules::get(schedule_version).0;
				let ps = ProducerAuthoritySchedule::new(schedule_version, producers);
				ps
			},
			None => {
				let schedule_version = PendingScheduleVersion::get();
				let producers = ProducerSchedules::get(schedule_version).0;
				let ps = ProducerAuthoritySchedule::new(schedule_version, producers);
				ps
			}
		};

		let schedule_hash: Checksum256 = ps.schedule_hash().map_err(|_| Error::<T>::InvalidScheduleHash)?;

		Ok((schedule_hash, ps))
	}

	fn get_action_transfer_from_action(act: &Action) -> Result<ActionTransfer, Error<T>> {
		let action_transfer = ActionTransfer::read(&act.data, &mut 0).map_err(|_| Error::<T>::EosChainError)?;

		Ok(action_transfer)
	}

	fn transaction_from_eos_to_bifrost(
		action_transfer: &ActionTransfer
	) -> Result<(T::AccountId, T::Balance), Error<T>> {
		// check memo, example like "alice@bifrost:EOS", the formatter: {receiver}@{chain}:{token_symbol}
		let split_memo = action_transfer.memo.as_str().split(|c| c == '@' || c == ':').collect::<Vec<_>>();

		// the length should be 2, either 3.
		if split_memo.len().gt(&3) || split_memo.len().lt(&2) {
			return Err(Error::<T>::InvalidMemo);
		}

		// get account
		let account_data = Self::get_account_data(split_memo[0])?;
		let target = Self::into_account(account_data)?;

		let token_symbol = {
			match split_memo.len() {
				2 => TokenSymbol::vEOS,
				3 => {
					match split_memo[2] {
						"" | "vEOS" => TokenSymbol::vEOS,
						"EOS" => TokenSymbol::EOS,
						_ => {
							debug::error!("A invalid token type, default token type will be vtoken");
							return Err(Error::<T>::InvalidMemo);
						}
					}
				}
				_ => unreachable!("previous step checked he length of split_memo.")
			}
		};
		// todo, vEOS or EOS, all asset will be added to EOS asset, instead of vEOS or EOS
		// but in the future, we will support both token, let user to which token he wants to get
		// according to the convert price
		let token_symbol_pair = token_symbol.paired_token(); // return (token_symbol, vtoken_symbol)
		let convert_pool = T::FetchConvertPool::fetch_convert_pool(token_symbol_pair.0);

		let symbol = action_transfer.quantity.symbol;
		let symbol_code = symbol.code().to_string().into_bytes();
		let symbol_precision = symbol.precision() as u16;
		// ensure symbol and precision matched
		let existed_token_symbol = T::AssetTrait::get_token(token_symbol_pair.0);
		ensure!(
			existed_token_symbol.symbol == symbol_code && existed_token_symbol.precision == symbol_precision,
			Error::<T>::EOSSymbolMismatch
		);

		let token_balances = (action_transfer.quantity.amount as u128) * 10u128.pow(12 - symbol_precision as u32);
		let new_balance: T::Balance = TryFrom::<u128>::try_from(token_balances).map_err(|_| Error::<T>::ConvertBalanceError)?;

		if token_symbol.is_vtoken() {
			// according convert pool to convert EOS to vEOS
			let vtoken_balances: T::Balance = {
				new_balance.saturating_mul(convert_pool.vtoken_pool) / convert_pool.token_pool
			};
			T::AssetTrait::asset_issue(token_symbol_pair.1, &target, vtoken_balances);
		} else {
			T::AssetTrait::asset_issue(token_symbol_pair.0, &target, new_balance);
		}

		Ok((target, new_balance))
	}

	fn transaction_from_bifrost_to_eos(
		pending_trx_id: Checksum256,
		action_transfer: &ActionTransfer
	) -> Result<T::AccountId, Error<T>> {
		let (processing_trx, trade_index) = ProcessingBridgeTrxV2::<T>::get(&pending_trx_id);
		debug::error!(target: "bridge-eos", "bifrost => eos {:?}", processing_trx);
		debug::error!(target: "bridge-eos", "bifrost => eos index {:?}", trade_index);
		match processing_trx {
			TxOutV1::Sent { tx_id, ref from, token_symbol } if pending_trx_id.eq(&tx_id) => {
				let target = from.clone();
				let token_symbol = token_symbol;

				let all_vtoken_balances = T::AssetTrait::get_account_asset(token_symbol, &target).balance;

				let symbol = action_transfer.quantity.symbol;
				let symbol_code = symbol.code().to_string().into_bytes();
				let symbol_precision = symbol.precision() as u16;
				// ensure symbol and precision matched
				let existed_token_symbol = T::AssetTrait::get_token(token_symbol);
				ensure!(
					existed_token_symbol.symbol == symbol_code && existed_token_symbol.precision == symbol_precision,
					Error::<T>::EOSSymbolMismatch
				);

				let token_balances = (action_transfer.quantity.amount as u128) * 10u128.pow(12 - symbol_precision as u32);
				let vtoken_balances = TryFrom::<u128>::try_from(token_balances).map_err(|_| Error::<T>::ConvertBalanceError)?;

				if all_vtoken_balances.lt(&vtoken_balances) {
					debug::warn!("origin account balance must be greater than or equal to the transfer amount.");
					return Err(Error::<T>::InsufficientBalance);
				}

				// the trade is verified, unlock asset
				T::AssetTrait::unlock_asset(&target, token_symbol, vtoken_balances);

				// update times of trade from Bifrost => EOS
				if LowLimitOnCrossChain::<T>::get() <= vtoken_balances {
					TimesOfCrossChainTrade::<T>::mutate(&target, |times| {
						times.1 = times.1.saturating_add(1);
					});
				}

				// change status of this transction, remove it from BridgeTrxStatus
				BridgeTrxStatusV2::<T>::remove(&(processing_trx, trade_index));

				// delete this handled transaction
				ProcessingBridgeTrxV2::<T>::remove(pending_trx_id);

				return Ok(target.clone());
			}
			_ => (),
		}

		Err(Error::<T>::InvalidAccountId)
	}

	/// check receiver account format
	/// https://github.com/paritytech/substrate/wiki/External-Address-Format-(SS58)
	fn get_account_data(receiver: &str) -> Result<[u8; 32], Error<T>> {
		let decoded_ss58 = bs58::decode(receiver).into_vec().map_err(|_| Error::<T>::InvalidAccountId)?;

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
	fn tx_transfer_to<P, B>(
		raw_to: Vec<u8>,
		bridge_asset: BridgeAssetBalance<T::AccountId, P, B>,
	) -> Result<TxOutV1<T::AccountId>, Error<T>>
		where
			P: AtLeast32Bit + Copy,
			B: AtLeast32Bit + Copy,
	{
		let (raw_from, threshold) = BridgeContractAccount::get();
		let memo = core::str::from_utf8(&bridge_asset.memo).map_err(|_| Error::<T>::ParseUtf8Error)?.to_string();
		let amount = Self::convert_to_eos_asset::<T::AccountId, P, B>(&bridge_asset)?;

		let tx_out = TxOutV1::<T::AccountId>::init(raw_from, raw_to, amount, threshold, &memo, bridge_asset.from.clone(), bridge_asset.token_symbol)?;

		CrossTradeIndexV2::<T>::mutate(&bridge_asset.from, |index| {
			*index += 1;
		});
		
		BridgeTrxStatusV2::<T>::insert((&tx_out, CrossTradeIndexV2::<T>::get(&bridge_asset.from)), TrxStatus::Initial);

		Ok(tx_out)
	}

	fn offchain(_now_block: T::BlockNumber) -> Result<(), Error<T>> {
		// let node_url = Self::get_offchain_storage(EOS_NODE_URL)?;

		// let sk_str = Self::get_offchain_storage(EOS_SECRET_KEY)?;
		// let sk = SecretKey::from_wif(&sk_str).map_err(|_| Error::<T>::ParseSecretKeyError)?;

		// let alice = (
		// 	"g2ZXjuTYMgCZdunkCLUnxHfGK7Dvus4rAzeJAcQ8ZDLq3bV",
		// 	"http://10.115.61.83:6123",
		// 	"5KDXMiphWpzETsNpp3eL3sjWAa4gMvMXCtMquT2PDpKtV1STbHp"
		// );
		// let bob = (
		// 	"hUWrDdiLs9uu6Hu1qA6Edt9GTW3mFWXR5uEAQZZT45KJszT",
		// 	"http://10.115.61.83:6123",
		// 	"5JNV39rZLZWr5p1hdLXVVNvJsXpgZnzvTrcZYJggTPuv1GzChB6"
		// );

		let alice = (
			"fw9DNydGr6yDbe4PF2yd9JX5HY7LZLFGAaRXXAJTuW3MdE7",
			"http://122.51.241.19:8080",
			"5KDXMiphWpzETsNpp3eL3sjWAa4gMvMXCtMquT2PDpKtV1STbHp"
		);
		let bob = (
			"cgUeaC4T7BCyx4CVX3HXvee2PUqqM5bE1VFVHNccovXeJEs",
			"http://122.51.241.19:8080",
			"5JNV39rZLZWr5p1hdLXVVNvJsXpgZnzvTrcZYJggTPuv1GzChB6"
		);

		let node_url = String::from_utf8(EOSNodeAddress::get()).map_err(|_| Error::<T>::ParseUtf8Error)?;
		debug::error!(target: "bridge-eos", "_node_url {:?}", node_url);
		let mut sk_str = String::new();
		{
			let (alice_acc, bob_acc) = (Self::get_account_data(alice.0)?, Self::get_account_data(bob.0)?);
			let (alice_acc, bob_acc) = (Self::into_account(alice_acc)?, Self::into_account(bob_acc)?);
			if <pallet_authorship::Module<T>>::author() == alice_acc {
				sk_str = alice.2.to_string();
			}
			if <pallet_authorship::Module<T>>::author() == bob_acc {
				sk_str = bob.2.to_string();
			}
		};
		let sk = SecretKey::from_wif(&sk_str).map_err(|_| Error::<T>::ParseSecretKeyError)?;

	
		let mut changed_status_trxs = Vec::new();
		for ((trx, index), status) in BridgeTrxStatusV2::<T>::iter()
			.filter(|(_, status)|
				status == &TrxStatus::Initial ||
				status == &TrxStatus::Generated ||
				status == &TrxStatus::Signed
			)
		{
			match (trx.clone(), status) {
				(TxOutV1::<T::AccountId>::Initialized(_), TrxStatus::Initial) => {
					match trx.clone().generate::<T>(node_url.as_str()) {
						Ok(generated_trx) => {
							changed_status_trxs.push(((generated_trx, index), TrxStatus::Generated, (trx.clone(), index),  None));
						}
						Err(e) => {
							debug::error!("failed to get latest block due to: {:?}", e);
						}
					}
				}
				(TxOutV1::<T::AccountId>::Created(_), TrxStatus::Generated) => {
					let author = <pallet_authorship::Module<T>>::author();
					// ensure current node has the right to sign a cross trade
					if NotaryKeys::<T>::get().contains(&author) {
						match trx.clone().sign::<T>(sk.clone(), author.clone()) {
							Ok(signed_trx) => {
								// ensure this transaction collects enough signatures
								let status = {
									if let TxOutV1::<T::AccountId>::Created(_) = signed_trx {
										TrxStatus::Generated
									} else {
										TrxStatus::Signed
									}
								};
								changed_status_trxs.push(((signed_trx, index), status, (trx.clone(), index), None));
							}
							Err(e) => {
								debug::error!("failed to get latest block due to: {:?}", e);
							}
						}
					}
				}
				(TxOutV1::<T::AccountId>::CompleteSigned(_), TrxStatus::Signed) => {
					match trx.clone().send::<T>(node_url.as_str()) {
						Ok(processing_trx) => {
							debug::error!(target: "bridge-eos", "bto.send {:?}", processing_trx);
							let trx_id = match processing_trx {
								TxOutV1::Sent { tx_id, .. } => Some(tx_id.clone()),
								_ => None,
							};
							debug::error!(target: "bridge-eos", "bto.send trx id {:?}, index: {:?}", trx_id.as_ref().unwrap().to_string(), index);
							changed_status_trxs.push(((processing_trx, index), TrxStatus::Processing, (trx.clone(), index), trx_id));
						}
						Err(e) => {
							debug::error!(target: "bridge-eos", "error happened while pushing transaction: {:?}", e);
							debug::error!(target: "bridge-eos", "bto.send error {:?}, index: {:?}", trx, index);
							match e {
								Error::<T>::SendingDuplicatedTransaction => {
									changed_status_trxs.push(((trx.clone(), index), TrxStatus::Processing, (trx, index), None));
								}
								Error::<T>::TransactionExpired => {
									changed_status_trxs.push(((trx.clone(), index), TrxStatus::Processing, (trx, index), None));
								}
								_ => {}
							}
						}
					}
				}
				_ => continue,
			}
		}

		if !changed_status_trxs.is_empty() {
			let call = Call::update_bridge_trx_status(changed_status_trxs.clone());
			match SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()) {
				Ok(_) => debug::error!(target: "bridge-eos", "submit unsigned trxs {:?}", ()),
				Err(e) => debug::error!("Failed to sent transaction due to: {:?}", e),
			}
		}

		Ok(())
	}

	fn convert_to_eos_asset<A, P, B>(
		bridge_asset: &BridgeAssetBalance<A, P, B>
	) -> Result<Asset, Error<T>>
		where
			P: AtLeast32Bit + Copy,
			B: AtLeast32Bit + Copy
	{
		let precision = bridge_asset.symbol.precision.saturated_into::<u8>();
		let symbol_str = core::str::from_utf8(&bridge_asset.symbol.symbol).map_err(|_| Error::<T>::ParseUtf8Error)?;
		let symbol_code = SymbolCode::try_from(symbol_str).map_err(|_| Error::<T>::ParseUtf8Error)?;
		let symbol = Symbol::new_with_code(precision, symbol_code);

		let amount = (bridge_asset.amount.saturated_into::<u128>() / (10u128.pow(12 - precision as u32))) as i64;

		Ok(Asset::new(amount, symbol))
	}

	fn get_offchain_storage(key: &[u8]) -> Result<String, Error<T>> {
		let value = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, key, ).ok_or(Error::<T>::NoLocalStorage)?;

		Ok(String::from_utf8(value).map_err(|_| Error::<T>::ParseUtf8Error)?)
	}
}

impl<T: Trait> BridgeAssetTo<T::AccountId, T::Precision, T::Balance> for Module<T> {
	type Error = crate::Error<T>;
	fn bridge_asset_to(target: Vec<u8>, bridge_asset: BridgeAssetBalance<T::AccountId, T::Precision, T::Balance>) -> Result<(), Self::Error> {
		let _ = Self::tx_transfer_to(target, bridge_asset)?;

		Ok(())
	}

	fn redeem(_: TokenSymbol, _: T::Balance, _: Vec<u8>) -> Result<(), Self::Error> { Ok(()) }
	fn stake(_: TokenSymbol, _: T::Balance, _: Vec<u8>) -> Result<(), Self::Error> { Ok(()) }
	fn unstake(_: TokenSymbol, _: T::Balance, _: Vec<u8>) -> Result<(), Self::Error> { Ok(()) }
}

#[allow(deprecated)]
impl<T: Trait> frame_support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		if let Call::update_bridge_trx_status(_) = call {
			let now_block = <frame_system::Module<T>>::block_number().saturated_into::<u64>();
			ValidTransaction::with_tag_prefix("BridgeEos")
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

#[allow(dead_code)]
mod weight_for {
	use frame_support::{traits::Get, weights::Weight};
	use super::Trait;

	/// cross_to_eos weight
	pub(crate) fn cross_to_eos<T: Trait>(memo_len: Weight) -> Weight {
		let db = T::DbWeight::get();
		db.writes(1) // put task to tx_out
			.saturating_add(db.reads(1)) // token exists or not
			.saturating_add(db.reads(1)) // get token
			.saturating_add(db.reads(1)) // get account asset
			.saturating_add(memo_len.saturating_add(10000)) // memo length
			.saturating_mul(1000)
	}
}
