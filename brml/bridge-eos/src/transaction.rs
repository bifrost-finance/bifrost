use codec::{Encode, Decode};
use rstd::prelude::*;
use substrate_primitives::offchain::Timestamp;
#[cfg(feature = "std")]
use eos_chain::{Transaction, PermissionLevel, Action, Asset, Symbol, SignedTransaction, Read};
#[cfg(feature = "std")]
use eos_rpc::{HyperClient, GetInfo, GetBlock, get_info, get_block, push_transaction, PushTransaction};
use sr_primitives::traits::{SimpleArithmetic, SaturatedConversion};
use eos_chain::{SerializeData, Signature};
use core::str::from_utf8;

pub type TransactionSignature = Vec<u8>;

#[cfg(feature = "std")]
#[derive(Debug)]
pub enum Error {
	InvalidTxOutType,
	EosPrimitivesError(eos_chain::Error),
	EosReadError(eos_chain::bytes::ReadError),
	EosSerializationDataError(eos_chain::Error),
	HttpResponseError(eos_rpc::Error),
	ParseUtf8Error(core::str::Utf8Error),
	SecretKeyError(eos_keys::error::Error),
	HexError(hex::FromHexError),
}

#[cfg(feature = "std")]
impl core::convert::From<eos_chain::symbol::ParseSymbolError> for Error {
	fn from(err: eos_chain::symbol::ParseSymbolError) -> Self {
		Self::EosPrimitivesError(eos_chain::Error::ParseSymbolError(err))
	}
}

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
	pub fn genrate_transfer(
		raw_from: Vec<u8>,
		raw_to: Vec<u8>,
		amount: Asset
	) -> Result<Self, Error> {
		let node: &'static str = "http://47.101.139.226:8888/";
		let hyper_client = HyperClient::new(node);

		// fetch info
		let info: GetInfo = get_info().fetch(&hyper_client).map_err(Error::HttpResponseError)?;
		let chain_id: Vec<u8> = hex::decode(info.chain_id).map_err(Error::HexError)?;
		let head_block_id = info.head_block_id;

		// fetch block
		let block: GetBlock = get_block(head_block_id).fetch(&hyper_client).map_err(Error::HttpResponseError)?;
		let ref_block_num = (block.block_num & 0xffff) as u16;
		let ref_block_prefix = block.ref_block_prefix as u32;

		let from = core::str::from_utf8(&raw_from).map_err(Error::ParseUtf8Error)?;
		let to = core::str::from_utf8(&raw_to).map_err(Error::ParseUtf8Error)?;

		// Construct action
		let permission_level = PermissionLevel::from_str(
			from,
			"active",
		).map_err(Error::EosPrimitivesError)?;

		let memo = "a memo";
		let action = Action::transfer(from, to, amount.to_string().as_ref(), memo)
			.map_err(Error::EosPrimitivesError)?;

		let actions = vec![action];

		// Construct transaction
		let tx = Transaction::new(600, ref_block_num, ref_block_prefix, actions);
		let multi_sig_tx = MultiSigTx {
			raw_tx: tx.to_serialize_data().map_err(self::Error::EosSerializationDataError)?,
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
	pub fn sign(&mut self) -> Result<Self, Error> {
		// import private key
		let sk = eos_keys::secret::SecretKey::from_wif("5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3")
			.map_err(Error::SecretKeyError)?;

		match self {
			TxOut::Pending(ref mut multi_sig_tx) => {
				let chain_id = &multi_sig_tx.chain_id;
				let trx = Transaction::read(&multi_sig_tx.raw_tx, &mut 0).map_err(Error::EosReadError)?;
				let sig: Signature = trx.sign(sk, chain_id.clone()).map_err(Error::EosPrimitivesError)?;
				let sig_hex_data = sig.to_serialize_data().map_err(self::Error::EosSerializationDataError)?;
				multi_sig_tx.multi_sig.signatures.push(sig_hex_data);

				Ok(self.clone())
			},
			_ => Err(Error::InvalidTxOutType)
		}
	}

	#[cfg(feature = "std")]
	pub fn send(&self) -> Result<Self, Error> {
		match self {
			TxOut::Pending(ref multi_sig_tx) => {
				let node: &'static str = "http://47.101.139.226:8888/";
				let hyper_client = HyperClient::new(node);

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
					.map_err(Error::HttpResponseError)?;
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
