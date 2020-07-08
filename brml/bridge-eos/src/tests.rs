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

#![cfg(test)]

use crate::*;
use crate::mock::*;
use core::{convert::From, str::FromStr};
use eos_chain::{
	Action, ActionReceipt, Checksum256, get_proof, ProducerAuthoritySchedule,
	IncrementalMerkle, ProducerSchedule, SignedBlockHeader
};
#[cfg(feature = "std")]
use std::{
	error::Error,
	fs::File,
	io::Read as StdRead,
	path::Path,
};
use sp_core::H256;
use sp_core::offchain::{
	OffchainExt, TransactionPoolExt,
	testing::{TestOffchainExt, TestTransactionPoolExt},
};
use sp_runtime::traits::Header as HeaderT;
use sp_runtime::{generic::DigestItem, testing::Header};
use node_primitives::{BridgeAssetSymbol, BlockchainType};
use frame_support::{assert_ok, dispatch};

#[test]
fn get_latest_schedule_version_should_work() {
	new_test_ext().execute_with(|| {
		PendingScheduleVersion::put(3);
		run_to_block(100); // after 100 blocks are produced
		let ver = PendingScheduleVersion::get();
		assert_eq!(ver, 3);
	});
}

#[test]
#[ignore = "need to collect data from EOS 2.0 node"]
fn get_producer_schedules_should_work() {
	new_test_ext().execute_with(|| {
		assert!(ProducerSchedules::contains_key(0));

		let json = "change_schedule_9313.json";
		let signed_blocks_str = read_json_from_file(json);
		let signed_blocks: Result<Vec<SignedBlockHeader>, _> = serde_json::from_str(&signed_blocks_str.unwrap());
		assert!(signed_blocks.is_ok());
		let signed_blocks_headers = signed_blocks.unwrap();

		let schedule = signed_blocks_headers.first().as_ref().unwrap().block_header.new_producers.as_ref().unwrap().clone();
		let pending_schedule_hash = schedule.schedule_hash();
		assert!(pending_schedule_hash.is_ok());

		PendingScheduleVersion::put(schedule.version);

		let schedule = ProducerAuthoritySchedule::default();
		let schedule_hash = schedule.schedule_hash();
		ProducerSchedules::insert(schedule.version, (&schedule.producers, schedule_hash.unwrap()));

		run_to_block(100); // after 100 blocks are produced
		assert!(ProducerSchedules::contains_key(schedule.version));
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
#[ignore = "need to collect data from EOS 2.0 node"]
fn verify_block_header_signature_should_succeed() {
	new_test_ext().execute_with(|| {
		let json = "change_schedule_9313.json";
		let signed_blocks_str = read_json_from_file(json);
		let signed_blocks: Result<Vec<SignedBlockHeader>, _> = serde_json::from_str(&signed_blocks_str.unwrap());
		assert!(signed_blocks.is_ok());
		let signed_blocks_headers = signed_blocks.unwrap();

		let signed_block_header = signed_blocks_headers.first().as_ref().unwrap().clone();

		let schedule_hash_and_producer_schedule = BridgeEos::get_schedule_hash_and_public_key(signed_blocks_headers[0].block_header.new_producers.as_ref());
		assert!(schedule_hash_and_producer_schedule.is_ok());
		let (schedule_hash, producer_schedule) = schedule_hash_and_producer_schedule.unwrap();

		let mroot: Checksum256 = "bd1dc07bd4f14bf4d9a32834ec1d35ea92eda26cc220fe91f4f65052bfb1d45a".into();
		let result = BridgeEos::verify_block_header_signature(&schedule_hash, &producer_schedule, &signed_block_header, &mroot);
		assert!(result.is_ok());
	});
}

#[test]
#[ignore = "need to collect data from EOS 2.0 node"]
fn verify_block_headers_should_succeed() {
	new_test_ext().execute_with(|| {
		let json = "change_schedule_9313.json";
		let signed_blocks_str = read_json_from_file(json);
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

		let schedule_hash_and_producer_schedule = BridgeEos::get_schedule_hash_and_public_key(signed_blocks_headers[0].block_header.new_producers.as_ref());
		assert!(schedule_hash_and_producer_schedule.is_ok());
		let (schedule_hash, producer_schedule) = schedule_hash_and_producer_schedule.unwrap();

		let merkle = IncrementalMerkle::new(node_count, active_nodes);
		assert_ok!(BridgeEos::verify_block_headers(merkle, &schedule_hash, &producer_schedule, &signed_blocks_headers, block_ids_list));
	});
}

#[test]
#[ignore = "need to collect data from EOS 2.0 node"]
fn change_schedule_should_work() {
	new_test_ext().execute_with(|| {
		// insert producers schedule v1 in advance.
		let shedule_json = "schedule_v1.json";
		let v1_producers_str = read_json_from_file(shedule_json);
		assert!(v1_producers_str.is_ok());
		let v1_producers: Result<ProducerSchedule, _> = serde_json::from_str(&v1_producers_str.unwrap());
		assert!(v1_producers.is_ok());
		let v1_producers = ProducerAuthoritySchedule::default();

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
		assert_ok!(BridgeEos::change_schedule(Origin::root(), Checksum256::default(), v1_producers, merkle, signed_blocks_headers, block_ids_list));
	});
}

#[test]
#[ignore = "need to collect data from EOS 2.0 node"]
fn prove_action_should_be_ok() {
	new_test_ext().execute_with(|| {
		//	save producer schedule for block signature verification
		let shedule_json = "schedule_v2.json";
		let v2_producers_str = read_json_from_file(shedule_json);
		assert!(v2_producers_str.is_ok());
		let v2_producers: Result<ProducerSchedule, _> = serde_json::from_str(&v2_producers_str.unwrap());
		assert!(v2_producers.is_ok());

		let v2_producers = ProducerAuthoritySchedule::default();
		let v2_schedule_hash = v2_producers.schedule_hash();
		assert!(v2_schedule_hash.is_ok());

		PendingScheduleVersion::put(v2_producers.version);
		ProducerSchedules::insert(v2_producers.version, (&v2_producers.producers, v2_schedule_hash.unwrap()));

		// get block headers
		let block_headers_json = "actions_verification_10776.json";
		let signed_blocks_str = read_json_from_file(block_headers_json);
		let signed_blocks: Result<Vec<SignedBlockHeader>, _> = serde_json::from_str(&signed_blocks_str.unwrap());
		assert!(signed_blocks.is_ok());
		let signed_blocks_headers = signed_blocks.unwrap();

		// merkle
		let node_count = 10774;
		let active_nodes: Vec<Checksum256> = vec![
			"45c2c1cbc4b049d72a627124b05f5c476ae1cc87955fbea70bc8dbe549cf395a".into(),
			"d96747605aaed959630b23a28e0004f42a87eae93f51d5fe241735644a0c3921".into(),
			"937a489eea576d74a3d091cc4dcf1cb867f01e314ac7f1334f6cec00dfcee476".into(),
			"36cbf5d9c35b2538181bf7f8af4ee57c55c17e516eedd992a73bace9ca14a5c3".into(),
			"40e8bb864481e7bb01674ec3517c84e557869fea8160c4b2762d3e83d71d6034".into(),
			"afa502d408f5bdf1660fa9fe3a1fcb432462467e7eb403a8499392ee5297d8d1".into(),
			"f1329d3ee84040279460cbc87b6769b7363e477a832f73d639e0692a4042f093".into()
		];
		let merkle = IncrementalMerkle::new(node_count, active_nodes);

		// block ids list
		let ids_json = "block_ids_list_10776.json";
		let ids_str = read_json_from_file(ids_json).unwrap();
		let block_ids_list: Result<Vec<Vec<String>>, _> = serde_json::from_str(&ids_str);
		assert!(block_ids_list.is_ok());

		let block_ids_list: Vec<Vec<Checksum256>> = block_ids_list.as_ref().unwrap().iter().map(|ids| {
			ids.iter().map(|id| Checksum256::from_str(id).unwrap()).collect::<Vec<_>>()
		}).collect::<Vec<_>>();

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

		let proof = get_proof(15, action_merkle_paths);
		assert!(proof.is_ok());
		let actual_merkle_paths = proof.unwrap();

		// get action
		let actions_json = "actions_from_10776.json";
		let actions_str = read_json_from_file(actions_json);
		assert!(actions_str.is_ok());
		let actions: Result<Vec<Action>, _> = serde_json::from_str(actions_str.as_ref().unwrap());
		assert!(actions.is_ok());
		let actions = actions.unwrap();

		let action = actions[3].clone();

		let action_receipt = r#"{
			"receiver": "megasuper333",
			"act_digest": "eaa3b4bf845a1b41668ab7ca49fb5644fc91a6c0156dfd33911b4ec69d2e41d6",
			"global_sequence": 3040972,
			"recv_sequence": 1,
			"auth_sequence": [
			  [
				"junglefaucet",
				21
			  ]
			],
			"code_sequence": 2,
			"abi_sequence": 2
		}"#;
		let action_receipt: Result<ActionReceipt, _> = serde_json::from_str(action_receipt);
		assert!(action_receipt.is_ok());
		let action_receipt = action_receipt.unwrap();

		assert_ok!(
			BridgeEos::prove_action(Origin::root(), action.clone(), action_receipt.clone(), actual_merkle_paths, merkle, signed_blocks_headers, block_ids_list, Checksum256::default())
		);

		// ensure action_receipt is saved after proved action
		assert_eq!(BridgeActionReceipt::get(&action_receipt), action);
	});
}

#[test]
#[ignore = "This is a simulated http server, no response actually."]
fn bridge_eos_offchain_should_work() {
	let mut ext = new_test_ext();
	let (offchain, _state) = TestOffchainExt::new();
	let (pool, pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		System::set_block_number(1);
		sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, b"EOS_NODE_URL", b"http://127.0.0.1:8888/");

		// EOS secret key of account testa
		sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, b"EOS_SECRET_KEY", b"5JgbL2ZnoEAhTudReWH1RnMuQS6DBeLZt4ucV6t8aymVEuYg7sr");

		let raw_to = b"alice".to_vec();
		let raw_symbol = b"EOS".to_vec();
		let asset_symbol = BridgeAssetSymbol::new(BlockchainType::EOS, raw_symbol, 4u32);
		let bridge_asset = BridgeAssetBalance {
			symbol: asset_symbol.clone(),
			amount: 1 * 10u64.pow(8),
			memo: vec![],
			from: 1,
			token_symbol: TokenSymbol::EOS,
		};
		assert_ok!(BridgeEos::bridge_asset_to(raw_to.clone(), bridge_asset));
		assert_ok!(BridgeEos::offchain(1));

		// EOS secret key of account testb
		sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, b"EOS_SECRET_KEY", b"5J6vV6xbVV2UEwBYYDRQQ8yTDcSmHJw67XqRriF4EkEzWKUFNKj");

		rotate_author(2);
		assert_ok!(BridgeEos::offchain(2));

		use codec::Decode;
		let transaction = pool_state.write().transactions.pop().unwrap();
		assert_eq!(pool_state.read().transactions.len(), 1);
		let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
		let tx_outs = match ex.call {
			crate::mock::Call::BridgeEos(crate::Call::bridge_tx_report(tx_outs)) => tx_outs,
			e => panic!("Unexpected call: {:?}", e),
		};

		assert_eq!(tx_outs.iter().filter(|out| {
			match out {
				TxOut::Processing{ .. } => true,
				_ => false,
			}
		}).count(), 1);
	});
}

