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
use crate::mock::{new_test_ext, run_to_block, BridgeEos, Origin};
use core::{convert::From, str::FromStr};
use eos_chain::{
	Action, ActionReceipt, Checksum256,
	IncrementalMerkle, ProducerSchedule, SignedBlockHeader
};
#[cfg(feature = "std")]
use std::{
	error::Error,
	fs::File,
	io::Read,
	path::Path,
};

#[test]
fn get_latest_schedule_version_shuold_work() {
	new_test_ext().execute_with(|| {
		assert!(!PendingScheduleVersion::exists());
		PendingScheduleVersion::put(3);
		run_to_block(100); // after 100 blocks are produced
		let ver = PendingScheduleVersion::get();
		assert_eq!(ver, 3);
	});
}

#[test]
fn get_producer_schedules_should_work() {
	new_test_ext().execute_with(|| {
		assert!(!ProducerSchedules::exists(0));

		let json = "change_schedule_9313.json";
		let signed_blocks_str = read_json_from_file(json);
		let signed_blocks: Result<Vec<SignedBlockHeader>, _> = serde_json::from_str(&signed_blocks_str.unwrap());
		assert!(signed_blocks.is_ok());
		let signed_blocks_headers = signed_blocks.unwrap();

		let schedule = signed_blocks_headers.first().as_ref().unwrap().block_header.new_producers.as_ref().unwrap().clone();
		let pending_schedule_hash = schedule.schedule_hash();
		assert!(pending_schedule_hash.is_ok());

		PendingScheduleVersion::put(schedule.version);
		ProducerSchedules::insert(schedule.version, (&schedule.producers, pending_schedule_hash.as_ref().unwrap()));

		run_to_block(100); // after 100 blocks are produced
		assert!(ProducerSchedules::exists(schedule.version));
		let (producers_list, schedule_hash) = ProducerSchedules::get(schedule.version);
		assert_eq!(producers_list, schedule.producers);
		assert_eq!(pending_schedule_hash.unwrap(), schedule_hash);
	});
}

#[test]
fn test_incremental_merkle() {
	let id = "00002460d0b0d9a7dbf1a82779c657edc04abcd9b74e03111fd79a3acae3b216"; // 9312
	let id = Checksum256::from(id);

	let node_count = 9311;
	let active_nodes: Vec<Checksum256> = vec![
		"0000245f60aa338bd246cb7598a14796ee0210f669f9c9b37f6ddad0b5765649".into(),
		"9d41d4581cab233fe68c4510cacd05d0cc979c53ae317ce9364040578037de6a".into(),
		"a397d1a6dc90389dc592ea144b1801c4b323c12b0b2f066aa55faa5892803317".into(),
		"0cf502411e185ea7e3cc790e0b757807987e767a81c463c3e4ee5970b7fd1c67".into(),
		"9f774a35e86ddb2d293da1bfe2e25b7b447fd3d9372ee580fce230a87fefa586".into(),
		"4d018eda9a22334ac0492489fdf79118d696eea52af3871a7e4bf0e2d5ab5945".into(),
		"acba7c7ee5c1d8ba97ea1a841707fbb2147e883b56544ba821814aebe086383e".into(),
		"afa502d408f5bdf1660fa9fe3a1fcb432462467e7eb403a8499392ee5297d8d1".into(),
		"4d723385cad26cf80c2db366f9666a3ef77679c098e07d1af48d523b64b1d460".into()
	];

	let mut merkle = IncrementalMerkle::new(node_count, active_nodes);
	let _ = merkle.append(id);
	assert_eq!("bd1dc07bd4f14bf4d9a32834ec1d35ea92eda26cc220fe91f4f65052bfb1d45a", merkle.get_root().to_string());

	let id = "00002461b0e90b849fe37ff9514230b4f9fc4012d0394b7d2445fedef6a45807"; // 9313
	let id = Checksum256::from(id);
	let _ = merkle.append(id);
	assert_eq!("33cef09fe2565cb5ed2c18c389209897a226a4f8c47360d88cdc2dcc17a8cfc5", merkle.get_root().to_string());
}

