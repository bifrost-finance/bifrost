// Copyright 2019 Liebi Technologies.
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

use core::str::FromStr;

use eos_chain::{Action, ActionReceipt, Asset, Checksum256, Digest, IncrementalMerkle, merkle, ProducerKey, ProducerSchedule, SignedBlockHeader, Symbol, SymbolCode};
use rstd::prelude::Vec;
use sr_primitives::traits::{Member, SaturatedConversion, SimpleArithmetic};
use sr_primitives::transaction_validity::{TransactionLongevity, TransactionValidity, UnknownTransaction, ValidTransaction};
use support::{decl_error, decl_event, decl_module, decl_storage, ensure, Parameter};
use system::ensure_root;
use system::offchain::SubmitUnsignedTransaction;

use bridge;
use node_primitives::{BridgeAssetBalance, BridgeAssetFrom, BridgeAssetTo};
use transaction::TxOut;

mod transaction;
mod mock;
mod tests;

#[derive(Debug)]
#[cfg(feature = "std")]
pub enum Error {
	LengthNotEqual(usize, usize), // (expected, actual)
	SignatureVerificationFailure,
	MerkleRootVerificationFailure,
	IncreMerkleError,
	ScheduleHashError,
	GetBlockIdFailure,
	CalculateMerkleError,
	EmptyActionMerklePaths,
	EosChainError(eos_chain::Error),
	TransactionError(transaction::Error),
}

pub type VersionId = u32;

pub trait Trait: system::Trait {
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;

	/// The units in which we record balances.
	type Balance: Member + Parameter + SimpleArithmetic + Default + Copy;

	/// The units in which we record asset precision.
	type Precision: Member + Parameter + SimpleArithmetic + Default + Copy;

	/// The units in which we record asset symbol.
	type Symbol: Member + Parameter + SimpleArithmetic + Default + Copy + AsRef<str>;

	/// Bridge asset from another blockchain.
	type BridgeAssetFrom: BridgeAssetFrom<Self::AccountId, Self::Precision, Self::Symbol, Self::Balance>;

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
	trait Store for Module<T: Trait> as BridgeEOS {
		ProducerSchedules: map VersionId => (Vec<ProducerKey>, Checksum256);

		PendingScheduleVersion: VersionId;

		BridgeTxOuts get(fn bridge_tx_outs): Vec<TxOut<T::Balance>>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

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

		fn change_schedule(
			origin,
			merkle: IncrementalMerkle,
			block_headers: Vec<SignedBlockHeader>,
			block_ids_list: Vec<Vec<Checksum256>>
		) {
			let _ = ensure_root(origin)?;

			ensure!(!block_headers.is_empty(), "The signed block headers cannot be empty.");
			ensure!(block_headers[0].block_header.new_producers.is_some(), "The producers list cannot be empty.");
			ensure!(block_ids_list.len() ==  block_headers.len(), "The block ids list cannot be empty.");

			let ps = block_headers[0].block_header.new_producers.as_ref().unwrap().clone(); // consider remove clone

			ensure!(PendingScheduleVersion::exists(), "PendingScheduleVersion has not been initialized.");

			let pending_schedule_version = PendingScheduleVersion::get();
			ensure!(ProducerSchedules::exists(pending_schedule_version), "ProducerSchedule has not been initialized.");

			ensure!(ps.version == pending_schedule_version + 1, "This is a wrong new producer lists due to wrong schedule version.");

			let schedule_hash = ps.schedule_hash();
			ensure!(schedule_hash.is_ok(), "Failed to calculate schedule hash value.");
			ProducerSchedules::insert(ps.version, (ps.producers, schedule_hash.unwrap()));
			PendingScheduleVersion::put(ps.version);

			#[cfg(feature = "std")]
			ensure!(Self::verify_block_headers(merkle, block_headers, block_ids_list).is_ok(), "Failed to verify block.");

			Self::deposit_event(Event::ChangeSchedule(pending_schedule_version, ps.version));
		}

		fn verify_block_headers_action_merkle(
			origin,
			block_header: SignedBlockHeader,
			actions: Vec<Action>,
			action_merkle_paths: Vec<Checksum256>,
			action_reciepts: Vec<ActionReceipt>
		) {
			let _ = ensure_root(origin)?;

			ensure!(
				!actions.is_empty() &&
				!action_reciepts.is_empty(),
				"actions or action receipts cannot be empty."
			);
			ensure!(action_reciepts.len() > 0, "action receipts cannot be empty.");
			ensure!(
				action_merkle_paths.len() == action_reciepts.len(),
				"the count of action merkle paths should be equal to action_reciepts."
			);

			#[cfg(feature = "std")]
			ensure!(
				Self::prove_action_merkle_root(&block_header, &actions, &action_merkle_paths, &action_reciepts).is_ok(),
				"action merkle verification error"
			);

			Self::deposit_event(Event::ProveAction);
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
		block_headers: Vec<SignedBlockHeader>,
		block_ids_list: Vec<Vec<Checksum256>>,
	) -> Result<(), Error> {
		ensure!(block_headers.len() == 15, Error::LengthNotEqual(15, block_headers.len()));
		ensure!(block_ids_list.len() == 15, Error::LengthNotEqual(15, block_ids_list.len()));

		for (block_header, block_ids) in block_headers.iter().zip(block_ids_list.iter()) {
			// calculate merkle root
			Self::calculate_block_header_merkle_root(&mut merkle, &block_header, &block_ids)?;

			// verify block header signature
			Self::verify_block_header_signature(block_header, &merkle.get_root())?;

			// append current block id
			let block_id = block_header.id().map_err(|_| Error::GetBlockIdFailure)?;
			merkle.append(block_id).map_err(|_| Error::IncreMerkleError)?;
		}
		Ok(())
	}

