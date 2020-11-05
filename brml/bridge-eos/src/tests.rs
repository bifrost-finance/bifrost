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
	Action, ActionReceipt, Checksum256, ProducerAuthoritySchedule, IncrementalMerkle, SignedBlockHeader
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
use frame_support::assert_ok;

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
		let json = "v2_data/change-schedule-27776771.json";
		let signed_blocks_str = read_json_from_file(json);
		let signed_blocks: Result<Vec<SignedBlockHeader>, _> = serde_json::from_str(&signed_blocks_str.unwrap());
		assert!(signed_blocks.is_ok());
		let signed_blocks_headers = signed_blocks.unwrap();

		let producers_schedule_json = "v2_data/producers-schedule-42.json";
		let producers_schedule_str = read_json_from_file(&producers_schedule_json).unwrap();
		let producers_schedule_v2: Result<ProducerAuthoritySchedule, _> = serde_json::from_str(&producers_schedule_str);
		assert!(producers_schedule_v2.is_ok());
		let producers_schedule_v2 = producers_schedule_v2.unwrap();

		let producers_schedule_v2_hash = producers_schedule_v2.schedule_hash().unwrap();

		let signed_block_header = signed_blocks_headers.first().as_ref().unwrap().clone();

		let mroot: Checksum256 = "166c86f7ac95b10550698df6bb6741466c065e49c799a8fe92c0e293cb87a03c".into();
		let result = BridgeEos::verify_block_header_signature(&producers_schedule_v2_hash, &producers_schedule_v2, &signed_block_header, &mroot);
		assert!(result.is_ok());
	});
}

#[test]
fn verify_block_headers_should_succeed() {
	new_test_ext().execute_with(|| {
		let json = "v2_data/change-schedule-27776771.json";
		let signed_blocks_str = read_json_from_file(json);
		let signed_blocks: Result<Vec<SignedBlockHeader>, _> = serde_json::from_str(&signed_blocks_str.unwrap());
		assert!(signed_blocks.is_ok());
		let signed_blocks_headers = signed_blocks.unwrap();

		let ids_json = "v2_data/change-schedule-id-list.json";
		let ids_str = read_json_from_file(ids_json).unwrap();
		let block_ids_list: Result<Vec<Vec<String>>, _> = serde_json::from_str(&ids_str);
		assert!(block_ids_list.is_ok());

		let producers_schedule_json = "v2_data/producers-schedule-42.json";
		let producers_schedule_str = read_json_from_file(&producers_schedule_json).unwrap();
		let producers_schedule_v2: Result<ProducerAuthoritySchedule, _> = serde_json::from_str(&producers_schedule_str);
		assert!(producers_schedule_v2.is_ok());
		let producers_schedule_v2 = producers_schedule_v2.unwrap();

		let producers_schedule_v2_hash = producers_schedule_v2.schedule_hash();
		assert!(producers_schedule_v2_hash.is_ok());

		let block_ids_list: Vec<Vec<Checksum256>> = block_ids_list.as_ref().unwrap().iter().map(|ids| {
			ids.iter().map(|id| Checksum256::from_str(id).unwrap()).collect::<Vec<_>>()
		}).collect::<Vec<_>>();

		let node_count = 27776769;
		let active_nodes: Vec<Checksum256> = vec![
			"01a7d701bf986db45ca31d0883ec4056f84f0a2e79832cd2fe141f8de62014ca".into(),
			"8883146502c5064d29f01b850ea962b63e59e06d6a759fa1bd3e43783c2743e9".into(),
			"4dd031e64709b50af0bc7f2563d21d2d80bf8017cf601240c1b70560b4d55325".into(),
			"3228d1d1f25582ce3b3b554bc662cff1e28778dfd640869b6ba1134975213de0".into(),
			"29d583e011dcf974e9c9e0be3d186eb82bdcdda22ed77f169244bb4b574f9091".into(),
			"f9e4963773784b5ea90c5725896db87a551611f661437df0cb9bef37101d1ae2".into(),
			"06b43c70cd0377a3035cf70aacba3603229406585d24f7ad528d824f1861124a".into(),
			"86c8b86d129007c2c47669d6c491f288e7470b0c6ebcc57cb13ffa6ada5cace2".into(),
			"bdf93f64dffc9c1fc95de7803cfa83569491a38f594a5a757d7cc12c72c5f1db".into(),
			"f646dfd3cffb71b89443a038c04dd2449411a8c75ab765bb057aff5b3cc5ad46".into(),
			"28e66467001758a36489e45cc20f9c1812a3b853f2827a6a662fa02935b5ee59".into(),
			"89c365695ac50e1342d854bf4bdb35066ca7b1954d04c6df4581915c5147b4df".into(),
			"9292a3e4f9af07c619ba70ca31f8aef64bae7a31154369544f32874b416b2dad".into(),
			"4095785f71399bf608c063e43ca5a8b0d09500d7560b1f2ea991ad3d59be5c26".into()
		];

		let merkle = IncrementalMerkle::new(node_count, active_nodes);
		assert_ok!(BridgeEos::verify_block_headers(merkle, &producers_schedule_v2_hash.unwrap(), &producers_schedule_v2, &signed_blocks_headers, block_ids_list));
	});
}

