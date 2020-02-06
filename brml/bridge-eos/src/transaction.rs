#[cfg(feature = "std")]
use core::str::from_utf8;

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use eos_chain::{
	Action, Asset, Read, SerializeData, SignedTransaction, Signature, Transaction
};
#[cfg(feature = "std")]
use eos_keys::secret::SecretKey;
#[cfg(feature = "std")]
use eos_rpc::{
	get_block, get_info, GetBlock, GetInfo, HyperClient, push_transaction, PushTransaction
};
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
	#[cfg(feature = "std")]
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
	#[cfg(feature = "std")]
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

	#[cfg(feature = "std")]
	pub fn generate(self, eos_node_url: &str) -> Result<TxOut<AccountId>, Error> {
		match self {
			TxOut::Initial(mut multi_sig_tx) => {
				let hyper_client = HyperClient::new(eos_node_url);

				// fetch info
				let info: GetInfo = get_info().fetch(&hyper_client).map_err(Error::EosRpcError)?;
				let chain_id: Vec<u8> = hex::decode(info.chain_id).map_err(Error::HexError)?;
				let head_block_id = info.head_block_id;

				// fetch block
				let block: GetBlock = get_block(head_block_id).fetch(&hyper_client).map_err(Error::EosRpcError)?;
				let ref_block_num = (block.block_num & 0xffff) as u16;
				let ref_block_prefix = block.ref_block_prefix as u32;

				let actions = vec![multi_sig_tx.action.clone()];

				// Construct transaction
				let tx = Transaction::new(600, ref_block_num, ref_block_prefix, actions);
				multi_sig_tx.raw_tx = tx.to_serialize_data().map_err(Error::EosChainError)?;
				multi_sig_tx.chain_id = chain_id;

				Ok(TxOut::Generated(multi_sig_tx))
			},
			_ => Err(Error::InvalidTxOutType)
		}
	}

	#[cfg(feature = "std")]
	pub fn sign(self, sk: SecretKey, author: AccountId) -> Result<TxOut<AccountId>, Error> {
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

	#[cfg(feature = "std")]
	pub fn send(self, eos_node_url: &str) -> Result<TxOut<AccountId>, Error> {
		match self {
			TxOut::Signed(multi_sig_tx) => {
				let hyper_client = HyperClient::new(eos_node_url);

				let signatures = multi_sig_tx.multi_sig.signatures.iter()
					.map(|tx_sig|
						Signature::read(&tx_sig.signature, &mut 0).map_err(Error::EosReadError)
					)
					.map(Result::unwrap)
					.collect::<Vec<Signature>>();
				let trx = Transaction::read(&multi_sig_tx.raw_tx, &mut 0)
					.map_err(Error::EosReadError)?;
				let signed_trx = SignedTransaction {
					signatures,
					context_free_data: vec![],
					trx,
				};
				let push_tx: PushTransaction = push_transaction(signed_trx).fetch(&hyper_client)
					.map_err(Error::EosRpcError)?;
				let tx_id = hex::decode(push_tx.transaction_id).map_err(Error::HexError)?;

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
		let account_id_1= 1u32;
		let account_id_2= 1u32;

		// init tx
		let tx_out = TxOut::<u32>::init(raw_from, raw_to, asset, 2);
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