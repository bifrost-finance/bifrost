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

use alloc::borrow::ToOwned;
use alloc::string::{String, ToString};
use core::iter::FromIterator;
use codec::{Decode, Encode};
use crate::Error;
use eos_chain::{Action, Asset, Read, SerializeData, Signature, Transaction};
use eos_keys::secret::SecretKey;
use sp_core::offchain::Duration;
use sp_std::prelude::*;

#[derive(Encode, Decode, Clone, PartialEq, Debug, Default)]
pub struct TxSig<AccountId> {
	signature: Vec<u8>,
	author: AccountId,
}

#[derive(Encode, Decode, Clone, PartialEq, Debug)]
pub struct MultiSig<AccountId> {
	/// Collection of signature of transaction
	signatures: Vec<TxSig<AccountId>>,
	/// Threshold of signature
	threshold: u8,
}

impl<AccountId: PartialEq> MultiSig<AccountId> {
	fn new(threshold: u8) -> Self {
		MultiSig {
			signatures: Default::default(),
			threshold,
		}
	}

	fn reach_threshold(&self) -> bool {
		self.signatures.len() >= self.threshold as usize
	}

	fn has_signed(&self, author: AccountId) -> bool {
		self.signatures.iter().find(|sig| sig.author == author).is_some()
	}
}

impl<AccountId> Default for MultiSig<AccountId> {
	fn default() -> Self {
		Self {
			signatures: Default::default(),
			threshold: 1,
		}
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Debug)]
pub struct MultiSigTx<AccountId> {
	/// Chain id of Eos node that transaction will be sent
	chain_id: Vec<u8>,
	/// Transaction raw data for signing
	raw_tx: Vec<u8>,
	/// Signatures of transaction
	multi_sig: MultiSig<AccountId>,
	action: Action,
}

#[derive(Encode, Decode, Clone, PartialEq, Debug)]
pub enum TxOut<AccountId> {
	/// Initial Eos multi-sig transaction
	Initial(MultiSigTx<AccountId>),
	/// Generated and signing Eos multi-sig transaction
	Generated(MultiSigTx<AccountId>),
	/// Signed Eos multi-sig transaction
	Signed(MultiSigTx<AccountId>),
	/// Sending Eos multi-sig transaction to and fetching tx id from Eos node
	Processing {
		tx_id: Vec<u8>,
		multi_sig_tx: MultiSigTx<AccountId>,
	},
	/// Eos multi-sig transaction processed successfully, so only save tx id
	Success(Vec<u8>),
	/// Eos multi-sig transaction processed failed
	Fail {
		tx_id: Vec<u8>,
		reason: Vec<u8>,
		tx: MultiSigTx<AccountId>,
	},
}

impl<AccountId: PartialEq + Clone> TxOut<AccountId> {
	pub fn init(
		raw_from: Vec<u8>,
		raw_to: Vec<u8>,
		amount: Asset,
		threshold: u8,
	) -> Result<Self, Error> {
		let from = core::str::from_utf8(&raw_from).map_err(Error::ParseUtf8Error)?;
		let to = core::str::from_utf8(&raw_to).map_err(Error::ParseUtf8Error)?;

		// Construct action
		let memo = "a memo";
		let action = Action::transfer(from, to, amount.to_string().as_ref(), memo).map_err(Error::EosChainError)?;

		// Construct transaction
		let multi_sig_tx = MultiSigTx {
			chain_id: Default::default(),
			raw_tx: Default::default(),
			multi_sig: MultiSig::new(threshold),
			action,
		};
		Ok(TxOut::Initial(multi_sig_tx))
	}