#[test]
fn change_schedule_should_work() {
	new_test_ext().execute_with(|| {
		// insert producers schedule v1 in advance.
		let producers_schedule_json = "v2_data/producers-schedule-42.json";
		let producers_schedule_str = read_json_from_file(&producers_schedule_json).unwrap();
		let producers_schedule_v2: Result<ProducerAuthoritySchedule, _> = serde_json::from_str(&producers_schedule_str);
		assert!(producers_schedule_v2.is_ok());
		let producers_schedule_v2 = producers_schedule_v2.unwrap();

		let producers_schedule_v2_hash = producers_schedule_v2.schedule_hash();
		assert!(producers_schedule_v2_hash.is_ok());

		PendingScheduleVersion::put(producers_schedule_v2.version);
		ProducerSchedules::insert(producers_schedule_v2.version, (&producers_schedule_v2.producers, producers_schedule_v2_hash.unwrap()));

		let block_headers_json = "v2_data/change-schedule-27776771.json";
		let signed_blocks_str = read_json_from_file(block_headers_json);
		let signed_blocks: Result<Vec<SignedBlockHeader>, _> = serde_json::from_str(&signed_blocks_str.unwrap());
		assert!(signed_blocks.is_ok());
		let signed_blocks_headers = signed_blocks.unwrap();

		let ids_json = "v2_data/change-schedule-id-list.json";
		let ids_str = read_json_from_file(ids_json).unwrap();
		let block_ids_list: Result<Vec<Vec<String>>, _> = serde_json::from_str(&ids_str);
		assert!(block_ids_list.is_ok());

		let block_ids_list: Vec<Vec<Checksum256>> = block_ids_list.as_ref().unwrap().iter().map(|ids| {
			ids.iter().map(|id| Checksum256::from_str(id).unwrap()).collect::<Vec<_>>()
		}).collect::<Vec<_>>();

		let node_count = 27776769;
		let active_nodes: Vec<Checksum256> = vec![
			"01a7d701bf986db45ca31d0883ec4056f84f0a2e79832cd2fe141f8de62014ca".into(),
			"8883146502c5064d29f01b850ea962b63e59e06d6a759fa1bd3e43783c2743e9".into(),
			"4dd031e64709b50af0bc7f2563d21d2d80bf8017cf601240c1b70560b4d55325".into(),
			"3228d1d1f25582ce3b3b554bc662cff1e28778dfd640869b6ba1134975213de0".into(),
			"29d583e011dcf974e9c9e0be3d186eb82bdcdda22ed77f169244bb4b574f9091".into(),
			"f9e4963773784b5ea90c5725896db87a551611f661437df0cb9bef37101d1ae2".into(),
			"06b43c70cd0377a3035cf70aacba3603229406585d24f7ad528d824f1861124a".into(),
			"86c8b86d129007c2c47669d6c491f288e7470b0c6ebcc57cb13ffa6ada5cace2".into(),
			"bdf93f64dffc9c1fc95de7803cfa83569491a38f594a5a757d7cc12c72c5f1db".into(),
			"f646dfd3cffb71b89443a038c04dd2449411a8c75ab765bb057aff5b3cc5ad46".into(),
			"28e66467001758a36489e45cc20f9c1812a3b853f2827a6a662fa02935b5ee59".into(),
			"89c365695ac50e1342d854bf4bdb35066ca7b1954d04c6df4581915c5147b4df".into(),
			"9292a3e4f9af07c619ba70ca31f8aef64bae7a31154369544f32874b416b2dad".into(),
			"4095785f71399bf608c063e43ca5a8b0d09500d7560b1f2ea991ad3d59be5c26".into()
		];

		let alice = Origin::signed(1u64);

		let merkle = IncrementalMerkle::new(node_count, active_nodes);
		assert_ok!(BridgeEos::change_schedule(alice, Checksum256::default(), producers_schedule_v2, merkle, signed_blocks_headers, block_ids_list));
	});
}

