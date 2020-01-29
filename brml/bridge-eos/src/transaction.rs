#[cfg(feature = "std")]
use core::str::from_utf8;

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use eos_chain::{
	Action, Asset, PermissionLevel, Read, SerializeData, SignedTransaction, Signature, Transaction
};
#[cfg(feature = "std")]
use eos_keys::secret::SecretKey;
#[cfg(feature = "std")]
use eos_rpc::{get_block, get_info, GetBlock, GetInfo, HyperClient, push_transaction, PushTransaction};
use sp_runtime::traits::SimpleArithmetic;
use sp_std::prelude::*;

use crate::Error;

pub type TransactionSignature = Vec<u8>;

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub struct MultiSig {
	/// Collection of signature of transaction
	signatures: Vec<TransactionSignature>,
	/// Threshold of signature
	threshold: u8,
}

impl MultiSig {
	pub fn reach_threshold(&self) -> bool {
		self.signatures.len() >= self.threshold as usize
	}
}

impl Default for MultiSig {
	fn default() -> Self {
		Self {
			signatures: Default::default(),
			threshold: 1,
		}
	}
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub struct MultiSigTx<Balance> {
	/// Chain id of Eos node that transaction will be sent
	chain_id: Vec<u8>,
	/// Transaction raw data for signing
	raw_tx: Vec<u8>,
	/// Signatures of transaction
	multi_sig: MultiSig,
	amount: Balance,
	to_name: Vec<u8>,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub enum TxOut<Balance> {
	/// Generating and signing Eos multi-sig transaction
	Pending(MultiSigTx<Balance>),
	/// Sending Eos multi-sig transaction to and fetching tx id from Eos node
	Processing {
		tx_id: Vec<u8>,
		multi_sig_tx: MultiSigTx<Balance>,
	},
	/// Eos multi-sig transaction processed successfully, so only save tx id
	Success(Vec<u8>),
	/// Eos multi-sig transaction processed failed
	Fail {
		tx_id: Vec<u8>,
		reason: Vec<u8>,
		tx: MultiSigTx<Balance>,
	},
}

impl<Balance> TxOut<Balance> where Balance: SimpleArithmetic + Default + Copy {
	#[cfg(feature = "std")]
	pub fn generate_transfer(
		eos_node_url: &str,
		raw_from: Vec<u8>,
		raw_to: Vec<u8>,
		amount: Asset
	) -> Result<Self, Error> {
		let hyper_client = HyperClient::new(eos_node_url);

		// fetch info
		let info: GetInfo = get_info().fetch(&hyper_client).map_err(Error::EosRpcError)?;
		let chain_id: Vec<u8> = hex::decode(info.chain_id).map_err(Error::HexError)?;
		let head_block_id = info.head_block_id;

		// fetch block
		let block: GetBlock = get_block(head_block_id).fetch(&hyper_client).map_err(Error::EosRpcError)?;
		let ref_block_num = (block.block_num & 0xffff) as u16;
		let ref_block_prefix = block.ref_block_prefix as u32;

		let from = from_utf8(&raw_from).map_err(Error::ParseUtf8Error)?;
		let to = from_utf8(&raw_to).map_err(Error::ParseUtf8Error)?;

		// Construct action
		let permission_level = PermissionLevel::from_str(
			from,
			"active",
		).map_err(Error::EosChainError)?;

		let memo = "a memo";
		let action = Action::transfer(from, to, amount.to_string().as_ref(), memo)
			.map_err(Error::EosChainError)?;

		let actions = vec![action];

		// Construct transaction
		let tx = Transaction::new(600, ref_block_num, ref_block_prefix, actions);
		let multi_sig_tx = MultiSigTx {
			raw_tx: tx.to_serialize_data().map_err(Error::EosChainError)?,
			chain_id,
			multi_sig: Default::default(),
			amount: Default::default(),
			to_name: vec![],
		};

		Ok(TxOut::Pending(multi_sig_tx))
	}

	pub fn reach_threshold(&self) -> bool {
		match self {
			TxOut::Pending(multi_sig_tx) => multi_sig_tx.multi_sig.reach_threshold(),
			_ => false,
		}
	}

	#[cfg(feature = "std")]
	pub fn sign(&mut self, sk: SecretKey) -> Result<Self, Error> {
		match self {
			TxOut::Pending(ref mut multi_sig_tx) => {
				let chain_id = &multi_sig_tx.chain_id;
				let trx = Transaction::read(&multi_sig_tx.raw_tx, &mut 0).map_err(Error::EosReadError)?;
				let sig: Signature = trx.sign(sk, chain_id.clone()).map_err(Error::EosChainError)?;
				let sig_hex_data = sig.to_serialize_data().map_err(Error::EosChainError)?;
				multi_sig_tx.multi_sig.signatures.push(sig_hex_data);

				Ok(self.clone())
			},
			_ => Err(Error::InvalidTxOutType)
		}
	}

	#[cfg(feature = "std")]
	pub fn send(&self, eos_node_url: &str) -> Result<Self, Error> {
		match self {
			TxOut::Pending(ref multi_sig_tx) => {
				let hyper_client = HyperClient::new(eos_node_url);

				let signatures = multi_sig_tx.multi_sig.signatures.iter()
					.map(|sig|
						Signature::read(&sig, &mut 0).map_err(Error::EosReadError)
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
					multi_sig_tx: multi_sig_tx.clone(),
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
	fn tx_send_should_work() {
		let eos_node_url: &'static str = "http://127.0.0.1:8888/";
		let raw_from = vec![0x62, 0x69, 0x66, 0x72, 0x6F, 0x73, 0x74]; // bifrost
		let raw_to = vec![0x61, 0x6C, 0x69, 0x63, 0x65]; // alice
		let sym = Symbol::from_str("4,EOS").unwrap();
		let asset = Asset::new(1i64, sym);

		// generate tx
		let tx_out = TxOut::<u128>::generate_transfer(eos_node_url, raw_from, raw_to, asset);
		assert!(tx_out.is_ok());

		// sign tx
		let mut tx_out = tx_out.unwrap();
		let sk = SecretKey::from_wif("5HrPPFF2hq1X8ktBVfUVubeAmSaerRHwz2aGxGSUqvAuaNhR8a5").unwrap();
		let tx_out= tx_out.sign(sk);
		assert!(tx_out.is_ok());

		// send tx
		let tx_out = tx_out.unwrap();
		let tx_out = tx_out.send(eos_node_url);
		assert!(tx_out.is_ok());
	}
}