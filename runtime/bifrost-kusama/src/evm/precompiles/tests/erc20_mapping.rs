use crate::evm::precompiles::erc20_mapping::{Erc20Mapping, HydraErc20Mapping};
use hex_literal::hex;
use primitive_types::H160;

macro_rules! encode {
	($asset_id:expr) => {{
		HydraErc20Mapping::encode_evm_address($asset_id).unwrap()
	}};
}

macro_rules! decode {
	($evm_address:expr) => {{
		HydraErc20Mapping::decode_evm_address(H160::from($evm_address)).unwrap()
	}};
}

macro_rules! decode_optional {
	($evm_address:expr) => {{
		HydraErc20Mapping::decode_evm_address(H160::from($evm_address))
	}};
}

#[test]
fn decode_asset_id_from_evm_address_should_work() {
	assert_eq!(decode!(hex!("0000000000000000000000000000000100000000")), 0);
	assert_eq!(decode!(hex!("0000000000000000000000000000000100000001")), 1);
	assert_eq!(decode!(hex!("0000000000000000000000000000000100000002")), 2);
	assert_eq!(decode!(hex!("0000000000000000000000000000000100000003")), 3);
	assert_eq!(decode!(hex!("0000000000000000000000000000000100000009")), 9);
	assert_eq!(decode!(hex!("000000000000000000000000000000010000000a")), 10);
	assert_eq!(decode!(hex!("000000000000000000000000000000010000000b")), 11);
	assert_eq!(decode!(hex!("00000000000000000000000000000001000000ff")), 255);
	assert_eq!(decode!(hex!("00000000000000000000000000000001ffffffff")), 4294967295);
}

#[test]
fn decode_asset_id_from_evm_address_should_not_work_with_invalid_asset_addresses() {
	assert_eq!(decode_optional!(hex!("0000000000000000000000000000000200000000")), None);
	assert_eq!(decode_optional!(hex!("0000000000000000000000000000000000000001")), None);
	assert_eq!(decode_optional!(hex!("90000000000000000000000000000001ffffffff")), None);
	assert_eq!(decode_optional!(hex!("0000000000000000000000000000001100000003")), None);
	assert_eq!(decode_optional!(hex!("0000000000000000900000000000000100000003")), None);
	assert_eq!(decode_optional!(hex!("7777777777777777777777777777777777777777")), None);
}

#[test]
fn encode_asset_id_to_evm_address_should_work() {
	assert_eq!(encode!(0), H160::from(hex!("0000000000000000000000000000000100000000")));
	assert_eq!(encode!(1), H160::from(hex!("0000000000000000000000000000000100000001")));
	assert_eq!(encode!(2), H160::from(hex!("0000000000000000000000000000000100000002")));
	assert_eq!(encode!(3), H160::from(hex!("0000000000000000000000000000000100000003")));
	assert_eq!(encode!(9), H160::from(hex!("0000000000000000000000000000000100000009")));
	assert_eq!(
		encode!(10),
		H160::from(hex!("000000000000000000000000000000010000000a"))
	);
	assert_eq!(
		encode!(11),
		H160::from(hex!("000000000000000000000000000000010000000b"))
	);
	assert_eq!(
		encode!(255),
		H160::from(hex!("00000000000000000000000000000001000000ff"))
	);
	assert_eq!(
		encode!(4294967295),
		H160::from(hex!("00000000000000000000000000000001ffffffff"))
	);
}