	pub fn generate(self, eos_node_url: &str) -> Result<Self, Error> {
		match self {
			TxOut::Initial(mut multi_sig_tx) => {
				// fetch info
				let (chain_id, head_block_id) = eos_rpc::get_info(eos_node_url)?;
				let chain_id: Vec<u8> = hex::decode(chain_id).map_err(Error::HexError)?;

				// fetch block
				let (ref_block_num, ref_block_prefix) = eos_rpc::get_block(eos_node_url, head_block_id)?;

				let actions = vec![multi_sig_tx.action.clone()];
				// Construct transaction
				let expiration = (sp_io::offchain::timestamp().add(Duration::from_millis(600 * 1000)).unix_millis() as f64 / 1000.0) as u32;
				let tx = Transaction::new(expiration, ref_block_num, ref_block_prefix, actions);
				multi_sig_tx.raw_tx = tx.to_serialize_data().map_err(Error::EosChainError)?;
				multi_sig_tx.chain_id = chain_id;

				Ok(TxOut::Generated(multi_sig_tx))
			},
			_ => Err(Error::InvalidTxOutType)
		}
	}

	pub fn sign(self, sk: SecretKey, author: AccountId) -> Result<Self, Error> {
		match self {
			TxOut::Generated(mut multi_sig_tx) => {
				if multi_sig_tx.multi_sig.has_signed(author.clone()) {
					return Err(Error::AlreadySignedByAuthor);
				}

				let chain_id = &multi_sig_tx.chain_id;
				let trx = Transaction::read(&multi_sig_tx.raw_tx, &mut 0).map_err(Error::EosReadError)?;
				let sig: Signature = trx.sign(sk, chain_id.clone()).map_err(Error::EosChainError)?;
				let sig_hex_data = sig.to_serialize_data().map_err(Error::EosChainError)?;
				multi_sig_tx.multi_sig.signatures.push(TxSig {author, signature: sig_hex_data});

				if multi_sig_tx.multi_sig.reach_threshold() {
					Ok(TxOut::Signed(multi_sig_tx))
				} else {
					Ok(TxOut::Generated(multi_sig_tx))
				}
			},
			_ => Err(Error::InvalidTxOutType)
		}
	}

	pub fn send(self, eos_node_url: &str) -> Result<TxOut<AccountId>, Error> {
		match self {
			TxOut::Signed(multi_sig_tx) => {
				let signed_trx = eos_rpc::serialize_push_transaction_params(&multi_sig_tx)?;

				let transaction_vec = eos_rpc::push_transaction(eos_node_url, signed_trx)?;

				let transaction_id = core::str::from_utf8(transaction_vec.as_slice()).map_err(|_| "Error string conversion failed")?;
				let tx_id = hex::decode(transaction_id).map_err(Error::HexError)?;

				Ok(TxOut::Processing {
					tx_id,
					multi_sig_tx,
				})
			},
			_ => Err(Error::InvalidTxOutType)
		}
	}
}

pub(crate) mod eos_rpc {
	use alloc::collections::btree_map::BTreeMap;
	use alloc::string::ToString;
	use crate::Error;
	use lite_json::{parse_json, JsonValue, Serialize};
	use sp_runtime::offchain::http;
	use super::*;

	const CHAIN_ID: [char; 8] = ['c', 'h', 'a', 'i', 'n', '_', 'i', 'd']; // key chain_id
	const HEAD_BLOCK_ID: [char; 13] = ['h', 'e', 'a', 'd', '_', 'b', 'l', 'o', 'c', 'k', '_', 'i', 'd']; // key head_block_id
	const GET_INFO_API: &'static str = "/v1/chain/get_info";
	const GET_BLOCK_API: &'static str = "/v1/chain/get_block";
	const PUSH_TRANSACTION_API: &'static str = "/v1/chain/push_transaction";

	type ChainId = String;
	type HeadBlockId = String;
	type BlockNum = u16;
	type RefBlockPrefix = u32;

