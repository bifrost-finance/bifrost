// Copyright 2019-2021 Liebi Technologies.
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

use alloc::string::{String, ToString};
use core::iter::FromIterator;

use codec::{Decode, Encode};
use iost_chain::{IostAction, Read, SerializeData, Tx};
use sp_core::offchain::Duration;
use sp_std::prelude::*;

use crate::Error;

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

impl<AccountId: PartialEq + core::fmt::Debug> MultiSig<AccountId> {
	fn new(threshold: u8) -> Self {
		MultiSig {
			signatures: Default::default(),
			threshold,
		}
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
pub struct IostMultiSigTx<AccountId, AssetId> {
	/// Chain id of Eos node that transaction will be sent
	chain_id: i32,
	/// Transaction raw data for signing
	raw_tx: Vec<u8>,
	/// Signatures of transaction
	multi_sig: MultiSig<AccountId>,
	// IOST transaction action
	action: IostAction,
	/// Who sends Transaction to EOS
	pub from: AccountId,
	/// token type
	pub token_type: AssetId,
}

/// Status of a transaction
#[derive(Encode, Decode, Clone, PartialEq, Debug)]
pub enum IostTxOut<AccountId, AssetId> {
	/// Initial Eos multi-sig transaction
	Initial(IostMultiSigTx<AccountId, AssetId>),
	/// Generated and signing Eos multi-sig transaction
	Generated(IostMultiSigTx<AccountId, AssetId>),
	/// Signed Eos multi-sig transaction
	Signed(IostMultiSigTx<AccountId, AssetId>),
	/// Sending Eos multi-sig transaction to and fetching tx id from Eos node
	Processing {
		tx_id: Vec<u8>,
		multi_sig_tx: IostMultiSigTx<AccountId, AssetId>,
	},
	/// Eos multi-sig transaction processed successfully, so only save tx id
	Success(Vec<u8>),
	/// Eos multi-sig transaction processed failed
	Fail {
		tx_id: Vec<u8>,
		reason: Vec<u8>,
		// tx: IostMultiSigTx<AccountId, AssetId>,
	},
}

impl<AccountId: PartialEq + Clone + core::fmt::Debug, AssetId> IostTxOut<AccountId, AssetId> {
	pub fn init<T: crate::Config>(
		raw_from: Vec<u8>,
		raw_to: Vec<u8>,
		amount: String,
		threshold: u8,
		memo: &str,
		from: AccountId,
		token_type: AssetId,
	) -> Result<Self, Error<T>> {
		let eos_from = core::str::from_utf8(&raw_from).map_err(|_| Error::<T>::ParseUtf8Error)?;
		let eos_to = core::str::from_utf8(&raw_to).map_err(|_| Error::<T>::ParseUtf8Error)?;

		// Construct action
		let action = IostAction::transfer(eos_from, eos_to, amount.as_str(), memo)
			.map_err(|_| Error::<T>::IostChainError)?;
		log::info!(target: "bridge-iost", "++++++++++++++++++++++++ TxOut.init is called.");

		let multi_sig_tx = IostMultiSigTx {
			chain_id: Default::default(),
			raw_tx: Default::default(),
			multi_sig: MultiSig::new(threshold),
			action,
			from,
			token_type,
		};
		Ok(IostTxOut::Initial(multi_sig_tx))
	}

	pub fn generate<T: crate::Config>(self, iost_node_url: &str) -> Result<Self, Error<T>> {
		match self {
			IostTxOut::Initial(mut multi_sig_tx) => {
				// fetch info
				let (chain_id, _head_block_id) = iost_rpc::get_info(iost_node_url)?;

				// Construct transaction
				let time = (sp_io::offchain::timestamp().unix_millis() * 1000_000) as i64;

				let expiration = time + Duration::from_millis(1000 * 1000_000).millis() as i64;

				let tx = Tx::new(
					time,
					expiration,
					chain_id as u32,
					vec![multi_sig_tx.action.clone()],
				);

				multi_sig_tx.raw_tx = tx
					.to_serialize_data()
					.map_err(|_| Error::<T>::IostChainError)?;
				multi_sig_tx.chain_id = chain_id;

				// tx.sign("admin".to_string(), iost_keys::algorithm::SECP256K1,
				//		 bs58::decode("3BZ3HWs2nWucCCvLp7FRFv1K7RR3fAjjEQccf9EJrTv4").into_vec().unwrap().as_slice());
				// log::info!(target: "bridge-iost", "tx verify {:?}", tx.verify());

				Ok(IostTxOut::Generated(multi_sig_tx))
			}
			_ => Err(Error::<T>::InvalidTxOutType),
		}
	}

	pub fn sign<T: crate::Config>(
		self,
		sk: Vec<u8>,
		account_name: &str,
		sig_algorithm: &str,
	) -> Result<Self, Error<T>> {
		match self {
			IostTxOut::Generated(mut multi_sig_tx) => {
				let mut tx = Tx::read(&multi_sig_tx.raw_tx, &mut 0)
					.map_err(|_| Error::<T>::IostChainError)?;

				let _ignore = tx.sign(account_name.to_string(), sig_algorithm, sk.as_slice());
				match tx.verify() {
					Ok(_) => {
						multi_sig_tx.raw_tx = tx
							.to_serialize_data()
							.map_err(|_| Error::<T>::IostChainError)?;
						Ok(IostTxOut::Signed(multi_sig_tx))
					}
					_ => Err(Error::<T>::IostChainError),
				}
			}
			_ => Err(Error::<T>::InvalidTxOutType),
		}
	}

	pub fn send<T: crate::Config>(self, iost_node_url: &str) -> Result<Self, Error<T>> {
		match self {
			IostTxOut::Signed(multi_sig_tx) => {
				let signed_trx = iost_rpc::serialize_push_transaction_params(&multi_sig_tx)?;

				let tx_id = iost_rpc::push_transaction(iost_node_url, signed_trx)?;

				Ok(IostTxOut::Processing {
					tx_id,
					multi_sig_tx,
				})
			}
			_ => Err(Error::<T>::InvalidTxOutType),
		}
	}
}

pub(crate) mod iost_rpc {
	use lite_json::{parse_json, JsonValue};
	use sp_runtime::offchain::http;

	use crate::Error;

	use super::*;

	const HASH: [char; 4] = ['h', 'a', 's', 'h']; // tx hash
	const CHAIN_ID: [char; 8] = ['c', 'h', 'a', 'i', 'n', '_', 'i', 'd']; // key chain_id
	const HEAD_BLOCK_HASH: [char; 15] = [
		'h', 'e', 'a', 'd', '_', 'b', 'l', 'o', 'c', 'k', '_', 'h', 'a', 's', 'h',
	]; // key head_block_hash
	const GET_INFO_API: &'static str = "/getChainInfo";
	const PUSH_TRANSACTION_API: &'static str = "/sendTx";

	type ChainId = i32;
	type HeadBlockHash = String;

	pub(crate) fn get_info<T: crate::Config>(
		node_url: &str,
	) -> Result<(ChainId, HeadBlockHash), Error<T>> {
		let req_api = format!("{}{}", node_url, GET_INFO_API);
		let pending = http::Request::get(&req_api)
			.add_header("Content-Type", "application/json")
			.send()
			.map_err(|_| Error::<T>::OffchainHttpError)?;

		let response = pending.wait().map_err(|_| Error::<T>::OffchainHttpError)?;
		let body = response.body().collect::<Vec<u8>>();
		let body_str =
			core::str::from_utf8(body.as_slice()).map_err(|_| Error::<T>::ParseUtf8Error)?;
		let node_info = parse_json(body_str).map_err(|_| Error::<T>::LiteJsonError)?;
		let mut chain_id = 0;
		let mut head_block_hash = Default::default();

		match node_info {
			JsonValue::Object(ref json) => {
				for item in json.iter() {
					if item.0 == CHAIN_ID {
						chain_id = {
							match item.1.clone() {
								JsonValue::Number(number_value) => number_value.to_f64() as i32,
								_ => return Err(Error::<T>::IOSTRpcError),
							}
						};
					}
					if item.0 == HEAD_BLOCK_HASH {
						head_block_hash = {
							match item.1 {
								JsonValue::String(ref chars) => String::from_iter(chars.iter()),
								_ => return Err(Error::<T>::IOSTRpcError),
							}
						};
					}
				}
			}
			_ => return Err(Error::<T>::IOSTRpcError),
		}
		log::info!(target: "bridge-iost", "chain_id -- {:?} head_block_hash -- {:?}.", chain_id, head_block_hash);

		if chain_id == 0 || head_block_hash == String::default() {
			return Err(Error::<T>::IOSTRpcError);
		}

		Ok((chain_id, head_block_hash))
	}

	pub(crate) fn serialize_push_transaction_params<T: crate::Config, AccountId, AssetId>(
		multi_sig_tx: &IostMultiSigTx<AccountId, AssetId>,
	) -> Result<Vec<u8>, Error<T>> {
		let tx = Tx::read(&multi_sig_tx.raw_tx, &mut 0).map_err(|_| Error::<T>::IostChainError)?;
		Ok(tx.no_std_serialize_vec())
	}

	pub(crate) fn push_transaction<T: crate::Config>(
		node_url: &str,
		signed_trx: Vec<u8>,
	) -> Result<Vec<u8>, Error<T>> {
		// log::info!(target: "bridge-iost", "signed_trx -- {:?}.", String::from_utf8_lossy(&signed_trx[..]));

		let pending = http::Request::post(
			&format!("{}{}", node_url, PUSH_TRANSACTION_API),
			vec![signed_trx],
		)
		.send()
		.map_err(|_| Error::<T>::OffchainHttpError)?;
		let response = pending.wait().map_err(|_| Error::<T>::OffchainHttpError)?;

		let body = response.body().collect::<Vec<u8>>();
		let body_str = String::from_utf8(body).map_err(|_| Error::<T>::ParseUtf8Error)?;
		let tx_id = get_transaction_id(&body_str)?;
		bs58::decode(tx_id)
			.into_vec()
			.map_err(|_| Error::<T>::DecodeBase58Error)
	}

	pub(crate) fn get_transaction_id<T: crate::Config>(
		trx_response: &str,
	) -> Result<String, Error<T>> {
		// log::info!(target: "bridge-iost", "trx_response -- {:?}.", trx_response);

		// error happens while pushing transaction to EOS node
		if !trx_response.contains("hash") && !trx_response.contains("pre_tx_receipt") {
			return Err(Error::<T>::IOSTRpcError);
		}
		let mut trx_id = String::new();
		let node_info = parse_json(trx_response).map_err(|_| Error::<T>::LiteJsonError)?;

		match node_info {
			JsonValue::Object(ref json) => {
				for item in json.iter() {
					if item.0 == HASH {
						trx_id = {
							match item.1.clone() {
								JsonValue::String(ref chars) => String::from_iter(chars.iter()),
								_ => return Err(Error::<T>::IOSTRpcError),
							}
						};
					}
				}
			}
			_ => return Err(Error::<T>::IOSTRpcError),
		}

		if trx_id.eq("") {
			return Err(Error::<T>::IOSTRpcError);
		}

		Ok(trx_id)
	}
}
