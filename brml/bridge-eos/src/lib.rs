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
use core::str::FromStr;

use codec::Encode;
use eos_chain::{
	Action, ActionTransfer, ActionReceipt, Asset, Checksum256, Digest, IncrementalMerkle, ProducerKey,
	ProducerSchedule, SignedBlockHeader, Symbol, Read, verify_proof, ActionName,
};
#[cfg(feature = "std")]
use eos_keys::secret::SecretKey;
use sp_std::prelude::*;
use sp_core::offchain::StorageKind;
use sp_runtime::{
	traits::{Member, SaturatedConversion, SimpleArithmetic},
	transaction_validity::{TransactionLongevity, TransactionValidity, UnknownTransaction, ValidTransaction},
};
use support::{decl_event, decl_module, decl_storage, debug, ensure, Parameter};
use system::{
	ensure_root, ensure_none,
	offchain::SubmitUnsignedTransaction
};

use node_primitives::{BridgeAssetBalance, BridgeAssetFrom, BridgeAssetTo};
use transaction::TxOut;

mod transaction;
mod mock;
mod tests;

lazy_static::lazy_static! {
	pub static ref ActionNames: [ActionName; 1] = {
		let name = ActionName::from_str("transfer").unwrap();
		[name]
	};
}

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
	ParseUtf8Error(core::str::Utf8Error),
	HexError(hex::FromHexError),
	EosChainError(eos_chain::Error),
	EosReadError(eos_chain::ReadError),
	#[cfg(feature = "std")]
	EosRpcError(eos_rpc::Error),
	#[cfg(feature = "std")]
	EosKeysError(eos_keys::error::Error),
	NoLocalStorage,
}

impl core::convert::From<eos_chain::symbol::ParseSymbolError> for Error {
	fn from(err: eos_chain::symbol::ParseSymbolError) -> Self {
		Self::EosChainError(eos_chain::Error::ParseSymbolError(err))
	}
}

pub type VersionId = u32;

pub trait Trait: system::Trait {
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;

	/// The units in which we record balances.
	type Balance: Member + Parameter + SimpleArithmetic + Default + Copy;

	/// The units in which we record asset precision.
	type Precision: Member + Parameter + SimpleArithmetic + Default + Copy;

