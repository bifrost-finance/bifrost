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
use iost_chain::Action;
// use eos_chain::{Asset, Checksum256, Read, SerializeData, Signature, Transaction};
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
    chain_id: Vec<u8>,
    /// Transaction raw data for signing
    raw_tx: Vec<u8>,
    /// Signatures of transaction
    multi_sig: MultiSig<AccountId>,
    /// EOS transaction action
    // action: Action,
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
        // tx_id: Checksum256,
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
        // amount: Asset,
        threshold: u8,
        memo: &str,
        from: AccountId,
        token_type: node_primitives::TokenSymbol,
    ) -> Result<Self, Error<T>> {
        let eos_from = core::str::from_utf8(&raw_from).map_err(|_| Error::<T>::ParseUtf8Error)?;
        let eos_to = core::str::from_utf8(&raw_to).map_err(|_| Error::<T>::ParseUtf8Error)?;

        // Construct action
        // let action = Action::transfer(eos_from, eos_to, amount.to_string().as_ref(), memo).map_err(|_| Error::<T>::IostChainError)?;

        // Construct transaction
        let multi_sig_tx = MultiSigTx {
            chain_id: Default::default(),
            raw_tx: Default::default(),
            multi_sig: MultiSig::new(threshold),
            // action,
            from,
            token_type,
        };

        Ok(TxOut::Initial(multi_sig_tx))
    }
}
pub(crate) mod iost_rpc {
    use super::*;
    use crate::Error;
    use lite_json::{parse_json, JsonValue, Serialize};
    use sp_runtime::offchain::http;

    const CHAIN_ID: [char; 8] = ['c', 'h', 'a', 'i', 'n', '_', 'i', 'd']; // key chain_id
    const HEAD_BLOCK_HASH: [char; 15] = [
        'h', 'e', 'a', 'd', '_', 'b', 'l', 'o', 'c', 'k', '_', 'h', 'a', 's', 'h',
    ]; // key head_block_id
    const GET_INFO_API: &'static str = "/getChainInfo";
    const GET_BLOCK_API: &'static str = "/getBlockByHash";
    const PUSH_TRANSACTION_API: &'static str = "/v1/chain/push_transaction";

    type ChainId = String;
    type HeadBlockHash = String;
    type BlockNum = u16;
    type RefBlockPrefix = u32;

    pub(crate) fn get_info<T: crate::Trait>(
        node_url: &str,
    ) -> Result<(ChainId, HeadBlockHash), Error<T>> {
        let req_api = format!("{}{}", node_url, GET_INFO_API);
        let pending = http::Request::post(&req_api, vec![b"{}"])
            .add_header("Content-Type", "application/json")
            .send()
            .map_err(|_| Error::<T>::OffchainHttpError)?;

        let response = pending.wait().map_err(|_| Error::<T>::OffchainHttpError)?;
        let body = response.body().collect::<Vec<u8>>();
        let body_str =
            core::str::from_utf8(body.as_slice()).map_err(|_| Error::<T>::ParseUtf8Error)?;
        let node_info = parse_json(body_str).map_err(|_| Error::<T>::LiteJsonError)?;
        let mut chain_id = Default::default();
        let mut head_block_hash = Default::default();

        match node_info {
            JsonValue::Object(ref json) => {
                for item in json.iter() {
                    if item.0 == CHAIN_ID {
                        chain_id = {
                            match item.1 {
                                JsonValue::String(ref chars) => String::from_iter(chars.iter()),
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
        if chain_id == String::default() || head_block_hash == String::default() {
            return Err(Error::<T>::IOSTRpcError);
        }

        Ok((chain_id, head_block_hash))
    }

    // pub(crate) fn get_block<T: crate::Trait>(node_url: &str, head_block_hash: String) -> Result<(BlockNum, RefBlockPrefix), Error<T>> {
    //
    // }
}