#[test]
fn bridge_eos_genesis_config_should_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(BridgeContractAccount::get(), (b"bifrostcross".to_vec(), 2));

		let producer_schedule = ProducerAuthoritySchedule::default();
		let version = producer_schedule.version;
		let producers = producer_schedule.clone().producers;
		let schedule_hash = producer_schedule.schedule_hash();
		assert_eq!(PendingScheduleVersion::get(), version);
		assert_eq!(ProducerSchedules::get(version), (producers, schedule_hash.unwrap()));
	});
}

#[test]
fn chech_receiver_is_ss58_format() {
	// alice is substrate address
	let alice_key = "5CFK52zU59zUhC3s6mRobEJ3zm7JeXQZaS6ybvcuCDDhWwGG";
	let expected_alice = [
		8u8, 22, 254, 6, 137, 50, 46, 38,
		205, 42, 169, 192, 220, 203, 108, 68,
		133, 19, 69, 233, 111, 150, 154, 232,
		92, 143, 26, 236, 159, 180, 112, 61
	];

	let data = BridgeEos::get_account_data(alice_key);
	assert!(data.is_ok());
	let data = data.unwrap();
	assert_eq!(data, expected_alice);
	let account_id = BridgeEos::into_account(data);
	assert!(account_id.is_ok());

	// this is a bifrost account address
	const BIFROST_PREFIX: u8 = 6;
	let bifrost_address = "gg2XUSNDsdmYR28YRVMZ7qeWqPpaKtG5PefahS4yKwshda2";
	let decoded_ss58 = bs58::decode(bifrost_address).into_vec();
	assert!(decoded_ss58.is_ok());
	let decoded_ss58 = decoded_ss58.unwrap();
	assert_eq!(decoded_ss58.len(), 35);
	assert_eq!(decoded_ss58[0], BIFROST_PREFIX);

	// this is a centrifuge account address
	let bifrost_address = "4bVs9EVoVRcx2BGcf66nMsv3Bw5WAwVLSzorZcUnLjLZebf7";
	let decoded_ss58 = bs58::decode(bifrost_address).into_vec();
	assert!(decoded_ss58.is_ok());
	let decoded_ss58 = decoded_ss58.unwrap();
	assert_eq!(decoded_ss58.len(), 35);
	assert_ne!(decoded_ss58[0], BIFROST_PREFIX);
}