#[test]
fn block_id_append_should_be_ok() {
	let ids: Vec<Checksum256> = vec![
		"00002461b0e90b849fe37ff9514230b4f9fc4012d0394b7d2445fedef6a45807".into(),
		"00002462003ceef70350d95ac4822360c55bfc47242c79918bbbe6408ea370c9".into(),
		"0000246332247ed8351c67c1a897dda27b90ec98f7e48a9c7bf527630645ad6e".into(),
		"00002464da28dba8459bb800938b779e697c374c62358a0705942c64ccf12c69".into(),
		"00002465a14b8e3ac4a712da00760b853349abf7de463eeaeb5848c392b23c28".into(),
		"00002466353e42ed9f9539cc21fdab7b926c9c3fda26e21bee0ad40777484921".into(),
		"0000246720f59108cda58140cea1a280d43deaf551279cb1385545128b667b24".into(),
		"00002468823e36de4caf75fcf63bcd2c875453a130d8448ec04e5ff1ce13e261".into(),
		"0000246915e14cc2f5577cad9cc75ad0b910c98f4bcb0153d8191d061176d6ff".into(),
		"0000246a7b047fb30b2052ad0af56787299bf123431e29f947c0f9c2a75f5cfb".into(),
		"0000246b53b5dae2e0e3a84c84c3a67119fa1f1cfb4983995ef7c69d1c6adba1".into(),
		"0000246c6388126ca93772c79ccf476f8f1aea397895fe35c05c6b153dad4e4f".into()
	];

	let node_count = 9312;
	let active_nodes: Vec<Checksum256> = vec![
		"f0c9603f20e413ee058fb10a85169a2acdbcf275c0b0a6bd63239a0c0be4538e".into(),
		"4d018eda9a22334ac0492489fdf79118d696eea52af3871a7e4bf0e2d5ab5945".into(),
		"acba7c7ee5c1d8ba97ea1a841707fbb2147e883b56544ba821814aebe086383e".into(),
		"afa502d408f5bdf1660fa9fe3a1fcb432462467e7eb403a8499392ee5297d8d1".into(),
		"bd1dc07bd4f14bf4d9a32834ec1d35ea92eda26cc220fe91f4f65052bfb1d45a".into()
	];
	let mut merkle = IncrementalMerkle::new(node_count, active_nodes);
	for id in ids {
		let _ = merkle.append(id);
	}
	assert_eq!(merkle.get_root().to_string(), "fc30e3852df3ecde2314fa1bbd4372341d1c9b922f9f44ce04460fb209b263e4");
}

#[test]
fn verify_block_header_signature_should_succeed() {
	new_test_ext().execute_with(|| {
		let json = "change_schedule_9313.json";
		let signed_blocks_str = read_json_from_file(json);
		let signed_blocks: Result<Vec<SignedBlockHeader>, _> = serde_json::from_str(&signed_blocks_str.unwrap());
		assert!(signed_blocks.is_ok());
		let signed_blocks_headers = signed_blocks.unwrap();

		let signed_block_header = signed_blocks_headers.first().as_ref().unwrap().clone();

		let mroot: Checksum256 = "bd1dc07bd4f14bf4d9a32834ec1d35ea92eda26cc220fe91f4f65052bfb1d45a".into();
		let result = BridgeEos::verify_block_header_signature(&signed_block_header, &mroot);
		assert!(result.is_ok());
	});
}