	pub(crate) fn get_info(node_url: &str) -> Result<(ChainId, HeadBlockId), Error> {
		let req_api = format!("{}{}", node_url, GET_INFO_API);
		let pending = http::Request::post(&req_api, vec![b"{}"])
			.add_header("Content-Type", "application/json")
			.send().map_err(|_| "Http request error")?;
		let response = pending.wait().map_err(|_| "Http request error")?;

		let body = response.body().collect::<Vec<u8>>();
		let body_str= core::str::from_utf8(body.as_slice()).map_err(|_| "Error string conversion failed")?;
		let node_info = parse_json(body_str).map_err(|_| "Deserialization node info failure")?;

		let mut chain_id = Default::default();
		let mut head_block_id = Default::default();

		match node_info {
			JsonValue::Object(ref json) => {
				for item in json.iter() {
					if item.0 == CHAIN_ID {
						chain_id = {
							match item.1 {
								JsonValue::String(ref chars) => String::from_iter(chars.iter()),
								_ => return Err(Error::EOSRpcError("Failed to parse eos node info.".to_owned())),
							}
						};
					}
					if item.0 == HEAD_BLOCK_ID {
						head_block_id = {
							match item.1 {
								JsonValue::String(ref chars) => String::from_iter(chars.iter()),
								_ => return Err(Error::EOSRpcError("Failed to parse eos node info.".to_owned())),
							}
						};
					}
				}
			}
			_ => return Err(Error::EOSRpcError("Failed to parse eos node info.".to_owned())),
		}
		if chain_id == String::default() || head_block_id == String::default() {
			return Err(Error::EOSRpcError("Failed to parse eos node info.".to_owned()));
		}

		Ok((chain_id, head_block_id))
	}

	pub(crate) fn get_block(node_url: &str, head_block_id: String) -> Result<(BlockNum, RefBlockPrefix), Error> {
		let req_body = {
			JsonValue::Object(vec![
				(
					"block_num_or_id".chars().collect::<Vec<_>>(),
					JsonValue::String(head_block_id.chars().collect::<Vec<_>>()),
				),
			]).serialize()
		};
		let pending = http::Request::post(&format!("{}{}", node_url, GET_BLOCK_API), vec![req_body.as_slice()])
			.add_header("Content-Type", "application/json")
			.send().map_err(|_| "Error in waiting http response back")?;
		let response = pending.wait().map_err(|_| "Error in waiting http response back")?;

		let body = response.body().collect::<Vec<u8>>();
		let body_str = core::str::from_utf8(body.as_slice()).map_err(|_| "Error cannot convert to string")?;

		let maps = body_str.trim_matches(|c| c == '{' || c == '}')
			.split(',').into_iter().filter_map(|i| {
			if i.rfind("block_num").is_some() || i.rfind("ref_block_prefix").is_some() {
				match i.split(':').collect::<Vec<&str>>().as_slice() {
					[key, val] => Some((key.clone(), val.clone())),
					_ => None
				}
			} else {
				None
			}
		}).collect::<BTreeMap<_, _>>();

		if maps.is_empty() {
			return Err(Error::EOSRpcError("Failed to parse eos block.".to_owned()));
		}

		let block_num = {
			let num_str = maps.get("\"block_num\"").ok_or("cannot find block_num")?;
			let block_num: u64 = num_str.parse().map_err(|_| "parse block number error")?;
			(block_num & 0xffff) as u16
		};
		let ref_block_prefix = {
			let prefix = maps.get("\"ref_block_prefix\"").ok_or("cannot find ref_block_prefix")?;
			let prefix_num: u32 = prefix.parse().map_err(|_| "parse block number error")?;
			prefix_num
		};

		Ok((block_num, ref_block_prefix))
	}

	pub(crate) fn push_transaction(node_url: &str, signed_trx: Vec<u8>) -> Result<Vec<u8>, Error>{
		let pending = http::Request::post(&format!("{}{}", node_url, PUSH_TRANSACTION_API), vec![signed_trx]).send().map_err(|_| "Error in waiting http response back")?;
		let response = pending.wait().map_err(|_| "Error in waiting http response back")?;

		let body = response.body().collect::<Vec<u8>>();
		let body_str = String::from_utf8(body).map_err(|_| "Error cannot convert to string")?;
		let tx_id = get_transaction_id(&body_str)?;

		Ok(tx_id.into_bytes())
	}