#[cfg(feature = "std")]
fn read_json_from_file(json_name: impl AsRef<str>) -> Result<String, Box<dyn Error>> {
	let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/")).join(json_name.as_ref());
	let mut file = File::open(path)?;
	let mut json_str = String::new();
	file.read_to_string(&mut json_str)?;
	Ok(json_str)
}

#[allow(dead_code)]
fn bridge_tx_report() -> dispatch::DispatchResult {
	#[allow(deprecated)]
	use frame_support::unsigned::ValidateUnsigned;

	let tx_outs = vec![TxOut::Success(vec![])];

	#[allow(deprecated)]
	BridgeEos::pre_dispatch(&crate::Call::bridge_tx_report(tx_outs.clone())).map_err(|e| <&'static str>::from(e))?;

	BridgeEos::bridge_tx_report(
		Origin::none(),
		tx_outs,
	)
}

fn seal_header(mut header: Header, author: u64) -> Header {
	{
		let digest = header.digest_mut();
		digest.logs.push(DigestItem::PreRuntime(TEST_ID, author.encode()));
		digest.logs.push(DigestItem::Seal(TEST_ID, author.encode()));
	}

	header
}

fn create_header(number: u64, parent_hash: H256, state_root: H256) -> Header {
	Header::new(
		number,
		Default::default(),
		state_root,
		parent_hash,
		Default::default(),
	)
}

fn rotate_author(author: u64) {
	let mut header = seal_header(
		create_header(1, Default::default(), [1; 32].into()),
		author,
	);

	header.digest_mut().pop(); // pop the seal off.
	System::initialize(
		&1,
		&Default::default(),
		&Default::default(),
		header.digest(),
		Default::default(),
	);

	assert_eq!(Authorship::author(), author);
}

#[test]
fn lite_json_deserialize_push_transaction() {
	let trx_response = r#"
	{
		"transaction_id": "58e71de1c3f1a93417addbf1fc79e58e4f57a0930ec9c4f294b4ad64375c9dc6",
		"processed": {
			"id": "58e71de1c3f1a93417addbf1fc79e58e4f57a0930ec9c4f294b4ad64375c9dc6",
			"block_num": 11665607,
			"block_time": "2020-04-27T08:09:26.500",
			"producer_block_id": null,
			"receipt": {
				"status": "executed",
				"cpu_usage_us": 197,
				"net_usage_words": 17
			},
			"elapsed": 197,
			"net_usage": 136,
			"scheduled": false,
			"action_traces": [{
				"action_ordinal": 1,
				"creator_action_ordinal": 0,
				"closest_unnotified_ancestor_action_ordinal": 0,
				"receipt": {
					"receiver": "eosio.token",
					"act_digest": "955dfd6bd4edc99af285b853927ea8d3a244aec2d30d7c7a2adefb8f6e518510",
					"global_sequence": 15255106,
					"recv_sequence": 821754,
					"auth_sequence": [
						["bifrostcross", 50]
					],
					"code_sequence": 1,
					"abi_sequence": 1
				},
				"receiver": "eosio.token",
				"act": {
					"account": "eosio.token",
					"name": "transfer",
					"authorization": [{
						"actor": "bifrostcross",
						"permission": "active"
					}],
					"data": {
						"from": "bifrostcross",
						"to": "bifrostliebi",
						"quantity": "1.0000 EOS",
						"memo": "a memo"
					},
					"hex_data": "8031bd28637a973be08e7231637a973b102700000000000004454f53000000000661206d656d6f"
				},
				"context_free": false,
				"elapsed": 55,
				"console": "",
				"trx_id": "58e71de1c3f1a93417addbf1fc79e58e4f57a0930ec9c4f294b4ad64375c9dc6",
				"block_num": 11665607,
				"block_time": "2020-04-27T08:09:26.500",
				"producer_block_id": null,
				"account_ram_deltas": [],
				"except": null,
				"error_code": null,
				"inline_traces": [{
					"action_ordinal": 2,
					"creator_action_ordinal": 1,
					"closest_unnotified_ancestor_action_ordinal": 1,
					"receipt": {
						"receiver": "bifrostcross",
						"act_digest": "955dfd6bd4edc99af285b853927ea8d3a244aec2d30d7c7a2adefb8f6e518510",
						"global_sequence": 15255107,
						"recv_sequence": 21,
						"auth_sequence": [
							["bifrostcross", 51]
						],
						"code_sequence": 1,
						"abi_sequence": 1
					},
					"receiver": "bifrostcross",
					"act": {
						"account": "eosio.token",
						"name": "transfer",
						"authorization": [{
							"actor": "bifrostcross",
							"permission": "active"
						}],
						"data": {
							"from": "bifrostcross",
							"to": "bifrostliebi",
							"quantity": "1.0000 EOS",
							"memo": "a memo"
						},
						"hex_data": "8031bd28637a973be08e7231637a973b102700000000000004454f53000000000661206d656d6f"
					},
					"context_free": false,
					"elapsed": 22,
					"console": "",
					"trx_id": "58e71de1c3f1a93417addbf1fc79e58e4f57a0930ec9c4f294b4ad64375c9dc6",
					"block_num": 11665607,
					"block_time": "2020-04-27T08:09:26.500",
					"producer_block_id": null,
					"account_ram_deltas": [],
					"except": null,
					"error_code": null,
					"inline_traces": []
				}, {
					"action_ordinal": 3,
					"creator_action_ordinal": 1,
					"closest_unnotified_ancestor_action_ordinal": 1,
					"receipt": {
						"receiver": "bifrostliebi",
						"act_digest": "955dfd6bd4edc99af285b853927ea8d3a244aec2d30d7c7a2adefb8f6e518510",
						"global_sequence": 15255108,
						"recv_sequence": 28,
						"auth_sequence": [
							["bifrostcross", 52]
						],
						"code_sequence": 1,
						"abi_sequence": 1
					},
					"receiver": "bifrostliebi",
					"act": {
						"account": "eosio.token",
						"name": "transfer",
						"authorization": [{
							"actor": "bifrostcross",
							"permission": "active"
						}],
						"data": {
							"from": "bifrostcross",
							"to": "bifrostliebi",
							"quantity": "1.0000 EOS",
							"memo": "a memo"
						},
						"hex_data": "8031bd28637a973be08e7231637a973b102700000000000004454f53000000000661206d656d6f"
					},
					"context_free": false,
					"elapsed": 3,
					"console": "",
					"trx_id": "58e71de1c3f1a93417addbf1fc79e58e4f57a0930ec9c4f294b4ad64375c9dc6",
					"block_num": 11665607,
					"block_time": "2020-04-27T08:09:26.500",
					"producer_block_id": null,
					"account_ram_deltas": [],
					"except": null,
					"error_code": null,
					"inline_traces": []
				}]
			}],
			"account_ram_delta": null,
			"except": null,
			"error_code": null
		}
	}
	"#;
	let trx_id: Result<String, _> = transaction::eos_rpc::get_transaction_id::<Test>(trx_response);
	assert!(trx_id.is_ok());
	assert_eq!(trx_id.unwrap(), "58e71de1c3f1a93417addbf1fc79e58e4f57a0930ec9c4f294b4ad64375c9dc6");
}