#[test]
fn prove_action_should_be_ok() {
	new_test_ext().execute_with(|| {
		//	save producer schedule for block signature verification
		let producers_schedule_json = "v2_data/producers-schedule-55.json";
		let producers_schedule_str = read_json_from_file(&producers_schedule_json).unwrap();
		let producers_schedule_v2: Result<ProducerAuthoritySchedule, _> = serde_json::from_str(&producers_schedule_str);
		assert!(producers_schedule_v2.is_ok());
		let producers_schedule_v2 = producers_schedule_v2.unwrap();

		let producers_schedule_v2_hash = producers_schedule_v2.schedule_hash();
		assert!(producers_schedule_v2_hash.is_ok());

		PendingScheduleVersion::put(producers_schedule_v2.version);
		ProducerSchedules::insert(producers_schedule_v2.version, (&producers_schedule_v2.producers, producers_schedule_v2_hash.unwrap()));

		// get block headers
		let block_headers_json = "v2_data/prove-action-blockheaders.json";
		let signed_blocks_str = read_json_from_file(block_headers_json);
		let signed_blocks: Result<Vec<SignedBlockHeader>, _> = serde_json::from_str(&signed_blocks_str.unwrap());
		assert!(signed_blocks.is_ok());
		let signed_blocks_headers = signed_blocks.unwrap();

		// blockroot merkle
		let node_count = 30294599;
		let active_nodes: Vec<Checksum256> = vec![
			"01ce42477e231bdfb1b93edf0ba675404f52001464403ef4f4b2bf4ac0724b95".into(),
			"2b0b66f83b18d7234cbd5153473525417eada5b56d8dd039331c6018b9d404b8".into(),
			"f4b68a787a024d30a31b16ee017ac9d136f8a8aced0c497c61e13f73969e1a95".into(),
			"f2b8d826f2f464ad7a2349ed8a569f53f4f2f0775feafedbc76672ce6195df27".into(),
			"965b1a8a0649a3444074c282c7d5477819312546d737b2a77ba0e06e60a11955".into(),
			"7343399a64dc2e9b22f856cb10f73df6d660eaa65bf1758f7ea30495b7d61c97".into(),
			"e4c06fbd91fb9a0dd6e33d7516a007e140f46aebc20df2f929be2425d5b2afe1".into(),
			"6272eefdd706d0b6e1fccc489e293d7b277298dfd89fd624c9d15f9a545ec4aa".into(),
			"fce2593e1cc4cc47ab31b2dbd200c65acaf7f0ceb112c640a2c5a062a898c057".into(),
			"ca50800f17f87553c5905dcd56740e07ad8422527a9d2ca3b03a98bc2ed1aff9".into(),
			"89c365695ac50e1342d854bf4bdb35066ca7b1954d04c6df4581915c5147b4df".into(),
			"9292a3e4f9af07c619ba70ca31f8aef64bae7a31154369544f32874b416b2dad".into(),
			"a40873bf277caf1bc14709cbb3f79f1391f2f7145016d8dd93e2ecf15723a007".into()
		];
		let merkle = IncrementalMerkle::new(node_count, active_nodes);

		// block ids list
		let ids_json = "v2_data/prove-action-id-list.json";
		let ids_str = read_json_from_file(ids_json).unwrap();
		let block_ids_list: Result<Vec<Vec<String>>, _> = serde_json::from_str(&ids_str);
		assert!(block_ids_list.is_ok());

		let block_ids_list: Vec<Vec<Checksum256>> = block_ids_list.as_ref().unwrap().iter().map(|ids| {
			ids.iter().map(|id| Checksum256::from_str(id).unwrap()).collect::<Vec<_>>()
		}).collect::<Vec<_>>();

		// read action merkle paths
		let action_merkle_paths_json = "v2_data/prove-action-merkle-paths.json";
		let action_merkle_paths_str = read_json_from_file(action_merkle_paths_json);
		assert!(action_merkle_paths_str.is_ok());
		let action_merkle_paths: Result<Vec<String>, _> = serde_json::from_str(&action_merkle_paths_str.unwrap());
		assert!(action_merkle_paths.is_ok());
		let action_merkle_paths = action_merkle_paths.unwrap();
		let _action_merkle_paths = {
			let mut path: Vec<Checksum256> = Vec::with_capacity(action_merkle_paths.len());
			for path_str in action_merkle_paths {
				path.push(Checksum256::from_str(&path_str).unwrap());
			}
			path
		};

		let actual_merkle_paths = vec![
			"f33eca1a95a23a69d4bac97428c67efff07e2abf9e293740d057c686eb8c7d12".into(),
			"4d3498e9702fd9b6d1253a996e7f56de064c1e9f54046cbcf18dce51df6c16e2".into()
		];

		// get action
		let actions_json = "v2_data/prove-action-action.json";
		let actions_str = read_json_from_file(actions_json);
		assert!(actions_str.is_ok());
		let actions: Result<Action, _> = serde_json::from_str(actions_str.as_ref().unwrap());
		assert!(actions.is_ok());
		let action = actions.unwrap();

		let action_receipt_str = r#"{
			"receiver": "llcllcllcllc",
			"act_digest": "9fc13ae41b29fe5d61db11e2a7d9efe0a26d107aa0c990912ecc81923c725bdd",
			"global_sequence": 35683388,
			"recv_sequence": 810,
			"auth_sequence": [
				[
					"llcllcllcllc",
					1942
				]
			],
			"code_sequence": 1,
			"abi_sequence": 1
		}
		"#;
		let action_receipt: Result<ActionReceipt, _> = serde_json::from_str(&action_receipt_str);
		assert!(action_receipt.is_ok());
		let action_receipt = action_receipt.unwrap();

		let alice = Origin::signed(1u64);

		assert_ok!(
			BridgeEos::prove_action(alice, action.clone(), action_receipt.clone(), actual_merkle_paths, merkle, signed_blocks_headers, block_ids_list, Checksum256::default())
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
		let _: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
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
