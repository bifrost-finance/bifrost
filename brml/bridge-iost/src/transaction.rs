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

use crate::Error;
use alloc::string::{String, ToString};
use codec::{Decode, Encode};
use core::{iter::FromIterator, str::FromStr};
use frame_support::debug;
use iost_chain::{Action, Read, SerializeData, Tx};
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
        self.signatures
            .iter()
            .find(|sig| sig.author == author)
            .is_some()
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
    chain_id: i32,
    /// Transaction raw data for signing
    raw_tx: Vec<u8>,
    /// Signatures of transaction
    multi_sig: MultiSig<AccountId>,
    // IOST transaction action
    action: Action,
    /// Who sends Transaction to EOS
    pub from: AccountId,
    /// token type
    pub token_type: node_primitives::TokenSymbol,
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
    pub fn init<T: crate::Trait>(
        raw_from: Vec<u8>,
        raw_to: Vec<u8>,
        amount: String,
        threshold: u8,
        memo: &str,
        from: AccountId,
        token_type: node_primitives::TokenSymbol,
    ) -> Result<Self, Error<T>> {
        let eos_from = core::str::from_utf8(&raw_from).map_err(|_| Error::<T>::ParseUtf8Error)?;
        let eos_to = core::str::from_utf8(&raw_to).map_err(|_| Error::<T>::ParseUtf8Error)?;

        // Construct action
        let action = Action::transfer(eos_from, eos_to, amount.as_str(), memo)
            .map_err(|_| Error::<T>::IostChainError)?;
        debug::info!(target: "bridge-iost", "++++++++++++++++++++++++ TxOut.init is called.");

        let multi_sig_tx = MultiSigTx {
            chain_id: Default::default(),
            raw_tx: Default::default(),
            multi_sig: MultiSig::new(threshold),
            action,
            from,
            token_type,
        };
        Ok(TxOut::Initial(multi_sig_tx))
    }

    pub fn generate<T: crate::Trait>(self, iost_node_url: &str) -> Result<Self, Error<T>> {
        match self {
            TxOut::Initial(mut multi_sig_tx) => {
                // fetch info
                let (chain_id, head_block_id) = iost_rpc::get_info(iost_node_url)?;

                // Construct transaction
                let time = sp_io::offchain::timestamp().unix_millis() as i64;
                debug::info!(target: "bridge-iost", "tx timestamp {:?}", time);

                let expiration = time + Duration::from_millis(1000 * 10000).millis() as i64;

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
                //         bs58::decode("3BZ3HWs2nWucCCvLp7FRFv1K7RR3fAjjEQccf9EJrTv4").into_vec().unwrap().as_slice());
                // debug::info!(target: "bridge-iost", "tx verify {:?}", tx.verify());

                Ok(TxOut::Generated(multi_sig_tx))
            }
            _ => Err(Error::<T>::InvalidTxOutType),
        }
    }

    pub fn sign<T: crate::Trait>(self, sk: Vec<u8>, author: AccountId) -> Result<Self, Error<T>> {
        match self {
            TxOut::Generated(mut multi_sig_tx) => {
                // if multi_sig_tx.multi_sig.has_signed(author.clone()) {
                //     return Err(Error::<T>::AlreadySignedByAuthor);
                // }

                let mut tx = Tx::read(&multi_sig_tx.raw_tx, &mut 0)
                    .map_err(|_| Error::<T>::IostChainError)?;
                // let sig: Signature = tx.sign(sk, chain_id.clone()).map_err(|_| Error::<T>::IostChainError)?;
                tx.sign(
                    "admin".to_string(),
                    iost_keys::algorithm::ED25519,
                    sk.as_slice(),
                );
                match tx.verify() {
                    Ok(_) => {
                        multi_sig_tx.raw_tx = tx
                            .to_serialize_data()
                            .map_err(|_| Error::<T>::IostChainError)?;
                        Ok(TxOut::Signed(multi_sig_tx))
                    }
                    _ => Err(Error::<T>::IostChainError),
                }
            }
            _ => Err(Error::<T>::InvalidTxOutType),
        }
    }

    pub fn send<T: crate::Trait>(self, iost_node_url: &str) -> Result<Self, Error<T>> {
        match self {
            TxOut::Signed(multi_sig_tx) => {
                let signed_trx = iost_rpc::serialize_push_transaction_params(&multi_sig_tx)?;

                let tx_id = iost_rpc::push_transaction(iost_node_url, signed_trx)?;

                // let transaction_id = core::str::from_utf8(transaction_vec.as_slice()).map_err(|_| Error::<T>::ParseUtf8Error)?;
                // let tx_id = Checksum256::from_str(&transaction_id).map_err(|_| Error::<T>::InvalidChecksum256)?;

                Ok(TxOut::Processing {
                    tx_id,
                    multi_sig_tx,
                })
            }
            _ => Err(Error::<T>::InvalidTxOutType),
        }
    }
}

pub(crate) mod iost_rpc {
    use super::*;
    use crate::Error;
    use lite_json::{parse_json, JsonValue, Serialize};
    use sp_runtime::offchain::http;

    const HASH: [char; 4] = ['h', 'a', 's', 'h']; // tx hash
    const CHAIN_ID: [char; 8] = ['c', 'h', 'a', 'i', 'n', '_', 'i', 'd']; // key chain_id
    const HEAD_BLOCK_HASH: [char; 15] = [
        'h', 'e', 'a', 'd', '_', 'b', 'l', 'o', 'c', 'k', '_', 'h', 'a', 's', 'h',
    ]; // key head_block_id
    const GET_INFO_API: &'static str = "/getChainInfo";
    const GET_BLOCK_API: &'static str = "/getBlockByHash";
    const PUSH_TRANSACTION_API: &'static str = "/v1/chain/push_transaction";

    type ChainId = i32;
    type HeadBlockHash = String;
    type BlockNum = u16;
    type RefBlockPrefix = u32;

    pub(crate) fn get_info<T: crate::Trait>(
        node_url: &str,
    ) -> Result<(ChainId, HeadBlockHash), Error<T>> {
        let req_api = format!("{}{}", node_url, GET_INFO_API);
        let pending = http::Request::post(&req_api, vec![b"{}"])
            // .add_header("Content-Type", "application/json")
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
                                // JsonValue::Number(numberValue) => numberValue.into() as i32,
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
        if chain_id == 0 || head_block_hash == String::default() {
            return Err(Error::<T>::IOSTRpcError);
        }

        Ok((chain_id, head_block_hash))
    }

    pub(crate) fn serialize_push_transaction_params<T: crate::Trait, AccountId>(
        multi_sig_tx: &MultiSigTx<AccountId>,
    ) -> Result<Vec<u8>, Error<T>> {
        let mut tx =
            Tx::read(&multi_sig_tx.raw_tx, &mut 0).map_err(|_| Error::<T>::IostChainError)?;
        Ok(tx.no_std_serialize())
    }

    pub(crate) fn push_transaction<T: crate::Trait>(
        node_url: &str,
        signed_trx: Vec<u8>,
    ) -> Result<Vec<u8>, Error<T>> {
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

        Ok(tx_id.into_bytes())
    }

    pub(crate) fn get_transaction_id<T: crate::Trait>(
        trx_response: &str,
    ) -> Result<String, Error<T>> {
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