	#[cfg(feature = "std")]
	fn verify_block_header_signature(
		block_header: &SignedBlockHeader,
		expected_mroot: &Checksum256,
	) -> Result<(), Error> {
		let pending_schedule = match block_header.block_header.new_producers {
			Some(ref schedule) => schedule.clone(),
			None => {
				let pending_schedule_version = PendingScheduleVersion::get();
				let producers = ProducerSchedules::get(pending_schedule_version).0;
				ProducerSchedule::new(pending_schedule_version, producers)
			}
		};
		let producer = block_header.block_header.producer;
		let pk = pending_schedule.get_producer_key(producer);

		let pending_schedule_hash = ProducerSchedules::get(pending_schedule.version).1;

		block_header.verify(*expected_mroot, pending_schedule_hash, pk).map_err(|_| {
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

	#[cfg(feature = "std")]
	fn prove_action_merkle_root(
		block_header: &SignedBlockHeader,
		actions: &[Action],
		action_merkle_paths: &[Checksum256],
		action_reciepts: &[ActionReceipt],
	) -> Result<(), Error> {
		if action_merkle_paths.is_empty() || action_merkle_paths.len() != action_reciepts.len() {
			return Err(Error::MerkleRootVerificationFailure);
		}

		// compare action hash to act_digest in action receipt
		for action in actions {
			let action_hash = action.digest().map_err(|_| Error::CalculateMerkleError)?;
			let mut equal = false;
			for receipt in action_reciepts {
				if action_hash == receipt.act_digest {
					equal = true;
					break;
				}
			}
			if !equal { return Err(Error::MerkleRootVerificationFailure); }
		}

		// verify action hash with action receipts digest
		let mut actual_action_merkle_paths = Vec::with_capacity(action_merkle_paths.len());
		for receipt in action_reciepts.iter() {
			let path = receipt.digest().map_err(|_| Error::CalculateMerkleError)?;
			actual_action_merkle_paths.push(path);
		}

		// verify action merkle path
		let actual_action_mroot = merkle(actual_action_merkle_paths.to_vec()).map_err(|_| Error::CalculateMerkleError)?;
		let expected_action_mroot = merkle(action_merkle_paths.to_vec()).map_err(|_| Error::CalculateMerkleError)?;

		match (
			actual_action_mroot == expected_action_mroot,
			actual_action_mroot == block_header.block_header.action_mroot
		) {
			(true, true) => Ok(()),
			_ => Err(Error::MerkleRootVerificationFailure),
		}
	}

	fn tx_can_sign() -> bool {
		unimplemented!();
	}

	fn tx_is_signed() -> bool {
		unimplemented!();
	}

	/// generate transaction for transfer amount to
	#[cfg(feature = "std")]
	fn tx_transfer_to<P, S, B>(
		raw_to: Vec<u8>,
		bridge_asset: BridgeAssetBalance<P, S, B>,
	) -> Result<TxOut<T::Balance>, Error>
		where
			P: SimpleArithmetic,
			S: AsRef<str>,
			B: SimpleArithmetic,
	{
		let raw_from: Vec<u8> = Vec::new();

		let amount = Self::convert_to_eos_asset::<P, S, B>(bridge_asset)?;
		let mut tx_out = TxOut::genrate_transfer(raw_from, raw_to, amount)
			.map_err(Error::TransactionError)?;

		if Self::tx_can_sign() && !Self::tx_is_signed() {
			tx_out = tx_out.sign().map_err(Error::TransactionError)?;
		}

		<BridgeTxOuts<T>>::append([tx_out.clone()].into_iter());

		Ok(tx_out)
	}

	fn send_tx_result() {
		unimplemented!();
	}

	#[cfg(feature = "std")]
	fn offchain(now_block: T::BlockNumber) {
		<BridgeTxOuts<T>>::mutate(|bridge_tx_outs| {
			for bto in bridge_tx_outs.iter_mut() {
				if Self::tx_can_sign() && !Self::tx_is_signed() {
					*bto = bto.sign().unwrap();
				}

				if bto.reach_threshold() {
					*bto = bto.send().unwrap();
				}
			}
		});
	}

	fn convert_to_eos_asset<P, S, B>(
		bridge_asset: BridgeAssetBalance<P, S, B>
	) -> Result<Asset, Error>
		where
			P: SimpleArithmetic,
			S: AsRef<str>,
			B: SimpleArithmetic
	{
		let precision = bridge_asset.symbol.precision.saturated_into::<u8>();
		let code = SymbolCode::from_str(bridge_asset.symbol.symbol.as_ref())
			.map_err(|err| Error::EosChainError(err.into()))?;
		let symbol = Symbol::new_with_code(precision, code);
		let amount = (bridge_asset.amount.saturated_into::<u128>() / (10u128.pow(12 - precision as u32))) as i64;

		Ok(Asset::new(amount, symbol))
	}
}

impl<T: Trait> BridgeAssetTo<T::Precision, T::Symbol, T::Balance> for Module<T> {
	fn bridge_asset_to(target: Vec<u8>, bridge_asset: BridgeAssetBalance<T::Precision, T::Symbol, T::Balance>) {
		#[cfg(feature = "std")]
		Self::tx_transfer_to(target, bridge_asset);
	}
}