	pub(crate) fn serialize_push_transaction_params<AccountId>(multi_sig_tx: &MultiSigTx<AccountId>) -> Result<Vec<u8>, Error> {
		let serialized_signatures = {
			let mut serialized_signatures = Vec::with_capacity(multi_sig_tx.multi_sig.signatures.len());
			for tx_sig in multi_sig_tx.multi_sig.signatures.iter() {
				let sig = Signature::read(&tx_sig.signature, &mut 0).map_err(Error::EosReadError)?;
				let val = JsonValue::String(sig.to_string().chars().collect());
				serialized_signatures.push(val);
			}
			serialized_signatures
		};

		let signed_trx = JsonValue::Object(vec![
			(
				"signatures".chars().collect::<Vec<_>>(),
				JsonValue::Array(serialized_signatures),
			),
			(
				"compression".chars().collect::<Vec<_>>(),
				JsonValue::String("none".chars().collect()),
			),
			(
				"packed_context_free_data".chars().collect::<Vec<_>>(),
				JsonValue::String(Vec::new()),
			),
			(
				"packed_trx".chars().collect::<Vec<_>>(),
				JsonValue::String(
					hex::encode(&multi_sig_tx.raw_tx).chars().collect()
				),
			),
		]).serialize();

		Ok(signed_trx)
	}

	pub(crate) fn get_transaction_id(trx_response: &str) -> Result<String, Error> {
		// error happens while pushing transaction to EOS node
		if !trx_response.contains("transaction_id") && !trx_response.contains("processed") {
			return Err(Error::EOSRpcError(trx_response.to_string()));
		}

		let mut trx_id = String::new();
		let splited_strs: Vec<&str> = trx_response.trim_matches(|c| c == '{' || c == '}').split("processed").collect();
		for s in &splited_strs {
			if s.contains("transaction_id") {
				trx_id = s.replace("transaction_id", "").chars().filter(|c| c.is_numeric() || c.is_alphabetic()).collect();
				break;
			}
		}

		if trx_id.eq("") {
			return Err(Error::EOSRpcError("cannot find transaction id in rpc response.".to_string()));
		}

		Ok(trx_id)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use eos_chain::Symbol;
	use sp_std::str::FromStr;

	#[test]
	fn tx_send_with_multisig_should_work() {
		let eos_node_url: &'static str = "http://127.0.0.1:8888/";
		let raw_from = b"bifrost".to_vec();
		let raw_to = b"alice".to_vec();
		let sym = Symbol::from_str("4,EOS").unwrap();
		let asset = Asset::new(1i64, sym);
		let account_id_1= 1u64;
		let account_id_2= 2u64;

		// init tx
		let tx_out = TxOut::<u64>::init(raw_from, raw_to, asset, 2);
		assert!(tx_out.is_ok());

		// generate Eos raw tx
		let tx_out = tx_out.unwrap();
		let tx_out = tx_out.generate(eos_node_url);
		assert!(tx_out.is_ok());

		// sign tx by account testa
		let tx_out = tx_out.unwrap();
		let sk = SecretKey::from_wif("5JgbL2ZnoEAhTudReWH1RnMuQS6DBeLZt4ucV6t8aymVEuYg7sr").unwrap();
		let tx_out = tx_out.sign(sk, account_id_1);
		assert!(tx_out.is_ok());

		// tx by account testb
		let tx_out = tx_out.unwrap();
		let sk = SecretKey::from_wif("5J6vV6xbVV2UEwBYYDRQQ8yTDcSmHJw67XqRriF4EkEzWKUFNKj").unwrap();
		let tx_out = tx_out.sign(sk, account_id_2);
		assert!(tx_out.is_ok());

		// send tx
		let tx_out = tx_out.unwrap();
		let tx_out = tx_out.send(eos_node_url);
		assert!(tx_out.is_ok());
	}
}
