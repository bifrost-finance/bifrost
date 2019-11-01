use codec::{Encode, Decode};
use rstd::prelude::*;
use substrate_primitives::offchain::Timestamp;
#[cfg(feature = "std")]
use eos_primitives::{Transaction, PermissionLevel, Action, Asset, Symbol};
#[cfg(feature = "std")]
use eos_rpc::{HyperClient, GetInfo, GetBlock, get_info, get_block, push_transaction};
#[cfg(feature = "std")]
use std::str::FromStr;
use sr_primitives::traits::{SimpleArithmetic, SaturatedConversion};

pub type TransactionSignature = Vec<u8>;

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug, Default)]
pub struct SignatureCollection {
	/// Collection of signature of transaction
	signatures: Vec<TransactionSignature>,
	/// Threshold of signature
	threshold: u8
}

impl SignatureCollection {
	pub fn reach_threshold(&self) -> bool {
		self.signatures.len() >= self.threshold as usize
	}
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub enum TransactionStatus {
	Generated,
	Signed,
	Sent,
	GenerateError,
	SignError,
	SendError,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub struct TransactionIn {
	/// Transaction raw data for signing
	raw: Vec<u8>,
	/// Collection of signature of transaction
	pub signatures: SignatureCollection,
	/// Threshold of signature
	threshold: u8,
	/// Status of transaction
	status: TransactionStatus,
}

impl TransactionIn {
	pub fn new() -> Self {
		Self {
			raw: vec![],
			signatures: Default::default(),
			status: TransactionStatus::Generated,
			threshold: 1,
		}
	}
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub struct TransactionOut<Balance> {
	/// Transaction raw data for signing
	raw: Vec<u8>,
	pub signatures: SignatureCollection,
	/// Status of transaction
	status: TransactionStatus,

	pub amount: Balance,
	pub to_name: Vec<u8>,
}

impl<Balance> TransactionOut<Balance> where Balance: SimpleArithmetic + Default + Copy
{
	pub fn new() -> Self {
		Self {
			raw: vec![],
			signatures: Default::default(),
			status: TransactionStatus::Generated,
			amount: Default::default(),
			to_name: vec![],
		}
	}

	pub fn reach_threshold(&self) -> bool {
		self.signatures.reach_threshold()
	}

	pub fn reach_timestamp(&self, t: Timestamp) -> bool { true }

	// 生成`交易接收确认`交易
	#[cfg(feature = "std")]
	pub fn generate_unsigned_recv_tx(&self) -> Result<Self, crate::Error> {
		// import private key
		let sk = eos_keys::secret::SecretKey::from_wif("5KQwrPbwdL6PhXujxW37FSSQZ1JiwsST4cqQzDeyXtP79zkvFD3");
		assert!(sk.is_ok());
		let sk = sk.map_err(crate::Error::SecretKeyError)?;

		let node: &'static str = "http://47.101.139.226:8888/";
		let hyper_client = HyperClient::new(node);

		// fetch info
		let info: GetInfo = get_info().fetch(&hyper_client)
			.map_err(crate::Error::HttpResponseError)?;
		let chain_id = info.chain_id;
		let head_block_id = info.head_block_id;

		// fetch block
		let block: GetBlock = get_block(head_block_id).fetch(&hyper_client)
			.map_err(crate::Error::HttpResponseError)?;
		let ref_block_num = (block.block_num & 0xffff) as u16;
		let ref_block_prefix = block.ref_block_prefix as u32;

		// Construct action
		let permission_level = PermissionLevel::from_str(
			"alice",
			"active"
		).map_err(crate::Error::EosPrimitivesError)?;

		let to = core::str::from_utf8(&self.to_name).map_err(crate::Error::ParseUtf8Error)?;
		let eos_symbol = Symbol::from_str("4,EOS").map_err(crate::Error::from)?;
		let amount = Asset {
			amount: (self.amount.saturated_into::<u128>() / (10u128.pow(12 - eos_symbol.precision() as u32))) as i64,
			symbol: eos_symbol,
		};
		let memo = "a memo";
		let action = Action::transfer("alice", to, amount.to_string().as_ref(), memo)
			.map_err(crate::Error::EosPrimitivesError)?;

		let actions = vec![action];

		// Construct transaction
		let trx = Transaction::new(600, ref_block_num, ref_block_prefix, actions);
		let signed_trx = trx.sign(sk, chain_id).map_err(crate::Error::EosPrimitivesError)?;
		let res: PushTransaction = push_transaction(signed_trx).fetch(&hyper_client)
            .map_err(crate::Error::HttpResponseError)?;

		Ok(TransactionOut::new())
	}
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub enum BridgeDirection {
	In,
	Out,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub struct BridgeTransaction {
	direction: BridgeDirection,
	transaction: Vec<u8>,
	signatures: Vec<TransactionSignature>
}
