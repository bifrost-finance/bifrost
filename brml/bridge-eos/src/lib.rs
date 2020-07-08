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
	traits::{Member, SaturatedConversion, AtLeast32Bit, MaybeSerializeDeserialize},
	transaction_validity::{
		InvalidTransaction, TransactionLongevity, TransactionPriority,
		TransactionValidity, ValidTransaction, TransactionSource
	},
};
use frame_support::traits::{Get};
use frame_support::{
	decl_event, decl_module, decl_storage, decl_error, debug, ensure, Parameter,
	dispatch::{DispatchResult, DispatchError},
	weights::{DispatchClass, Weight, Pays}
};
use frame_system::{
	self as system, ensure_root, ensure_none, ensure_signed,
	offchain::{SubmitTransaction, SendTransactionTypes}
};

use node_primitives::{
	AssetTrait, BridgeAssetBalance, BridgeAssetFrom,
	BridgeAssetTo, BridgeAssetSymbol, BlockchainType, TokenSymbol,
};
use transaction::TxOut;
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
		SendTransactionSuccess,
		SendTransactionFailure,
		GrantedCrossChainPrivilege(AccountId),
		RemovedCrossChainPrivilege(AccountId),
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as BridgeEos {
		/// The current set of notary keys that may send bridge transactions to Eos chain.
		NotaryKeys get(fn notary_keys) config(): Vec<T::AccountId>;

		/// Config to enable/disable this runtime
		BridgeEnable get(fn is_bridge_enable): bool = true;

		/// Eos producer list and hash which in specific version id
		ProducerSchedules: map hasher(blake2_128_concat) VersionId => (Vec<ProducerAuthority>, Checksum256);

		/// Initialize a producer schedule while starting a node.
		InitializeSchedule get(fn producer_schedule): ProducerAuthoritySchedule;

		/// Save all unique transactions
		/// Every transaction has different action receipt, but can have the same action
		BridgeActionReceipt: map hasher(blake2_128_concat) ActionReceipt => Action;

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
		#[weight = (0, DispatchClass::Normal, Pays::No)]
		fn change_schedule(
			origin,
			legacy_schedule_hash: Checksum256,
			new_schedule: ProducerAuthoritySchedule,
			merkle: IncrementalMerkle,
			block_headers: Vec<SignedBlockHeader>,
			block_ids_list: Vec<Vec<Checksum256>>
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(CrossChainPrivilege::<T>::get(&origin), DispatchError::Other("You're not permitted to execute this call."));

			ensure!(BridgeEnable::get(), DispatchError::Other("This call is not enabled now!"));
			ensure!(!block_headers.is_empty(), DispatchError::Other("The signed block headers cannot be empty."));
			ensure!(block_headers[0].block_header.new_producers.is_some(), DispatchError::Other("The producers list cannot be empty."));
			ensure!(block_ids_list.len() == block_headers.len(), DispatchError::Other("The block ids list cannot be empty."));
			ensure!(block_ids_list.len() == 15, DispatchError::Other("The length of signed block headers must be 15."));
			ensure!(block_ids_list[0].is_empty(), DispatchError::Other("The first block ids must be empty."));
			ensure!(block_ids_list[1].len() ==  10, DispatchError::Other("The rest of block ids length must be 10."));

			let legacy_pending_schedule = block_headers[0].block_header.new_producers.as_ref();
			let legacy_pending_schedule_hash = legacy_pending_schedule.and_then(|ps| ps.schedule_hash().ok())
				.ok_or(DispatchError::Other("Failed to calculate legacy schedule hash value."))?;
			ensure!(legacy_pending_schedule_hash == legacy_schedule_hash, "invalid producers schedule");

			ensure!(PendingScheduleVersion::exists(), DispatchError::Other("PendingScheduleVersion has not been initialized."));

			let current_schedule_version = PendingScheduleVersion::get();

			let (schedule_hash, producer_schedule) = {
				let schedule_hash = new_schedule.schedule_hash().map_err(|_| DispatchError::Other("Failed to calculate schedule hash value."))?;
				(schedule_hash, new_schedule)
			};

			ensure!(
				Self::verify_block_headers(merkle, &schedule_hash, &producer_schedule, &block_headers, block_ids_list).is_ok(),
				"Failed to verify block."
			);

			// if verification is successful, save the new producers schedule.
			ProducerSchedules::insert(producer_schedule.version, (&producer_schedule.producers, schedule_hash));
			PendingScheduleVersion::put(producer_schedule.version);

			Self::deposit_event(RawEvent::ChangeSchedule(current_schedule_version, producer_schedule.version));

			Ok(())
		}

		#[weight = (0, DispatchClass::Normal, Pays::No)]
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
			ensure!(CrossChainPrivilege::<T>::get(&origin), DispatchError::Other("You're not permitted to execute this call."));

			// ensure this transaction is unique, and ensure no duplicated transaction
			ensure!(BridgeActionReceipt::get(&action_receipt).ne(&action), "This is a duplicated transaction");

			// ensure action is what we want
			ensure!(action.name == ACTION_NAMES[0], "This is an invalid action to Bifrost");

			ensure!(BridgeEnable::get(), "This call is not enable now!");
			ensure!(
				!block_headers.is_empty(),
				"The signed block headers cannot be empty."
			);
			ensure!(
				block_ids_list.len() ==  block_headers.len(),
				"The block ids list cannot be empty."
			);

			let action_hash = action.digest().map_err(|_| Error::<T>::ErrorOnCalculationActionHash)?;
			ensure!(
				action_hash == action_receipt.act_digest,
				"current action hash isn't equal to act_digest from action_receipt."
			);

			let leaf = action_receipt.digest().map_err(|_| Error::<T>::ErrorOnCalculationActionReceiptHash)?;

			let block_under_verification = &block_headers[0];
			ensure!(
				verify_proof(&action_merkle_paths, leaf, block_under_verification.block_header.action_mroot),
				"failed to prove action."
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

			ensure!(
				Self::verify_block_headers(merkle, &schedule_hash, &producer_schedule, &block_headers, block_ids_list).is_ok(),
				"Failed to verify blocks."
			);

			// save proves for this transaction
			BridgeActionReceipt::insert(&action_receipt, &action);

			Self::deposit_event(RawEvent::ProveAction);

			let action_transfer = Self::get_action_transfer_from_action(&action)?;

			let cross_account = BridgeContractAccount::get().0;
			// withdraw operation, Bifrost => EOS
			if cross_account == action_transfer.from.to_string().into_bytes() {
				match Self::transaction_from_bifrost_to_eos(trx_id, &action_transfer) {
					Ok(target) => Self::deposit_event(RawEvent::Withdraw(target, action_transfer.to.to_string().into_bytes())),
					Err(e) => {
						debug::info!("Bifrost => EOS failed due to {:?}", e);
						Self::deposit_event(RawEvent::WithdrawFail);
					}
				}
			}

			// deposit operation, EOS => Bifrost
			if cross_account == action_transfer.to.to_string().into_bytes() {
				match Self::transaction_from_eos_to_bifrost(&action_transfer) {
					Ok(target) => Self::deposit_event(RawEvent::Deposit(action_transfer.from.to_string().into_bytes(), target)),
					Err(e) => {
						debug::info!("EOS => Bifrost failed due to {:?}", e);
						Self::deposit_event(RawEvent::DepositFail);
					}
				}
			}

			Ok(())
		}

		#[weight = (0, DispatchClass::Normal, Pays::No)]
		fn bridge_tx_report(origin, tx_list: Vec<TxOut<T::AccountId>>) -> DispatchResult {
			ensure_none(origin)?;

			BridgeTxOuts::<T>::put(tx_list);

			Ok(())
		}

		#[weight = (weight_for::asset_redeem::<T>(memo.len() as Weight), DispatchClass::Normal)]
		fn asset_redeem(
			origin,
			to: Vec<u8>,
			token_symbol: TokenSymbol,
			#[compact] amount: T::Balance,
			memo: Vec<u8>
		) {
			let origin = system::ensure_signed(origin)?;
			let eos_amount = amount;

			// check vtoken id exist or not
			ensure!(T::AssetTrait::token_exists(token_symbol), "this token doesn't exist.");
			// ensure redeem EOS instead of any others tokens like vEOS, DOT, KSM etc
			ensure!(token_symbol == TokenSymbol::EOS, Error::<T>::InvalidTokenForTrade);

			let token = T::AssetTrait::get_token(token_symbol);
			let symbol_code = token.symbol;
			let symbol_precise = token.precision;

			let balance = T::AssetTrait::get_account_asset(token_symbol, &origin).balance;
			ensure!(symbol_precise <= 12, "symbol precise cannot bigger than 12.");
			let amount = amount.div(T::Balance::from(10u32.pow(12u32 - symbol_precise as u32)));
			ensure!(balance >= amount, "amount should be less than or equal to origin balance");

			let asset_symbol = BridgeAssetSymbol::new(BlockchainType::EOS, symbol_code, T::Precision::from(symbol_precise.into()));
			let bridge_asset = BridgeAssetBalance {
				symbol: asset_symbol,
				amount: eos_amount,
				memo,
				from: origin.clone(),
				token_symbol
			};

			if Self::bridge_asset_to(to, bridge_asset).is_ok() {
				debug::info!("sent transaction to EOS node.");
				Self::deposit_event(RawEvent::SendTransactionSuccess);
			} else {
				debug::warn!("failed to send transaction to EOS node.");
				Self::deposit_event(RawEvent::SendTransactionFailure);
			}
		}

		// Runs after every block.
		fn offchain_worker(now_block: T::BlockNumber) {
			debug::RuntimeLogger::init();

			// It's no nessesary to start offchain worker if no any task in queue
			if !BridgeTxOuts::<T>::get().is_empty() {
				// Only send messages if we are a potential validator.
				if sp_io::offchain::is_validator() {
					debug::info!(target: "bridge-eos", "Is validator at {:?}.", now_block);
					match Self::offchain(now_block) {
						Ok(_) => debug::info!("A offchain worker started."),
						Err(e) => debug::error!("A offchain worker got error: {:?}", e),
					}
				} else {
					debug::info!(target: "bridge-eos", "Skipping send tx at {:?}. Not a validator.",now_block)
				}
			} else {
				debug::info!("There's no offchain worker started.");
			}
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

	fn transaction_from_eos_to_bifrost(action_transfer: &ActionTransfer) -> Result<T::AccountId, Error<T>> {
		// check memo, example like "alice@bifrost:EOS", the formatter: {receiver}@{chain}:{token_symbol}
		let split_memo = action_transfer.memo.as_str().split(|c| c == '@' || c == ':').collect::<Vec<_>>();

		// the length should be 2, either 3.
		if split_memo.len().gt(&3) || split_memo.len().lt(&2) {
			return Err(Error::<T>::InvalidMemo);
		}

		// get account
		let account_data = Self::get_account_data(split_memo[0])?;
		let target = Self::into_account(account_data)?;

		#[allow(unused_variables)]
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

		let symbol = action_transfer.quantity.symbol;
		let symbol_code = symbol.code().to_string().into_bytes();
		let symbol_precise = symbol.precision();

		let token_balances = action_transfer.quantity.amount as usize;
		// todo, according convert price to save as vEOS
		let vtoken_balances = T::Balance::try_from(token_balances).map_err(|_| Error::<T>::ConvertBalanceError)?;

		// check vtoken id exists not not, if it doesn't exist, create a vtoken for it
		let token_symbol = match T::AssetTrait::asset_id_exists(&target, &symbol_code, symbol_precise.into()) {
			Some(id) => id,
			None => {
				let (id, _) = T::AssetTrait::asset_create(symbol_code, symbol_precise.into()).map_err(|_| Error::<T>::TokenNotExist)?;
				let token_symbol: TokenSymbol = id.into();
				token_symbol
			}
		};

		// issue asset to target
		T::AssetTrait::asset_issue(token_symbol, target.clone(), vtoken_balances);

		Ok(target)
	}

	fn transaction_from_bifrost_to_eos(pending_trx_id: Checksum256, action_transfer: &ActionTransfer) -> Result<T::AccountId, Error<T>> {
		let bridge_tx_outs = BridgeTxOuts::<T>::get();

		for trx in bridge_tx_outs.iter() {
			match trx {
				TxOut::Processing{ tx_id, multi_sig_tx } if pending_trx_id.eq(tx_id) => {
					let target = &multi_sig_tx.from;
					let token_symbol = multi_sig_tx.token_symbol;

					let all_vtoken_balances = T::AssetTrait::get_account_asset(token_symbol, &target).balance;
					let token_balances = action_transfer.quantity.amount as usize;
					let vtoken_balances = T::Balance::try_from(token_balances).map_err(|_| Error::<T>::ConvertBalanceError)?;

					if all_vtoken_balances.lt(&vtoken_balances) {
						debug::warn!("origin account balance must be greater than or equal to the transfer amount.");
						return Err(Error::<T>::InsufficientBalance);
					}

					T::AssetTrait::asset_redeem(token_symbol, target.clone(), vtoken_balances);
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
		let decoded_ss58 = bs58::decode(receiver).into_vec().map_err(|_| Error::<T>::InvalidAccountId)?;

		if decoded_ss58.len() == 35 && decoded_ss58.first() == Some(&42) {
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
	) -> Result<TxOut<T::AccountId>, Error<T>>
		where
			P: AtLeast32Bit + Copy,
			B: AtLeast32Bit + Copy,
	{
		let (raw_from, threshold) = BridgeContractAccount::get();
		let memo = core::str::from_utf8(&bridge_asset.memo).map_err(|_| Error::<T>::ParseUtf8Error)?.to_string();
		let amount = Self::convert_to_eos_asset::<T::AccountId, P, B>(&bridge_asset)?;

		let tx_out = TxOut::<T::AccountId>::init(raw_from, raw_to, amount, threshold, &memo, bridge_asset.from, bridge_asset.token_symbol)?;
		BridgeTxOuts::<T>::append(&tx_out);

		Ok(tx_out)
	}

	fn offchain(_now_block: T::BlockNumber) -> Result<(), Error<T>> {
		//  avoid borrow checker issue if use has_change: bool
		let has_change = core::cell::Cell::new(false);

		let bridge_tx_outs = BridgeTxOuts::<T>::get();

		let node_url = Self::get_offchain_storage(EOS_NODE_URL)?;
		let sk_str = Self::get_offchain_storage(EOS_SECRET_KEY)?;

		let sk = SecretKey::from_wif(&sk_str).map_err(|_| Error::<T>::ParseSecretKeyError)?;

		let bridge_tx_outs = bridge_tx_outs.into_iter()
			.map(|bto| {
				match bto {
					// generate raw transactions
					TxOut::<T::AccountId>::Initial(_) => {
						match bto.clone().generate::<T>(node_url.as_str()) {
							Ok(generated_bto) => {
								has_change.set(true);
								debug::info!(target: "bridge-eos", "bto.generate {:?}",generated_bto);
								debug::info!("bto.generate");
								generated_bto
							}
							Err(e) => {
								debug::info!("failed to get latest block due to: {:?}", e);
								bto
							}
						}
					},
					_ => bto,
				}
			})
			.map(|bto| {
				match bto {
					TxOut::<T::AccountId>::Generated(_) => {
						let author = <pallet_authorship::Module<T>>::author();
						let mut ret = bto.clone();
						if let Some(_) = Self::local_authority_keys()
							.find(|key| *key == author.clone().into())
						{
							match bto.sign::<T>(sk.clone(), author) {
								Ok(signed_bto) => {
									has_change.set(true);
									debug::info!(target: "bridge-eos", "bto.sign {:?}", signed_bto);
									debug::info!("bto.sign with: {:?}", sk.to_string());
									ret = signed_bto;
								}
								Err(e) => debug::warn!("bto.sign with failure: {:?}", e),
							}
						}
						ret
					},
					_ => bto,
				}
			})
			.map(|bto| {
				match bto {
					TxOut::<T::AccountId>::Signed(_) => {
						match bto.clone().send::<T>(node_url.as_str()) {
							Ok(sent_bto) => {
								has_change.set(true);
								debug::info!(target: "bridge-eos", "bto.send {:?}", sent_bto,);
								debug::info!("bto.send");
								sent_bto
							}
							Err(e) => {
								debug::warn!("error happened while pushing transaction: {:?}", e);
								bto
							}
						}
					},
					_ => bto,
				}
			}).collect::<Vec<_>>();

		if has_change.get() {
			BridgeTxOuts::<T>::put(bridge_tx_outs.clone()); // update transaction list
			let call = Call::bridge_tx_report(bridge_tx_outs.clone());
			match SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()) {
				Ok(_) => debug::info!(target: "bridge-eos", "Call::bridge_tx_report {:?}", bridge_tx_outs),
				Err(e) => debug::warn!("submit transaction with failure: {:?}", e),
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

	fn local_authority_keys() -> impl Iterator<Item=T::AuthorityId> {
		let authorities = NotaryKeys::<T>::get();
		let mut local_keys = T::AuthorityId::all();
		local_keys.sort();

		authorities.into_iter()
			.enumerate()
			.filter_map(move |(_, authority)| {
				local_keys.binary_search(&authority.into())
					.ok()
					.map(|location| local_keys[location].clone())
			})
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
		if let Call::bridge_tx_report(_tx_list) = call {
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

	/// asset_redeem weight
	pub(crate) fn asset_redeem<T: Trait>(memo_len: Weight) -> Weight {
		let db = T::DbWeight::get();
		db.writes(1) // put task to tx_out
			.saturating_add(db.reads(1)) // token exists or not
			.saturating_add(db.reads(1)) // get token
			.saturating_add(db.reads(1)) // get account asset
			.saturating_add(memo_len.saturating_add(10000)) // memo length
	}
}