	/// Bridge asset from another blockchain.
	type BridgeAssetFrom: BridgeAssetFrom<Self::AccountId, Self::Precision, Self::Balance>;

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
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as BridgeEos {
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
		BridgeTxOuts get(fn bridge_tx_outs): Vec<TxOut<T::Balance>>;

		/// Accounts where Eos bridge contract deployed
		BridgeContractAccounts get(fn bridge_contract_accounts): Vec<Vec<u8>>;
	}
	add_extra_genesis {
		build(|config: &GenesisConfig| {
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

		fn bridge_enable(origin) {
			ensure_root(origin)?;

			BridgeEnable::put(true);
		}

		fn bridge_disable(origin) {
			ensure_root(origin)?;

			BridgeEnable::put(false);
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

		fn set_contract_accounts(origin, account: Vec<Vec<u8>>) {
			let _ = ensure_root(origin)?;
			BridgeContractAccounts::put(account);
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

			#[cfg(feature = "std")]
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
			ensure!(action.name == ActionNames[0], "This is an invalid action to Bifrost");

			ensure!(BridgeEnable::get(), "This call is not enable now!");
			ensure!(
				!block_headers.is_empty(),
				"The signed block headers cannot be empty."
			);
			ensure!(
				block_ids_list.len() ==  block_headers.len(),
				"The block ids list cannot be empty."
			);

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

			#[cfg(feature = "std")]
			ensure!(
				Self::verify_block_headers(merkle, &schedule_hash, &producer_schedule, &block_headers, block_ids_list).is_ok(),
				"Failed to verify blocks."
			);

			// save proves for this transaction
			BridgeActionReceipt::insert(&action_receipt, &action);

			Self::deposit_event(Event::ProveAction);

			// withdraw or deposit
			Self::filter_account_by_action(&action);
		}

		fn tx_result(origin, block_num: T::BlockNumber) {
			ensure_none(origin)?;
			// TODO implement this function
		}

		// Runs after every block.
		fn offchain_worker(now_block: T::BlockNumber) {
			#[cfg(feature = "std")]
			Self::offchain(now_block);
		}
	}
}

impl<T: Trait> Module<T> {
	#[cfg(feature = "std")]
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

	#[cfg(feature = "std")]
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

	#[cfg(feature = "std")]
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

	fn filter_account_by_action(action: &Action) -> Result<(), Error> {
		let action_transfer = Self::get_action_transfer_from_action(&action)?;

		let from = action_transfer.from.to_string().as_bytes().to_vec();
		if BridgeContractAccounts::get().contains(&from) {
			todo!("withdraw");
			return Ok(());
		}

		let to = action_transfer.to.to_string().as_bytes().to_vec();
		if BridgeContractAccounts::get().contains(&to) {
			todo!("deposit");
			return Ok(());
		}

		Ok(())
	}

	fn tx_can_sign() -> bool {
		// TODO
		true
	}

	fn tx_is_signed() -> bool {
		// TODO
		false
	}

	/// generate transaction for transfer amount to
	#[cfg(feature = "std")]
	fn tx_transfer_to<P, B>(
		raw_to: Vec<u8>,
		bridge_asset: BridgeAssetBalance<P, B>,
	) -> Result<TxOut<T::Balance>, Error>
		where
			P: SimpleArithmetic,
			B: SimpleArithmetic,
	{
		let parse_offchain_storage = |key: &str| {
			let decode = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, key.as_bytes()).ok_or(Error::NoLocalStorage)?;
			let sigs_str = hex::decode(decode).map_err(Error::HexError)?;
			String::from_utf8(sigs_str).map_err(|e| Error::ParseUtf8Error(e.utf8_error()))
		};

		let node_url = parse_offchain_storage("EOS_NODE")?;
		let sk_str = parse_offchain_storage("EOS_KEY")?;
		let sk = SecretKey::from_wif(&sk_str).unwrap();

		let raw_from = vec![0x62, 0x69, 0x66, 0x72, 0x6F, 0x73, 0x74]; // bifrost
		let amount = Self::convert_to_eos_asset::<P, B>(bridge_asset)?;
		let mut tx_out = TxOut::generate_transfer(&node_url, raw_from, raw_to, amount)?;

		if Self::tx_can_sign() {
			tx_out = tx_out.sign(sk)?;
		}

		<BridgeTxOuts<T>>::append([&tx_out].into_iter());

		Ok(tx_out)
	}

	#[cfg(feature = "std")]
	fn offchain(now_block: T::BlockNumber) {
		// sign each transaction
		if Self::tx_can_sign() {
			// TODO
			let sk = SecretKey::from_wif("5HrPPFF2hq1X8ktBVfUVubeAmSaerRHwz2aGxGSUqvAuaNhR8a5").unwrap();
			<BridgeTxOuts<T>>::mutate(|bridge_tx_outs| {
				for bto in bridge_tx_outs.iter_mut().filter(|bto_filter| {
					match bto_filter {
						TxOut::Pending(_) => true,
						_ => false,
					}
				}) {
					if !bto.reach_threshold() && !Self::tx_is_signed() {
						*bto = bto.sign(sk).unwrap();
					}
				}
			});
		}

		// push each transaction to eos node
		let node_url: &str = "http://127.0.0.1:8888/";
		<BridgeTxOuts<T>>::mutate(|bridge_tx_outs| {
			for bto in bridge_tx_outs.iter_mut().filter(|bto_filter| {
				match bto_filter {
					TxOut::Pending(_) => true,
					_ => false,
				}
			}) {
				if bto.reach_threshold() {
					*bto = bto.send(node_url).unwrap();
				}
			}
		});
	}

	#[cfg(feature = "std")]
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
		let symbol = Symbol::from_str(format!("{},{}", precision, symbol_str).as_ref())
			.map_err(|err| Error::EosChainError(err.into()))?;
		let amount = (bridge_asset.amount.saturated_into::<u128>() / (10u128.pow(12 - precision as u32))) as i64;

		Ok(Asset::new(amount, symbol))
	}
}

impl<T: Trait> BridgeAssetTo<T::Precision, T::Balance> for Module<T> {
	fn bridge_asset_to(target: Vec<u8>, bridge_asset: BridgeAssetBalance<T::Precision, T::Balance>) {
		#[cfg(feature = "std")]
		Self::tx_transfer_to(target, bridge_asset);
	}
}

#[allow(deprecated)]
impl<T: Trait> support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	fn validate_unsigned(call: &Self::Call) -> TransactionValidity {
		match call {
			Call::tx_result(block_num) => {
				let now_block = <system::Module<T>>::block_number().saturated_into::<u64>();
				Ok(ValidTransaction {
					priority: 0,
					requires: vec![],
					provides: vec![(now_block).encode()],
					longevity: TransactionLongevity::max_value(),
					propagate: true,
				})
			},
			_ => UnknownTransaction::NoUnsignedValidator.into(),
		}
	}
}