#[test]
fn verify_block_headers_should_succeed() {
	new_test_ext().execute_with(|| {
		let json = "change_schedule_9313.json";
		let signed_blocks_str = read_json_from_file(json);
		let signed_blocks: Result<Vec<SignedBlockHeader>, _> = serde_json::from_str(&signed_blocks_str.unwrap());
		assert!(signed_blocks.is_ok());
		let signed_blocks_headers = signed_blocks.unwrap();

		let schedule = signed_blocks_headers.first().as_ref().unwrap().block_header.new_producers.as_ref().unwrap().clone();
		let pending_schedule_hash = schedule.schedule_hash();
		assert!(pending_schedule_hash.is_ok());

		PendingScheduleVersion::put(schedule.version);
		ProducerSchedules::insert(schedule.version, (&schedule.producers, pending_schedule_hash.unwrap()));

		let ids_json = "block_ids_list.json";
		let ids_str = read_json_from_file(ids_json).unwrap();
		let block_ids_list: Result<Vec<Vec<String>>, _> = serde_json::from_str(&ids_str);
		assert!(block_ids_list.is_ok());

		let block_ids_list: Vec<Vec<Checksum256>> = block_ids_list.as_ref().unwrap().iter().map(|ids| {
			ids.iter().map(|id| Checksum256::from_str(id).unwrap()).collect::<Vec<_>>()
		}).collect::<Vec<_>>();

		let node_count = 9311;
		let active_nodes: Vec<Checksum256> = vec![
			"0000245f60aa338bd246cb7598a14796ee0210f669f9c9b37f6ddad0b5765649".into(),
			"9d41d4581cab233fe68c4510cacd05d0cc979c53ae317ce9364040578037de6a".into(),
			"a397d1a6dc90389dc592ea144b1801c4b323c12b0b2f066aa55faa5892803317".into(),
			"0cf502411e185ea7e3cc790e0b757807987e767a81c463c3e4ee5970b7fd1c67".into(),
			"9f774a35e86ddb2d293da1bfe2e25b7b447fd3d9372ee580fce230a87fefa586".into(),
			"4d018eda9a22334ac0492489fdf79118d696eea52af3871a7e4bf0e2d5ab5945".into(),
			"acba7c7ee5c1d8ba97ea1a841707fbb2147e883b56544ba821814aebe086383e".into(),
			"afa502d408f5bdf1660fa9fe3a1fcb432462467e7eb403a8499392ee5297d8d1".into(),
			"4d723385cad26cf80c2db366f9666a3ef77679c098e07d1af48d523b64b1d460".into()
		];

		let merkle = IncrementalMerkle::new(node_count, active_nodes);
		assert!(BridgeEos::verify_block_headers(merkle, signed_blocks_headers, block_ids_list).is_ok());
	});
}

#[test]
fn change_schedule_should_work() {
	new_test_ext().execute_with(|| {
		// insert producers schedule v1 in advance.
		let shedule_json = "schedule_v1.json";
		let v1_producers_str = read_json_from_file(shedule_json);
		assert!(v1_producers_str.is_ok());
		let v1_producers: Result<ProducerSchedule, _> = serde_json::from_str(&v1_producers_str.unwrap());
		assert!(v1_producers.is_ok());
		let v1_producers = v1_producers.unwrap();

		let v1_schedule_hash = v1_producers.schedule_hash();
		assert!(v1_schedule_hash.is_ok());

		PendingScheduleVersion::put(v1_producers.version);
		ProducerSchedules::insert(v1_producers.version, (&v1_producers.producers, v1_schedule_hash.unwrap()));

		let block_headers_json = "change_schedule_9313.json";
		let signed_blocks_str = read_json_from_file(block_headers_json);
		let signed_blocks: Result<Vec<SignedBlockHeader>, _> = serde_json::from_str(&signed_blocks_str.unwrap());
		assert!(signed_blocks.is_ok());
		let signed_blocks_headers = signed_blocks.unwrap();

		let ids_json = "block_ids_list.json";
		let ids_str = read_json_from_file(ids_json).unwrap();
		let block_ids_list: Result<Vec<Vec<String>>, _> = serde_json::from_str(&ids_str);
		assert!(block_ids_list.is_ok());

		let block_ids_list: Vec<Vec<Checksum256>> = block_ids_list.as_ref().unwrap().iter().map(|ids| {
			ids.iter().map(|id| Checksum256::from_str(id).unwrap()).collect::<Vec<_>>()
		}).collect::<Vec<_>>();

		let node_count = 9311;
		let active_nodes: Vec<Checksum256> = vec![
			"0000245f60aa338bd246cb7598a14796ee0210f669f9c9b37f6ddad0b5765649".into(),
			"9d41d4581cab233fe68c4510cacd05d0cc979c53ae317ce9364040578037de6a".into(),
			"a397d1a6dc90389dc592ea144b1801c4b323c12b0b2f066aa55faa5892803317".into(),
			"0cf502411e185ea7e3cc790e0b757807987e767a81c463c3e4ee5970b7fd1c67".into(),
			"9f774a35e86ddb2d293da1bfe2e25b7b447fd3d9372ee580fce230a87fefa586".into(),
			"4d018eda9a22334ac0492489fdf79118d696eea52af3871a7e4bf0e2d5ab5945".into(),
			"acba7c7ee5c1d8ba97ea1a841707fbb2147e883b56544ba821814aebe086383e".into(),
			"afa502d408f5bdf1660fa9fe3a1fcb432462467e7eb403a8499392ee5297d8d1".into(),
			"4d723385cad26cf80c2db366f9666a3ef77679c098e07d1af48d523b64b1d460".into()
		];

		let merkle = IncrementalMerkle::new(node_count, active_nodes);
		assert!(BridgeEos::change_schedule(Origin::ROOT, merkle, signed_blocks_headers, block_ids_list).is_ok());
	});
}

