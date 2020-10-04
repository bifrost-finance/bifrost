// use alloc::string::{String, ToString};
// use core::{iter::FromIterator, str::FromStr};
//
// use crate::Error;
// use lite_json::{parse_json, JsonValue, Serialize};
// use sp_runtime::offchain::http;
// use sp_std::prelude::*;
//
// const CHAIN_ID: [char; 8] = ['c', 'h', 'a', 'i', 'n', '_', 'i', 'd'];
// // key chain_id
// const HEAD_BLOCK_HASH: [char; 15] = [
//     'h', 'e', 'a', 'd', '_', 'b', 'l', 'o', 'c', 'k', '_', 'h', 'a', 's', 'h',
// ];
// key head_block_id
// const GET_INFO_API: &'static str = "/getChainInfo";
// const GET_BLOCK_API: &'static str = "/getBlockByHash";
// const PUSH_TRANSACTION_API: &'static str = "/v1/chain/push_transaction";
//
// type ChainId = String;
// type HeadBlockHash = String;
// type BlockNum = u16;
// type RefBlockPrefix = u32;
//
// pub(crate) fn get_info<T: crate::Trait>(
//     node_url: &str,
// ) -> Result<(ChainId, HeadBlockHash), Error<T>> {
//     let req_api = format!("{}{}", node_url, GET_INFO_API);
//     let pending = http::Request::get(&req_api)
//         // .add_header("Content-Type", "application/json")
//         .send()
//         .map_err(|_| Error::<T>::OffchainHttpError)?;
//
//     let response = pending.wait().map_err(|_| Error::<T>::OffchainHttpError)?;
//     let body = response.body().collect::<Vec<u8>>();
//     let body_str =
//         core::str::from_utf8(body.as_slice()).map_err(|_| Error::<T>::ParseUtf8Error)?;
//     let node_info = parse_json(body_str).map_err(|_| Error::<T>::LiteJsonError)?;
//     let mut chain_id = Default::default();
//     let mut head_block_hash = Default::default();
//
//     match node_info {
//         JsonValue::Object(ref json) => {
//             for item in json.iter() {
//                 if item.0 == CHAIN_ID {
//                     chain_id = {
//                         match item.1 {
//                             JsonValue::String(ref chars) => String::from_iter(chars.iter()),
//                             _ => return Err(Error::<T>::IOSTRpcError),
//                         }
//                     };
//                 }
//                 if item.0 == HEAD_BLOCK_HASH {
//                     head_block_hash = {
//                         match item.1 {
//                             JsonValue::String(ref chars) => String::from_iter(chars.iter()),
//                             _ => return Err(Error::<T>::IOSTRpcError),
//                         }
//                     };
//                 }
//             }
//         }
//         _ => return Err(Error::<T>::IOSTRpcError),
//     }
//     if chain_id == String::default() || head_block_hash == String::default() {
//         return Err(Error::<T>::IOSTRpcError);
//     }
//
//     Ok((chain_id, head_block_hash))
// }
//
// // #[cfg(test)]
// // mod test {
// //     use super::*;
// //
// //     #[test]
// //     fn get_chain_info_should_be_ok() {
// //         println!("{:#?}", iost_rpc::get_info::<crate::Trait>("https://api.iost.io").unwrap());
// //     }
// // }