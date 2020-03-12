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

extern crate alloc;

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::string::ToString;
use core::{convert::TryFrom, ops::Div, str::FromStr};

use codec::{Decode, Encode};
use eos_chain::{
	Action, ActionTransfer, ActionReceipt, Asset, Checksum256, Digest, IncrementalMerkle, ProducerKey,
	ProducerSchedule, SignedBlockHeader, Symbol, SymbolCode, Read, verify_proof, ActionName,
};
use eos_keys::secret::SecretKey;
use sp_std::prelude::*;
use sp_core::offchain::StorageKind;
use sp_runtime::{
	offchain::http,
	traits::{Member, SaturatedConversion, SimpleArithmetic},
	transaction_validity::{
		InvalidTransaction, TransactionLongevity, TransactionPriority,
		TransactionValidity, ValidTransaction
	},
};
use frame_support::{decl_event, decl_module, decl_storage, debug, ensure, Parameter};
use frame_system::{
	self as system,
	ensure_root, ensure_none,
	offchain::SubmitUnsignedTransaction
};

use node_primitives::{
	AssetRedeem, AssetTrait, BridgeAssetBalance, BridgeAssetFrom,
	BridgeAssetTo, BridgeAssetSymbol, BlockchainType, TokenType,
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

pub mod sr25519 {
	mod app_sr25519 {
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

	/// An bridge-eos keypair using sr25519 as its crypto.
	#[cfg(feature = "std")]
	pub type AuthorityPair = app_sr25519::Pair;

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

	/// An bridge-eos keypair using ed25519 as its crypto.
	#[cfg(feature = "std")]
	pub type AuthorityPair = app_ed25519::Pair;

	/// An bridge-eos signature using ed25519 as its crypto.
	pub type AuthoritySignature = app_ed25519::Signature;

	/// An bridge-eos identifier using ed25519 as its crypto.
	pub type AuthorityId = app_ed25519::Public;
}

const EOS_NODE_URL: &[u8] = b"EOS_NODE_URL";
const EOS_SECRET_KEY: &[u8] = b"EOS_SECRET_KEY";

#[derive(Debug)]
pub enum Error {
	LengthNotEqual(usize, usize), // (expected, actual)
	SignatureVerificationFailure,
	MerkleRootVerificationFailure,
	IncreMerkleError,
	ScheduleHashError,
	GetBlockIdFailure,
	CalculateMerkleError,
	EmptyActionMerklePaths,
	InvalidTxOutType,
	AlreadySignedByAuthor,
	ParseUtf8Error(core::str::Utf8Error),
	HexError(hex::FromHexError),
	EosChainError(eos_chain::Error),
	EosReadError(eos_chain::ReadError),
	#[cfg(feature = "std")]
	EosRpcError(eos_rpc::Error),
	EosKeysError(eos_keys::error::Error),
	NoLocalStorage,
	InvalidAccountId,
	ConvertBalanceError,
}

impl core::convert::From<eos_chain::symbol::ParseSymbolError> for Error {
	fn from(err: eos_chain::symbol::ParseSymbolError) -> Self {
		Self::EosChainError(eos_chain::Error::ParseSymbolError(err))
	}
}

pub type VersionId = u32;

pub trait Trait: pallet_authorship::Trait + assets::Trait {
	/// The identifier type for an authority.
	type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord
		+ From<<Self as frame_system::Trait>::AccountId>;

	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;

	/// The units in which we record balances.
	type Balance: Member + Parameter + SimpleArithmetic + Default + Copy;

	/// The units in which we record asset precision.
	type Precision: Member + Parameter + SimpleArithmetic + Default + Copy;

	/// Bridge asset from another blockchain.
	type BridgeAssetFrom: BridgeAssetFrom<Self::AccountId, Self::Precision, <Self as assets::Trait>::Balance>;

	type AssetTrait: AssetTrait<<Self as assets::Trait>::AssetId, Self::AccountId, <Self as assets::Trait>::Balance>;

	/// A dispatchable call type.
	type Call: From<Call<Self>>;

	/// A transaction submitter.
	type SubmitTransaction: SubmitUnsignedTransaction<Self, <Self as Trait>::Call>;
}

decl_event! {
	pub enum Event {
		InitSchedule(VersionId),
		ChangeSchedule(VersionId, VersionId), // ChangeSchedule(from, to)
		ProveAction,
		RelayBlock,
		TransactionFail,
		TransactionSuccess,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as BridgeEos {
		/// The current set of notary keys that may send bridge transactions to Eos chain.
		NotaryKeys get(fn notary_keys) config(): Vec<T::AccountId>;

		/// Config to enable/disable this runtime
		BridgeEnable get(fn is_bridge_enable): bool = true;

		/// Eos producer list and hash which in specific version id
		ProducerSchedules: map hasher(blake2_256) VersionId => (Vec<ProducerKey>, Checksum256);

		/// Initialize a producer schedule while starting a node.
		InitializeSchedule get(fn producer_schedule) config(): ProducerSchedule;

		/// Save all unique transactions
		/// Every transaction has different action receipt, but can have the same action
		BridgeActionReceipt: map hasher(blake2_256) ActionReceipt => Action;

		/// Current pending schedule version
		PendingScheduleVersion: VersionId;

		/// Transaction sent to Eos blockchain
		BridgeTxOuts get(fn bridge_tx_outs): Vec<TxOut<T::AccountId>>;

		/// Account where Eos bridge contract deployed, (Account, Signature threshold)
		BridgeContractAccount get(fn bridge_contract_account) config(): (Vec<u8>, u8);
	}
	add_extra_genesis {
		build(|config: &GenesisConfig<T>| {
			BridgeContractAccount::put(config.bridge_contract_account.clone());

			NotaryKeys::<T>::put(config.notary_keys.clone());

			let ps_version = config.producer_schedule.version;
			if !ProducerSchedules::exists(ps_version) {
				let producers = &config.producer_schedule.producers;
				let schedule_hash = config.producer_schedule.schedule_hash();
				assert!(schedule_hash.is_ok());
				ProducerSchedules::insert(ps_version, (producers, schedule_hash.unwrap()));
				PendingScheduleVersion::put(ps_version);
				debug::info!("producer schedule has been initialized");
			} else {
				debug::info!("producer schedule cannot be initialized twice");
			}
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		fn bridge_enable(origin, enable: bool) {
			ensure_root(origin)?;

			BridgeEnable::put(enable);
		}

		fn init_schedule(origin, ps: ProducerSchedule) {
			let _ = ensure_root(origin)?;

			ensure!(!ProducerSchedules::exists(ps.version), "ProducerSchedule has been initialized.");
			ensure!(!PendingScheduleVersion::exists(), "PendingScheduleVersion has been initialized.");
			let schedule_hash = ps.schedule_hash();
			ensure!(schedule_hash.is_ok(), "Failed to calculate schedule hash value.");

			// calculate schedule hash just one time, instead of calculating it multiple times.
			ProducerSchedules::insert(ps.version, (ps.producers, schedule_hash.unwrap()));
			PendingScheduleVersion::put(ps.version);

			Self::deposit_event(Event::InitSchedule(ps.version));
		}

		fn set_contract_accounts(origin, account: Vec<u8>, threthold: u8) {
			let _ = ensure_root(origin)?;
			BridgeContractAccount::put((account, threthold));
		}

		// 1. block_headers length must be 15.
		// 2. the first block_header's new_producers cannot be none.
		// 3. compare current schedules version with pending_schedules'.
		// 4. verify incoming 180 block_headers to prove this new_producers list is valid.
		// 5. save the new_producers list.
		fn change_schedule(
			origin,
			merkle: IncrementalMerkle,
			block_headers: Vec<SignedBlockHeader>,
			block_ids_list: Vec<Vec<Checksum256>>
		) {
			let _ = ensure_root(origin)?;

			ensure!(BridgeEnable::get(), "This call is not enable now!");
			ensure!(!block_headers.is_empty(), "The signed block headers cannot be empty.");
			ensure!(block_headers[0].block_header.new_producers.is_some(), "The producers list cannot be empty.");
			ensure!(block_ids_list.len() == block_headers.len(), "The block ids list cannot be empty.");
			ensure!(block_ids_list.len() == 15, "The length of signed block headers must be 15.");
			ensure!(block_ids_list[0].is_empty(), "The first block ids must be empty.");
			ensure!(block_ids_list[1].len() ==  10, "The rest of block ids length must be 10.");

			let pending_schedule = block_headers[0].block_header.new_producers.as_ref().unwrap();

			ensure!(PendingScheduleVersion::exists(), "PendingScheduleVersion has not been initialized.");

			let current_schedule_version = PendingScheduleVersion::get();
			ensure!(ProducerSchedules::exists(current_schedule_version), "ProducerSchedule has not been initialized.");

			ensure!(current_schedule_version + 1 == pending_schedule.version, "This is a wrong new producer lists due to wrong schedule version.");

			let schedule_hash_and_producer_schedule = Self::get_schedule_hash_and_public_key(block_headers[0].block_header.new_producers.as_ref());
			ensure!(schedule_hash_and_producer_schedule.is_ok(), "Failed to calculate schedule hash value.");
			let (schedule_hash, producer_schedule) = schedule_hash_and_producer_schedule.unwrap();

			ensure!(
				Self::verify_block_headers(merkle, &schedule_hash, &producer_schedule, &block_headers, block_ids_list).is_ok(),
				"Failed to verify block."
			);

			// if verification is successful, save the new producers schedule.
			ProducerSchedules::insert(pending_schedule.version, (&pending_schedule.producers, schedule_hash));
			PendingScheduleVersion::put(pending_schedule.version);

			Self::deposit_event(Event::ChangeSchedule(current_schedule_version, pending_schedule.version));
		}

		fn prove_action(
			origin,
			target: T::AccountId,
			action: Action,
			action_receipt: ActionReceipt,
			action_merkle_paths: Vec<Checksum256>,
			merkle: IncrementalMerkle,
			block_headers: Vec<SignedBlockHeader>,
			block_ids_list: Vec<Vec<Checksum256>>
		) {
			let _ = ensure_root(origin)?;

			// ensure this transaction is unique
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

			// check memo, example like "alice@bifrost:EOS", the formatter: {receiver}@{chain}:{memo}
			let act_transfer = Self::get_action_transfer_from_action(&action);
			ensure!(act_transfer.is_ok(), "Cannot read transfer action from data.");
			let act_transfer = act_transfer.unwrap();
			let split_memo = act_transfer.memo.as_str().split(|c| c == '@' || c == ':').collect::<Vec<_>>();
			ensure!(split_memo.len() == 2 || split_memo.len() == 3, "This is an invalid memo.");

			// get account
			let account_data = Self::get_account_data(split_memo[0]);
			ensure!(account_data.is_ok(), "This is an invalid account.");
			let account = Self::into_account(account_data.unwrap());
			ensure!(account.is_ok(), "Cannot find this account in bifrost.");
			let target = account.unwrap();

			let action_hash = action.digest();
			ensure!(action_hash.is_ok(), "failed to calculate action digest.");
			let action_hash = action_hash.unwrap();
			ensure!(
				action_hash == action_receipt.act_digest,
				"current action hash isn't equal to act_digest from action_receipt."
			);

			let leaf = action_receipt.digest();
			ensure!(leaf.is_ok(), "failed to calculate action digest.");
			let leaf = leaf.unwrap();

			let block_under_verification = &block_headers[0];
			ensure!(
				verify_proof(&action_merkle_paths, leaf, block_under_verification.block_header.action_mroot),
				"failed to prove action."
			);

			let schedule_hash_and_producer_schedule = Self::get_schedule_hash_and_public_key(block_headers[0].block_header.new_producers.as_ref());
			ensure!(schedule_hash_and_producer_schedule.is_ok(), "Failed to calculate schedule hash value.");
			let (schedule_hash, producer_schedule) = schedule_hash_and_producer_schedule.unwrap();

			ensure!(
				Self::verify_block_headers(merkle, &schedule_hash, &producer_schedule, &block_headers, block_ids_list).is_ok(),
				"Failed to verify blocks."
			);

			// save proves for this transaction
			BridgeActionReceipt::insert(&action_receipt, &action);

			Self::deposit_event(Event::ProveAction);

			// withdraw or deposit
			Self::map_assets_by_action(target, &act_transfer);
		}

		fn bridge_tx_report(origin, tx_list: Vec<TxOut<T::AccountId>>) {
			ensure_none(origin)?;

			BridgeTxOuts::<T>::put(tx_list);
		}

		fn asset_redeem(
			origin,
			to: Vec<u8>,
			#[compact] amount: <T as assets::Trait>::Balance,
			vtoken_id: T::AssetId
		) {
			let origin = system::ensure_signed(origin)?;
			let origin_account = (vtoken_id, TokenType::VToken, &origin);
			let eos_amount = amount;

			// check vtoken id exist or not
			ensure!(T::AssetTrait::token_exists(vtoken_id), "this token doesn't exist.");

			let token = <assets::Tokens<T>>::get(vtoken_id).token;
			let symbol_code = token.symbol;
			let symbol_precise = token.precision;

			let balance = <assets::AccountAssets<T>>::get(origin_account).balance;
			ensure!(symbol_precise <= 12, "symbol precise cannot bigger than 12.");
			let amount = amount.div(<T as assets::Trait>::Balance::from(10u32.pow(12u32 - symbol_precise as u32)));
			ensure!(balance >= amount, "amount should be less than or equal to origin balance");

			let asset_symbol = BridgeAssetSymbol::new(BlockchainType::EOS, symbol_code, T::Precision::from(symbol_precise.into()));
			let bridge_asset = BridgeAssetBalance {
				symbol: asset_symbol,
				amount: eos_amount
			};
			if Self::bridge_asset_to(to, bridge_asset).is_ok() {
				T::AssetTrait::asset_redeem(vtoken_id, TokenType::VToken, origin, amount);
				Self::deposit_event(Event::TransactionSuccess);
			} else {
				Self::deposit_event(Event::TransactionFail);
			}
		}

		// Runs after every block.
		fn offchain_worker(now_block: T::BlockNumber) {
			debug::RuntimeLogger::init();

			// let node_info = "http://127.0.0.1:8888/v1/chain/get_info";
			// get eos node info
//			Self::get_eos_node_info(node_info);

			// Only send messages if we are a potential validator.
			if sp_io::offchain::is_validator() {
				debug::info!(
					target: "bridge-eos",
					"Is validator at {:?}.",
					now_block,
				);
				Self::offchain(now_block);
			} else {
				debug::info!(
					target: "bridge-eos",
					"Skipping send tx at {:?}. Not a validator.",
					now_block,
				)
			}
		}
	}
}

impl<T: Trait> Module<T> {
	fn verify_block_headers(
		mut merkle: IncrementalMerkle,
		schedule_hash: &Checksum256,
		producer_schedule: &ProducerSchedule,
		block_headers: &[SignedBlockHeader],
		block_ids_list: Vec<Vec<Checksum256>>,
	) -> Result<(), Error> {
		ensure!(block_headers.len() == 15, Error::LengthNotEqual(15, block_headers.len()));
		ensure!(block_ids_list.len() == 15, Error::LengthNotEqual(15, block_ids_list.len()));

		for (block_header, block_ids) in block_headers.iter().zip(block_ids_list.iter()) {
			// calculate merkle root
			Self::calculate_block_header_merkle_root(&mut merkle, &block_header, &block_ids)?;

			// verify block header signature
			Self::verify_block_header_signature(schedule_hash, producer_schedule, block_header, &merkle.get_root())?;

			// append current block id
			let block_id = block_header.id().map_err(|_| Error::GetBlockIdFailure)?;
			merkle.append(block_id).map_err(|_| Error::IncreMerkleError)?;
		}
		Ok(())
	}

	fn verify_block_header_signature(
		schedule_hash: &Checksum256,
		producer_schedule: &ProducerSchedule,
		block_header: &SignedBlockHeader,
		expected_mroot: &Checksum256,
	) -> Result<(), Error> {
		let pk = producer_schedule.get_producer_key(block_header.block_header.producer);
		block_header.verify(*expected_mroot, *schedule_hash, pk).map_err(|_| {
			Error::SignatureVerificationFailure
		})?;

		Ok(())
	}

	fn calculate_block_header_merkle_root(
		merkle: &mut IncrementalMerkle,
		block_header: &SignedBlockHeader,
		block_ids: &[Checksum256],
	) -> Result<(), Error> {
		for id in block_ids {
			merkle.append(*id).map_err(|_| Error::IncreMerkleError)?;
		}

		// append previous block id
		merkle.append(block_header.block_header.previous).map_err(|_| Error::IncreMerkleError)?;
		Ok(())
	}

	fn get_schedule_hash_and_public_key<'a>(
		new_producers: Option<&'a ProducerSchedule>
	) -> Result<(Checksum256, Cow<'a, ProducerSchedule>), Error> {
		let producer_schedule = match new_producers {
			Some(producers) => Cow::Borrowed(producers), // use Cow to avoid cloning
			None => {
				let schedule_version = PendingScheduleVersion::get();
				let producers = ProducerSchedules::get(schedule_version).0;
				let ps = ProducerSchedule::new(schedule_version, producers);
                Cow::Owned(ps)
			}
		};

		let schedule_hash = producer_schedule.schedule_hash().map_err(|_| Error::ScheduleHashError)?;

		Ok((schedule_hash, producer_schedule))
	}

	fn get_action_transfer_from_action(act: &Action) -> Result<ActionTransfer, Error> {
		let action_transfer = ActionTransfer::read(&act.data, &mut 0).map_err(|e| {
			crate::Error::EosChainError(eos_chain::Error::BytesReadError(e))
		})?;

		Ok(action_transfer)
	}

	fn map_assets_by_action(target: T::AccountId, action_transfer: &ActionTransfer) -> Result<(), Error> {
		let symbol = action_transfer.quantity.symbol;
		let symbol_code = symbol.code().to_string().into_bytes();
		let symbol_precise = symbol.precision();

		let token_balances = action_transfer.quantity.amount as usize;
		let vtoken_balances = <T as assets::Trait>::Balance::try_from(token_balances).map_err(|_| Error::ConvertBalanceError)?;

		// check vtoken id exists not not, if it doesn't exist, create a vtoken for it
		let vtoken_id = match T::AssetTrait::asset_id_exists(&target, &symbol_code, symbol_precise.into()) {
			Some(id) => id,
			None => {
				T::AssetTrait::asset_create(symbol_code, symbol_precise.into()).0
			}
		};

		// withdraw
		let from = action_transfer.from.to_string().as_bytes().to_vec();
		if BridgeContractAccount::get().0 == from {
			let origin_account = (vtoken_id, TokenType::VToken, &target);
			let vtoken_balances = <assets::AccountAssets<T>>::get(origin_account).balance;
			if vtoken_balances.lt(&vtoken_balances) {
				debug::info!("origin account balance must be greater than or equal to the transfer amount.");
				return Ok(())
			}

			T::AssetTrait::asset_redeem(vtoken_id, TokenType::VToken, target, vtoken_balances);
			return Ok(());
		}

		// deposit
		let to = action_transfer.to.to_string().as_bytes().to_vec();
		if BridgeContractAccount::get().0 == to {
			T::AssetTrait::asset_issue(vtoken_id, TokenType::VToken, target, vtoken_balances);
			return Ok(());
		}

		Ok(())
	}

	/// check receiver account format
	/// https://github.com/paritytech/substrate/wiki/External-Address-Format-(SS58)
	fn get_account_data(receiver: &str) -> Result<[u8; 32], Error> {
		let decoded_ss58 = bs58::decode(receiver).into_vec().map_err(|_| crate::Error::InvalidAccountId)?;

		if decoded_ss58.len() == 35 && decoded_ss58.first() == Some(&42) {
			let mut data = [0u8; 32];
			data.copy_from_slice(&decoded_ss58[1..33]);
			Ok(data)
		} else {
			Err(Error::InvalidAccountId)
		}
	}

	fn into_account(data: [u8; 32]) -> Result<T::AccountId, Error> {
		T::AccountId::decode(&mut &data[..]).map_err(|_| Error::InvalidAccountId)
	}

	/// generate transaction for transfer amount to
	fn tx_transfer_to<P, B>(
		raw_to: Vec<u8>,
		bridge_asset: BridgeAssetBalance<P, B>,
	) -> Result<TxOut<T::AccountId>, Error>
		where
			P: SimpleArithmetic,
			B: SimpleArithmetic,
	{
		let (raw_from, threshold) = BridgeContractAccount::get();
		let amount = Self::convert_to_eos_asset::<P, B>(bridge_asset)?;
		let tx_out = TxOut::<T::AccountId>::init(raw_from, raw_to, amount, threshold)?;
		BridgeTxOuts::<T>::append([&tx_out].into_iter());

		Ok(tx_out)
	}

	fn offchain(_now_block: T::BlockNumber) {
		//  avoid borrow checker issue if use has_change: bool
		let has_change = core::cell::Cell::new(false);

		let bridge_tx_outs = BridgeTxOuts::<T>::get();

		let node_url = Self::get_offchain_storage(EOS_NODE_URL);
		let sk_str = Self::get_offchain_storage(EOS_SECRET_KEY);
		if node_url.is_err() || sk_str.is_err() {
			return;
		}
		let node_url = node_url.unwrap();
		let sk = SecretKey::from_wif(sk_str.unwrap().as_str());
		if sk.is_err() {
			return;
		}
		let sk = sk.unwrap();

		let bridge_tx_outs = bridge_tx_outs.into_iter()
			.map(|bto| {
				match bto {
					// generate raw transactions
					#[cfg(feature = "std")]
					TxOut::<T::AccountId>::Initial(_) => {
						if let Ok(generated_bto) = bto.clone().generate(node_url.as_str()) {
							has_change.set(true);
							debug::info!(
								target: "bridge-eos",
								"bto.generate {:?}",
								generated_bto,
							);
							#[cfg(feature = "std")]
							dbg!("bto.generate");
							generated_bto
						} else {
							bto
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
							if let Ok(signed_bto) = bto.sign(sk.clone(), author) {
								has_change.set(true);
								debug::info!(
									target: "bridge-eos",
									"bto.sign {:?}",
									signed_bto,
								);
								#[cfg(feature = "std")]
								dbg!("bto.sign");
								ret = signed_bto;
							}
						}
						ret
					},
					_ => bto,
				}
			})
			.map(|bto| {
				match bto {
					#[cfg(feature = "std")]
					TxOut::<T::AccountId>::Signed(_) => {
						match bto.clone().send(node_url.as_str()) {
							Ok(sent_bto) => {
								has_change.set(true);
								debug::info!(
									target: "bridge-eos",
									"bto.send {:?}",
									sent_bto,
								);
								#[cfg(feature = "std")]
								dbg!("bto.send");
								sent_bto
							}
							Err(e) => {
								debug::info!("error while sending: {:?}", e);
								bto
							}
						}
					},
					_ => bto,
				}
			}).collect::<Vec<_>>();

		if has_change.get() {
			#[cfg(test)] // for testing
			BridgeTxOuts::<T>::put(bridge_tx_outs.clone());
			T::SubmitTransaction::submit_unsigned(Call::bridge_tx_report(bridge_tx_outs.clone())).unwrap();
			debug::info!(
				target: "bridge-eos",
				"Call::bridge_tx_report {:?}",
				bridge_tx_outs,
			);
		}
	}

	fn convert_to_eos_asset<P, B>(
		bridge_asset: BridgeAssetBalance<P, B>
	) -> Result<Asset, Error>
		where
			P: SimpleArithmetic,
			B: SimpleArithmetic
	{
		let precision = bridge_asset.symbol.precision.saturated_into::<u8>();
		let symbol_str = core::str::from_utf8(&bridge_asset.symbol.symbol)
			.map_err(Error::ParseUtf8Error)?;
		let symbol_code = SymbolCode::try_from(symbol_str)?;
		let symbol = Symbol::new_with_code(precision, symbol_code);
		let amount = (bridge_asset.amount.saturated_into::<u128>() / (10u128.pow(12 - precision as u32))) as i64;

		Ok(Asset::new(amount, symbol))
	}

	fn get_offchain_storage(key: &[u8]) -> Result<String, Error> {
		let value = sp_io::offchain::local_storage_get(
			StorageKind::PERSISTENT,
			key,
		).ok_or(Error::NoLocalStorage)?;
		Ok(String::from_utf8(value).map_err(|e| Error::ParseUtf8Error(e.utf8_error()))?)
	}

	fn local_authority_keys() -> impl Iterator<Item=T::AuthorityId> {
		let authorities = NotaryKeys::<T>::get();
		let mut local_keys = T::AuthorityId::all();
		local_keys.sort();

		authorities.into_iter()
			.enumerate()
			.filter_map(move |(index, authority)| {
				local_keys.binary_search(&authority.into())
					.ok()
					.map(|location| local_keys[location].clone())
			})
	}

	fn get_eos_node_info(node_api: &str) {
		let pending = http::Request::get(node_api).send().unwrap();
		let response = pending.wait();
		debug::info!("response is: {:?}", response);

		if response.is_ok() {
			debug::warn!("no response received.");
			return;
		}

		let body = response.unwrap().body();
		let body_bytes = body.collect::<Vec<u8>>();
		debug::info!("just body is: {:?}", body_bytes);

		// still cannot use serde_json in no_std due to compilation issue
//		let node_info: Result<serde_json::Value, _> = serde_json::from_slice(&body_bytes);
//		dbg!(&node_info);
	}

	fn get_eos_block(node_api: &str) {
		todo!();
	}

	fn push_transaction_to_eos(node_api: &str) {
		todo!();
	}
}

impl<T: Trait> BridgeAssetTo<T::Precision, <T as assets::Trait>::Balance> for Module<T> {
	type Error = crate::Error;
	fn bridge_asset_to(target: Vec<u8>, bridge_asset: BridgeAssetBalance<T::Precision, <T as assets::Trait>::Balance>) -> Result<(), Self::Error> {
		let _ = Self::tx_transfer_to(target, bridge_asset)?;

		Ok(())
	}
}

#[allow(deprecated)]
impl<T: Trait> frame_support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	fn validate_unsigned(call: &Self::Call) -> TransactionValidity {
		if let Call::bridge_tx_report(_tx_list) = call {
			let now_block = <frame_system::Module<T>>::block_number().saturated_into::<u64>();
			Ok(ValidTransaction {
				priority: TransactionPriority::max_value(),
				requires: vec![],
				provides: vec![(now_block).encode()],
				longevity: TransactionLongevity::max_value(),
				propagate: true,
			})
		} else {
			InvalidTransaction::Call.into()
		}
	}
}