#[test]
fn verify_block_headers_action_merkle_should_be_ok() {
	new_test_ext().execute_with(|| {
		// read action merkle paths
		let action_merkle_paths_json = "action_merkle_paths.json";
		let action_merkle_paths_str = read_json_from_file(action_merkle_paths_json);
		assert!(action_merkle_paths_str.is_ok());
		let action_merkle_paths: Result<Vec<String>, _> = serde_json::from_str(&action_merkle_paths_str.unwrap());
		assert!(action_merkle_paths.is_ok());
		let action_merkle_paths = action_merkle_paths.unwrap();
		let action_merkle_paths = {
			let mut path: Vec<Checksum256> = Vec::with_capacity(action_merkle_paths.len());
			for path_str in action_merkle_paths {
				path.push(Checksum256::from_str(&path_str).unwrap());
			}
			path
		};

		// from block 10776
		let actions_json = "actions_from_10776.json";
		let actions_str = read_json_from_file(actions_json);
		assert!(actions_str.is_ok());
		let actions: Result<Vec<Action>, _> = serde_json::from_str(actions_str.as_ref().unwrap());
		assert!(actions.is_ok());
		let actions = actions.unwrap();

		// get action receipts
		let action_receipts_json = "action_receipts.json";
		let action_receipts_str = read_json_from_file(action_receipts_json);
		let action_receipts: Result<Vec<ActionReceipt>, _> = serde_json::from_str(action_receipts_str.as_ref().unwrap());
		assert!(action_receipts.is_ok());
		let action_receipts = action_receipts.unwrap();

		// get block header
		let block_header_json = "prove_action_10776.json";
		let signed_block_str = read_json_from_file(block_header_json);
		let signed_block: Result<SignedBlockHeader, _> = serde_json::from_str(signed_block_str.as_ref().unwrap());
		assert!(signed_block.is_ok());
		let signed_block_header = signed_block.unwrap();

		assert!(BridgeEos::verify_block_headers_action_merkle(Origin::ROOT, signed_block_header, actions, action_merkle_paths, action_receipts).is_ok());
	});
}

#[cfg(feature = "std")]
fn read_json_from_file(json_name: impl AsRef<str>) -> Result<String, Box<dyn Error>> {
	let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/")).join(json_name.as_ref());
	let mut file = File::open(path)?;
	let mut json_str = String::new();
	file.read_to_string(&mut json_str)?;
	Ok(json_str)
}
