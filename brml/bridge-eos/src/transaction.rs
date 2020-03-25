use alloc::string::String;
use alloc::string::ToString;
use core::str::from_utf8;
use codec::{Decode, Encode};
use crate::Error;
use eos_chain::{
	Action, Asset, Read, SerializeData, SignedTransaction, Signature, Transaction
};
use eos_keys::secret::SecretKey;
use lite_json::{parse_json, JsonValue, Serialize, JsonObject};
use sp_core::offchain::Duration;
use sp_std::prelude::*;
use sp_runtime::offchain::http;

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
		let from = from_utf8(&raw_from).map_err(Error::ParseUtf8Error)?;
		let to = from_utf8(&raw_to).map_err(Error::ParseUtf8Error)?;

		// Construct action
		let memo = "a memo";
		let action = Action::transfer(from, to, amount.to_string().as_ref(), memo)
			.map_err(Error::EosChainError)?;

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
				let pending = http::Request::post(&eos_node_url, vec![b" "])
					.send()
					.map_err(|_| "Error in waiting http response back")?;
				let response = pending.wait()
					.map_err(|_| "Error in waiting http response back")?;
				let body = response.body().collect::<Vec<u8>>();
				let body_str= core::str::from_utf8(body.as_slice()).map_err(|_| "Error string conversion failed")?;
				let json_val = parse_json(body_str).map_err(|_| "Error deserialization failed")?;
				let mut chain_ids = vec![];
				let mut ref_block_num= 0;
				let mut ref_block_prefix= 0;
				match json_val {
					JsonValue::Object(ref obj ) => {
						let act: &JsonObject = obj;
						for a in act.iter() {
							let u8_vec = a.0.iter().map(|c| *c as u8).collect::<Vec<_>>();
							let key = String::from_utf8(u8_vec).map_err(|_| "Error string conversion failed")?;
							if key == "chain_id" {
								let mut vec: Vec<u8> = vec![];
								a.1.serialize_to(&mut vec,0,0);
								let value = String::from_utf8(vec).map_err(|_| "Error string conversion failed")?;
								let chain_id: Vec<u8> = hex::decode(value).map_err(Error::HexError)?;
								chain_ids = chain_id;
							} else if key == "head_block_id"{
								let mut block_vec: Vec<u8> = vec![];
								a.1.serialize_to(&mut block_vec,0,0);
								let head_block_id = String::from_utf8(block_vec).map_err(|_| "Error string conversion failed")?;
								let head_block_id_vec = head_block_id.as_bytes();
								let post_vec = vec![head_block_id_vec];
								let pending = http::Request::post(&eos_node_url, post_vec)
									.send()
									.map_err(|_| "Error in waiting http response back")?;
								let response = pending.wait()
									.map_err(|_| "Error in waiting http response back")?;
								let body = response.body().collect::<Vec<u8>>();
								let body_str = String::from_utf8(body).map_err(|_| "Error cannot convert to string")?;
								let json_value_get_block = parse_json(body_str.as_str()).map_err(|_| "Error deserialization failed")?;
								match json_value_get_block {
									JsonValue::Object(ref obj ) => {
										let get_block_act: &JsonObject = obj;
										for a in get_block_act.iter() {
											let u8_vec_get_block = a.0.iter().map(|c| *c as u8).collect::<Vec<_>>();
											let keys = String::from_utf8(u8_vec_get_block).map_err(|_| "Error string conversion failed")?;
											if keys == "block_num" {
												let mut block_num_vec: Vec<u8> = vec![];
												a.1.serialize_to(&mut block_num_vec,0,0);
												let pack_date = block_num_vec.as_slice().as_ptr() as u64;
												let ref_block_num_json = (pack_date & 0xffff) as u16;
												ref_block_num = ref_block_num_json;
											} else if keys == "ref_block_prefix" {
												let mut ref_block_prefix_vec: Vec<u8> = vec![];
												a.1.serialize_to(&mut ref_block_prefix_vec,0,0);
												let ptr = ref_block_prefix_vec.as_slice().as_ptr() as u32;
												ref_block_prefix = ptr;
											}
										}
									}
									_ => {}
								}
							};
						}
					}
					_ => {}
				};
				let actions = vec![multi_sig_tx.action.clone()];
				// Construct transaction
				let expiration = (sp_io::offchain::timestamp().add(Duration::from_millis(600 * 1000)).unix_millis() as f64 / 1000.0) as u32;
				let tx = Transaction::new(expiration, ref_block_num, ref_block_prefix, actions);
				multi_sig_tx.raw_tx = tx.to_serialize_data().map_err(Error::EosChainError)?;
				multi_sig_tx.chain_id = chain_ids;

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
				let signatures = multi_sig_tx.multi_sig.signatures.iter()
					.map(|tx_sig|
						Signature::read(&tx_sig.signature, &mut 0).map_err(Error::EosReadError)
					)
					.map(Result::unwrap)
					.collect::<Vec<Signature>>();
				let trx = Transaction::read(&multi_sig_tx.raw_tx, &mut 0)
					.map_err(Error::EosReadError)?;
				let len = signatures.len();
				let serialized_sigs =    {
					let mut t = Vec::with_capacity(len);
					for sig in signatures.iter() {
						let s = sig.to_serialize_data().unwrap().iter().map(|c| *c as char).collect::<Vec<_>>();
						let val = JsonValue::String(s);
						t.push(val);
					}
					t
				};

				let signed_trx = JsonValue::Object(vec![
						(
							b"signatures".iter().map(|c| *c as char).collect::<Vec<_>>(),
							JsonValue::Array(serialized_sigs),
						),
						(
							b"context_free_data".iter().map(|c| *c as char).collect::<Vec<_>>(),
							JsonValue::Array(Vec::new()),
						),
						(
							b"trx".iter().map(|c| *c as char).collect::<Vec<_>>(),
							JsonValue::String(trx.to_serialize_data().unwrap().iter().map(|c| *c as char).collect::<Vec<_>>()),
						),
					]
				).serialize();

				let vec = vec![signed_trx];
				let pending = http::Request::post(&eos_node_url, vec)
					.send()
					.map_err(|_| "Error in waiting http response back")?;
				let response = pending.wait()
					.map_err(|_| "Error in waiting http response back")?;
				let body = response.body().collect::<Vec<u8>>();
				let body_str = String::from_utf8(body).map_err(|_| "Error cannot convert to string")?;
				let json_value_push_transaction = parse_json(body_str.as_str()).map_err(|_| "Error deserialization failed")?;
				let mut transaction_vec = vec![];
				match json_value_push_transaction {
					JsonValue::Object(ref obj ) => {
						let get_block_act: &JsonObject = obj;
						for a in get_block_act.iter() {
							let u8_vec_get_block = a.0.iter().map(|c| *c as u8).collect::<Vec<_>>();
							let keys = String::from_utf8(u8_vec_get_block).map_err(|_| "Error string conversion failed")?;
							if keys == "transaction_id" {
								let mut transaction_id_vec: Vec<u8> = vec![];
								a.1.serialize_to(&mut transaction_id_vec,0,0);
								transaction_vec = transaction_id_vec;
//								let transaction_id = String::from_utf8(block_vec).map_err(|_| "Error string conversion failed")?;
							}
						}
					}
					_ => {}
				}
				let transaction_id = String::from_utf8(transaction_vec).map_err(|_| "Error string conversion failed")?;
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