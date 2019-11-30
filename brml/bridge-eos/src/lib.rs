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

use codec::{Decode, Encode};
use eos_chain::{Action, Checksum256, IncrementalMerkle, ProducerKey, ProducerSchedule, SignedBlockHeader};
use rstd::prelude::Vec;
use support::{decl_module, decl_storage, decl_event, ensure};
use system::ensure_root;

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
}

pub type VersionId = u32;

pub trait Trait: system::Trait {
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;
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
		ProducerSchedules get(fn producer_schedules): map VersionId => Vec<ProducerKey>;

		LatestScheduleVersion get(fn latest_schedule_version): VersionId;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		fn init_schedule(origin, ps: ProducerSchedule) {
			let _ = ensure_root(origin)?;

			ensure!(!ProducerSchedules::exists(ps.version), "ProducerSchedule has been initialized.");
			ensure!(!LatestScheduleVersion::exists(), "LatestScheduleVersion has been initialized.");
			ProducerSchedules::insert(ps.version, ps.producers);
			LatestScheduleVersion::put(ps.version);

			Self::deposit_event(Event::InitSchedule(ps.version));
		}

		fn change_schedule(
			origin,
			init_merkle: IncrementalMerkle,
			block_headers: Vec<SignedBlockHeader>,
			block_merkle_roots: Vec<Checksum256>,
			block_ids_list: Vec<Vec<Checksum256>>,
			pending_schedule_hashs: Vec<Checksum256>
		) {
			let _ = ensure_root(origin)?;

			ensure!(block_headers.len() > 0, "The signed block headers cannot be empty.");
			ensure!(block_headers[0].block_header.new_producers.is_some(), "Th producers list cannot be empty.");
			let ps = block_headers[0].block_header.new_producers.as_ref().unwrap().clone();

			ensure!(ProducerSchedules::exists(ps.version), "ProducerSchedule has not been initialized.");
			ensure!(LatestScheduleVersion::exists(), "LatestScheduleVersion has not been initialized.");

			let latest_schedule_version = <LatestScheduleVersion>::get();
			ensure!(ps.version == latest_schedule_version + 1, "This is a wrong new producer lists due to wrong schedule version.");

			#[cfg(feature = "std")]
			ensure!(Self::verify_block_headers(init_merkle, block_headers, block_merkle_roots, block_ids_list, pending_schedule_hashs).is_ok(), "Failed to verify block.");

			ProducerSchedules::insert(ps.version, ps.producers);
			LatestScheduleVersion::put(ps.version);

			Self::deposit_event(Event::ChangeSchedule(latest_schedule_version, ps.version));
		}

		fn prove_action(
			origin,
			block: SignedBlockHeader,
			block_merkle_root: Checksum256,
			block_merkle_paths: Vec<Checksum256>,
			action: Action,
			action_merkle_paths: Vec<Checksum256>
		) {
			let _ = ensure_root(origin)?;

			//	1. verify signature of block header
			//	2. verify block header is irreversible block
			//	3. verify action
			//	4. trigger event
			// TODO
			unimplemented!("todo")
		}
	}
}

impl<T: Trait> Module<T> {
	#[cfg(feature = "std")]
	fn verify_block_headers(
		mut init_merkle: IncrementalMerkle,
		block_headers: Vec<SignedBlockHeader>,
		block_merkle_roots: Vec<Checksum256>,
		block_ids_list: Vec<Vec<Checksum256>>,
		pending_schedule_hashs: Vec<Checksum256>
	) -> Result<(), Error> {
		ensure!(block_headers.len() == 16, Error::LengthNotEqual(16, block_headers.len()));
		ensure!(block_merkle_roots.len() == 16, Error::LengthNotEqual(16, block_merkle_roots.len()));
		ensure!(block_ids_list.len() == 15, Error::LengthNotEqual(15, block_ids_list.len()));

		for (index, (block_header, expected_mroot)) in block_headers.iter().zip(block_merkle_roots.iter()).enumerate() {
			let pending_schedule_hash = pending_schedule_hashs[index];
			Self::verify_block_header_signature(block_header, expected_mroot, &pending_schedule_hash)?;

			let block_ids = block_ids_list.get(index).unwrap();
			Self::verify_block_header_merkle_root(&mut init_merkle, &block_header, &block_ids, &expected_mroot)?;
		}
		Ok(())
	}

	#[cfg(feature = "std")]
	fn verify_block_header_signature(
		block_header: &SignedBlockHeader,
		expected_mroot: &Checksum256,
		pending_schedule_hash: &Checksum256,
	) -> Result<(), Error> {
		let schedule_version = block_header.block_header.schedule_version;
		let producers = ProducerSchedules::get(schedule_version);
		let ps = ProducerSchedule::new(schedule_version, producers);
		let producer = block_header.block_header.producer;
		let pk = ps.get_producer_key(producer);

		block_header.verify(*expected_mroot, *pending_schedule_hash, pk).map_err(|e| {
			Error::SignatureVerificationFailure
		})?;
		Ok(())
	}

	#[cfg(feature = "std")]
	fn verify_block_header_merkle_root(
		init_merkle: &mut IncrementalMerkle,
		block_header: &SignedBlockHeader,
		block_ids: &Vec<Checksum256>,
		expected_mroot: &Checksum256
	) -> Result<(), Error> {
		for id in block_ids {
			init_merkle.append(*id).map_err(|_| Error::IncreMerkleError)?;
		}

		init_merkle.append(block_header.block_header.previous).map_err(|_| Error::IncreMerkleError)?;
		let block_id = block_header.id().map_err(|_| Error::GetBlockIdFailure)?;
		init_merkle.append(block_id).map_err(|_| Error::IncreMerkleError)?;

		let actual_mroot = init_merkle.get_root();
		match actual_mroot == *expected_mroot {
			true => Ok(()),
			false => Err(Error::MerkleRootVerificationFailure),
		}
	}
}
