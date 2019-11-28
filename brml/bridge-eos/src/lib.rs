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
			block_ids_list: Vec<Vec<Checksum256>>
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
			ensure!(Self::verify_block_headers(init_merkle, block_headers, block_merkle_roots, block_ids_list).is_ok(), "Failed to verify block.");

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
		block_ids_list: Vec<Vec<Checksum256>>
	) -> Result<(), Error> {
		ensure!(block_headers.len() == 16, Error::LengthNotEqual(16, block_headers.len()));
		ensure!(block_merkle_roots.len() == 16, Error::LengthNotEqual(16, block_merkle_roots.len()));
		ensure!(block_ids_list.len() == 15, Error::LengthNotEqual(15, block_ids_list.len()));

		for (index, (block_header, expected_mroot)) in block_headers.iter().zip(block_merkle_roots.iter()).enumerate() {
			Self::verify_block_header_signature(block_header, expected_mroot)?;

			let block_ids = block_ids_list.get(index).unwrap();
			Self::verify_block_header_merkle_root(&mut init_merkle, &block_header, &block_ids, &expected_mroot)?;
		}
		Ok(())
	}

	#[cfg(feature = "std")]
	fn verify_block_header_signature(
		block_header: &SignedBlockHeader,
		expected_mroot: &Checksum256
	) -> Result<(), Error> {
		let schedule_version = block_header.block_header.schedule_version;
		let producers = ProducerSchedules::get(schedule_version - 1);
		dbg!(&producers.len());

//		let ps = ProducerSchedule::new(schedule_version, producers);
		let ps = generate_current_producers_list();
		let ps_hash = ps.schedule_hash().map_err(|_| Error::ScheduleHashError)?;
		dbg!(&ps_hash.to_string());

		let producer = block_header.block_header.producer;
		dbg!(&producer.to_string());
		let pk = ps.get_producer_key(producer);
		dbg!(&pk.to_string());

		block_header.verify(*expected_mroot, ps_hash, pk).map_err(|e| {
			dbg!(&e);
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

#[cfg(feature = "std")]
pub fn generate_current_producers_list() -> ProducerSchedule {
	let json = r#"
		{
        "version": 1,
        "producers": [
            {
                "producer_name": "batinthedark",
                "block_signing_key": "EOS6dwoM8XGMQn49LokUcLiony7JDkbHrsFDvh5svLvPDkXtvM7oR"
            },
            {
                "producer_name": "bighornsheep",
                "block_signing_key": "EOS5xfwWr4UumKm4PqUGnyCrFWYo6j5cLioNGg5yf4GgcTp2WcYxf"
            },
            {
                "producer_name": "bigpolarbear",
                "block_signing_key": "EOS6oZi9WjXUcLionUtSiKRa4iwCW5cT6oTzoWZdENXq1p2pq53Nv"
            },
            {
                "producer_name": "clevermonkey",
                "block_signing_key": "EOS5mp5wmRyL5RH2JUeEh3eoZxkJ2ZZJ9PVd1BcLioNuq4PRCZYxQ"
            },
            {
                "producer_name": "funnyhamster",
                "block_signing_key": "EOS7A9BoRetjpKtE3sqA6HRykRJ955MjQ5XdRmCLionVte2uERL8h"
            },
            {
                "producer_name": "gorillapower",
                "block_signing_key": "EOS8X5NCx1Xqa1xgQgBa9s6EK7M1SjGaDreAcLion4kDVLsjhQr9n"
            },
            {
                "producer_name": "hippopotamus",
                "block_signing_key": "EOS7qDcxm8YtAZUA3t9kxNGuzpCLioNnzpTRigi5Dwsfnszckobwc"
            },
            {
                "producer_name": "hungryolddog",
                "block_signing_key": "EOS6tw3AqqVUsCbchYRmxkPLqGct3vC63cEzKgVzLFcLionoY8YLQ"
            },
            {
                "producer_name": "iliketurtles",
                "block_signing_key": "EOS6itYvNZwhqS7cLion3xp3rLJNJAvKKegxeS7guvbBxG1XX5uwz"
            },
            {
                "producer_name": "jumpingfrogs",
                "block_signing_key": "EOS7oVWG413cLioNG7RU5Kv7NrPZovAdRSP6GZEG4LFUDWkgwNXHW"
            },
            {
                "producer_name": "lioninjungle",
                "block_signing_key": "EOS5BcLionmbgEtcmu7qY6XKWaE1q31qCQSsd89zXij7FDXQnKjwk"
            },
            {
                "producer_name": "littlerabbit",
                "block_signing_key": "EOS65orCLioNFkVT5uDF7J63bNUk97oF8T83iWfuvbSKWYUUq9EWd"
            },
            {
                "producer_name": "proudrooster",
                "block_signing_key": "EOS5qBd3T6nmLRsuACLion346Ue8UkCwvsoS5f3EDC1jwbrEiBDMX"
            },
            {
                "producer_name": "pythoncolors",
                "block_signing_key": "EOS8R7GB5CLionUEy8FgGksGAGtc2cbcQWgty3MTAgzJvGTmtqPLz"
            },
            {
                "producer_name": "soaringeagle",
                "block_signing_key": "EOS6iuBqJKqSK82QYCGuM96gduQpQG8xJsPDU1CLionPMGn2bT4Yn"
            },
            {
                "producer_name": "spideronaweb",
                "block_signing_key": "EOS6M4CYEDt3JDKS6nsxMnUcdCLioNcbyEzeAwZsQmDcoJCgaNHT8"
            },
            {
                "producer_name": "ssssssssnake",
                "block_signing_key": "EOS8SDhZ5CLioNLie9mb7kDu1gHfDXLwTvYBSxR1ccYSJERvutLqG"
            },
            {
                "producer_name": "thebluewhale",
                "block_signing_key": "EOS6Wfo1wwTPzzBVT8fe3jpz8vxCnf77YscLionBnw39iGzFWokZm"
            },
            {
                "producer_name": "thesilentowl",
                "block_signing_key": "EOS7y4hU89NJ658H1KmAdZ6A585bEVmSV8xBGJ3SbQM4Pt3pcLion"
            },
            {
                "producer_name": "wealthyhorse",
                "block_signing_key": "EOS5i1HrfxfHLRJqbExgRodhrZwp4dcLioNn4xZWCyhoBK6DNZgZt"
            }
        ]
    }
	"#;
	let new_producers: Result<ProducerSchedule, _> = serde_json::from_str(&json);
	assert!(new_producers.is_ok());

	new_producers.unwrap()
}