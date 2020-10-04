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
use crate::mock::*;
use crate::*;

use crate::mock::*;
use crate::*;
use core::{convert::From, str::FromStr};

use frame_support::{assert_ok, dispatch};
use node_primitives::{BlockchainType, BridgeAssetSymbol};
use sp_core::offchain::{
    testing::{TestOffchainExt, TestTransactionPoolExt},
    OffchainExt, TransactionPoolExt,
};
use sp_core::H256;
use sp_runtime::traits::Header as HeaderT;
use sp_runtime::{generic::DigestItem, testing::Header};
#[cfg(feature = "std")]
use std::{error::Error, fs::File, io::Read as StdRead, path::Path};

#[test]
#[ignore = "This is a simulated http server, no response actually."]
fn bridge_iost_offchain_should_work() {
    let mut ext = new_test_ext();
    let (offchain, _state) = TestOffchainExt::new();
    let (pool, pool_state) = TestTransactionPoolExt::new();
    ext.register_extension(OffchainExt::new(offchain));
    ext.register_extension(TransactionPoolExt::new(pool));

    ext.execute_with(|| {
        System::set_block_number(1);
        sp_io::offchain::local_storage_set(
            StorageKind::PERSISTENT,
            b"IOST_NODE_URL",
            b"http://127.0.0.1:30001/",
        );

        // EOS secret key of account testa
        sp_io::offchain::local_storage_set(
            StorageKind::PERSISTENT,
            b"IOST_SECRET_KEY",
            b"5JgbL2ZnoEAhTudReWH1RnMuQS6DBeLZt4ucV6t8aymVEuYg7sr",
        );

        let raw_to = b"alice".to_vec();
        let raw_symbol = b"IOST".to_vec();
        let asset_symbol = BridgeAssetSymbol::new(BlockchainType::IOST, raw_symbol, 4u32);
        let bridge_asset = BridgeAssetBalance {
            symbol: asset_symbol.clone(),
            amount: 1 * 10u64.pow(8),
            memo: vec![],
            from: 1,
            token_symbol: TokenSymbol::IOST,
        };
        assert_ok!(BridgeIost::bridge_asset_to(raw_to.clone(), bridge_asset));
        assert_ok!(BridgeIost::offchain(1));

        // EOS secret key of account testb
        sp_io::offchain::local_storage_set(
            StorageKind::PERSISTENT,
            b"IOST_SECRET_KEY",
            b"5J6vV6xbVV2UEwBYYDRQQ8yTDcSmHJw67XqRriF4EkEzWKUFNKj",
        );

        rotate_author(2);
        assert_ok!(BridgeEos::offchain(2));

        use codec::Decode;
        let transaction = pool_state.write().transactions.pop().unwrap();
        assert_eq!(pool_state.read().transactions.len(), 1);
        let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
        let tx_outs = match ex.call {
            crate::mock::Call::BridgeEos(crate::Call::bridge_tx_report(tx_outs)) => tx_outs,
            e => panic!("Unexpected call: {:?}", e),
        };

        assert_eq!(
            tx_outs
                .iter()
                .filter(|out| {
                    match out {
                        TxOut::Processing { .. } => true,
                        _ => false,
                    }
                })
                .count(),
            1
        );
    });
}
