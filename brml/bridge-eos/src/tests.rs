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

use crate::mock::{new_test_ext, run_to_block, BridgeEOSTestModule, System};
use eos_chain::{Action, Checksum256, IncrementalMerkle, ProducerKey, ProducerSchedule, SignedBlockHeader};
use support::{assert_ok};
use sr_primitives::traits::OnInitialize;
use sr_primitives::traits::OnFinalize;
use crate::*;

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

// TODO, add more test cases on verification blocks

fn generate_producers_list() -> ProducerSchedule{
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