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

#![cfg(test)]

use crate::*;
use crate::mock::{new_test_ext, run_to_block, BridgeEOSTestModule, System};
use eos_chain::{
	Action, Checksum256, IncrementalMerkle, TimePointSec,
	ProducerKey, ProducerSchedule, SignedBlockHeader,
	BlockHeader, BlockTimestamp, AccountName, Extension, Signature,
	SerializeData,
};
use support::{assert_ok};
use sr_primitives::traits::OnInitialize;
use sr_primitives::traits::OnFinalize;
use core::convert::From;
use core::str::FromStr;

#[test]
fn get_latest_schedule_version_shuold_work() {
	new_test_ext().execute_with(|| {
		assert!(!LatestScheduleVersion::exists());
		LatestScheduleVersion::put(3);
		run_to_block(100); // after 100 blocks are produced
		let ver = BridgeEOSTestModule::latest_schedule_version();
		assert_eq!(ver, 3);
	});
}

#[test]
fn get_producer_schedules_should_work() {
	new_test_ext().execute_with(|| {
		assert!(!ProducerSchedules::exists(0));

		let new_producers = generate_producers_list();
		let version = new_producers.version;
		ProducerSchedules::insert(version, &new_producers.producers);

		run_to_block(100); // after 100 blocks are produced
		assert!(ProducerSchedules::exists(version));
		let producers_list = BridgeEOSTestModule::producer_schedules(version);
		assert_eq!(producers_list, new_producers.producers);
	});
}

#[test]
fn test_incremental_merkle() {
	let id = "00002461b0e90b849fe37ff9514230b4f9fc4012d0394b7d2445fedef6a45807";
	let id = Checksum256::from(id);

	let node_count = 9312;
	let active_nodes = vec![
		"f0c9603f20e413ee058fb10a85169a2acdbcf275c0b0a6bd63239a0c0be4538e".into(),
		"4d018eda9a22334ac0492489fdf79118d696eea52af3871a7e4bf0e2d5ab5945".into(),
		"acba7c7ee5c1d8ba97ea1a841707fbb2147e883b56544ba821814aebe086383e".into(),
		"afa502d408f5bdf1660fa9fe3a1fcb432462467e7eb403a8499392ee5297d8d1".into(),
		"bd1dc07bd4f14bf4d9a32834ec1d35ea92eda26cc220fe91f4f65052bfb1d45a".into()
	];

	let mut merkle = IncrementalMerkle::new(node_count, active_nodes);
	merkle.append(id);
	assert_eq!("33cef09fe2565cb5ed2c18c389209897a226a4f8c47360d88cdc2dcc17a8cfc5", merkle.get_root().to_string());

	let id = "00002462003ceef70350d95ac4822360c55bfc47242c79918bbbe6408ea370c9";
	let id = Checksum256::from(id);
	merkle.append(id);
	assert_eq!("9086d260c4c8b8918762607d46defba3c6f534435573319a88c78bae48a58006", merkle.get_root().to_string());
}

// TODO, add more test cases on verification blocks
#[test]
fn verify_block_header_signature_should_succeed() {
	new_test_ext().execute_with(|| {
		let ps = generate_current_producers_list();
		<BridgeEOSTestModule as Store>::ProducerSchedules::insert(ps.version, ps.producers);

		let block_time = BlockTimestamp::from(TimePointSec::from_unix_seconds(1542994962));

		let block_header = BlockHeader::new(
			block_time,
			AccountName::from_str("hungryolddog").unwrap(),
			228,
			Checksum256::from("00002460d0b0d9a7dbf1a82779c657edc04abcd9b74e03111fd79a3acae3b216"),
			Checksum256::from("0000000000000000000000000000000000000000000000000000000000000000"),
			Checksum256::from("845df9d90c39c48ac3dd78f58c8a0235587435310ad6d29d7981ffcb37e0cdae"),
			1,
			Some(generate_producers_list()),
			Default::default()
		);

		let signed_block_header = SignedBlockHeader{
			block_header,
			producer_signature: Signature::from_str("SIG_K1_K1t32qBPbyMdWyMfSRQ8z4TwFMpDUxgCHw8oQCexepdyZkCUUGyqS9WQBQpTp1bJ9PgES1uWJB8kJjW4HDY63PzdTqTaAo").unwrap(),
		};

		let mroot: Checksum256 = "bd1dc07bd4f14bf4d9a32834ec1d35ea92eda26cc220fe91f4f65052bfb1d45a".into();
		let pending_schedule_hash: Checksum256 = "4204d5ca327bae53aac3b5405e356172d2b2dd42c2f609f4f970e41d0d3dcae1".into();
		let result = BridgeEOSTestModule::verify_block_header_signature(&signed_block_header, &mroot, &pending_schedule_hash);
		assert!(result.is_ok());
	});
}

//#[test]
//fn verify_merkle_root_should_succeed() {
//	let index: usize = 0;
//	let init_merkle: IncrementalMerkle;
//	let block_header: SignedBlockHeader;
//	let block_ids_list: &Vec<Vec<Checksum256>>;
//	let expected_mroot: Checksum256;
//
//	assert!(BridgeEOSTestModule::verify_block_header_merkle_root(index, &init_merkle, &block_header, &block_ids_list, &expected_mroot).is_ok));
//}

fn generate_producers_list() -> ProducerSchedule {
	let json = r#"
		{
			"version": 2,
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
				  "producer_name": "ohtigertiger",
				  "block_signing_key": "EOS7tigERwXDRuHsok212UDToxFS1joUhAxzvDUhRof8NjuvwtoHX"
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
