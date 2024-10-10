// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
#![cfg(test)]

use crate::{
	mock::*,
	types::{EthereumXcmCall, EthereumXcmTransaction, EthereumXcmTransactionV2, MoonbeamCall},
	*,
};
use bifrost_primitives::{TokenSymbol, DOT, VDOT};
use ethereum::TransactionAction;
use frame_support::{assert_noop, assert_ok, dispatch::RawOrigin, traits::OnIdle};
use hex_literal::hex;
use sp_core::{bounded::BoundedVec, crypto::Ss58Codec, U256};
use tiny_keccak::Hasher;

const EVM_ADDR: [u8; 20] = hex!["573394b77fC17F91E9E67F147A9ECe24d67C5073"];
const ASTAR_SLPX_ADDR: [u8; 20] = hex!["c6bf0C5C78686f1D0E2E54b97D6de6e2cEFAe9fD"];
const MOONBEAM_SLPX_ADDR: [u8; 20] = hex!["F1d4797E51a4640a76769A50b57abE7479ADd3d8"];

#[test]
fn test_account_convert_work() {
	new_test_ext().execute_with(|| {
		let address = H160::from_slice(&EVM_ADDR);
		let account_id: AccountId = Slpx::h160_to_account_id(&address);
		assert_eq!(
			account_id,
			sp_runtime::AccountId32::new(hex!(
				"b1c2dde9e562a738e264a554e467b30e5cd58e95ab98459946fb8e518cfe71c2"
			))
		);
		let public_key: [u8; 32] = account_id.encode().try_into().unwrap();
		assert_eq!(
			public_key,
			hex!("b1c2dde9e562a738e264a554e467b30e5cd58e95ab98459946fb8e518cfe71c2")
		);
	});
}

#[test]
fn xcm_derivative_account() {
	new_test_ext().execute_with(|| {
		let address = H160::from_slice(&ASTAR_SLPX_ADDR);
		let derivative_account =
			Slpx::xcm_derivative_account(SupportChain::Astar, address).unwrap();
		assert_eq!(
			derivative_account,
			sp_runtime::AccountId32::from_ss58check(
				"g96o4GVpsAop1MJiArnmUYtXUjEisfkbfcpsuqmXrS28MEr"
			)
			.unwrap()
		);

		let address = H160::from_slice(&MOONBEAM_SLPX_ADDR);
		let derivative_account =
			Slpx::xcm_derivative_account(SupportChain::Moonbeam, address).unwrap();
		assert_eq!(
			derivative_account,
			sp_runtime::AccountId32::from_ss58check(
				"gWEvf2EDMzxR7JHyrEHXf3nqxKLGvHaFbk7HUkJnNPUxDts"
			)
			.unwrap()
		);
	});
}

#[test]
fn add_whitelist() {
	new_test_ext().execute_with(|| {
		let astar_slpx_addr = H160::from_slice(&ASTAR_SLPX_ADDR);
		let moonbeam_slpx_addr = H160::from_slice(&MOONBEAM_SLPX_ADDR);
		let astar_slpx_account_id = sp_runtime::AccountId32::from_ss58check(
			"g96o4GVpsAop1MJiArnmUYtXUjEisfkbfcpsuqmXrS28MEr",
		)
		.unwrap();
		let moonbeam_slpx_account_id = sp_runtime::AccountId32::from_ss58check(
			"gWEvf2EDMzxR7JHyrEHXf3nqxKLGvHaFbk7HUkJnNPUxDts",
		)
		.unwrap();
		assert_ok!(Slpx::add_whitelist(
			RuntimeOrigin::root(),
			SupportChain::Astar,
			astar_slpx_addr
		));
		assert_eq!(
			WhitelistAccountId::<Test>::get(SupportChain::Astar).to_vec(),
			vec![astar_slpx_account_id]
		);

		assert_ok!(Slpx::add_whitelist(
			RuntimeOrigin::root(),
			SupportChain::Moonbeam,
			moonbeam_slpx_addr
		));
		assert_eq!(
			WhitelistAccountId::<Test>::get(SupportChain::Moonbeam).to_vec(),
			vec![moonbeam_slpx_account_id]
		);
	});
}

#[test]
fn add_whitelist_account_id_already_in_whitelist() {
	new_test_ext().execute_with(|| {
		let astar_slpx_addr = H160::from_slice(&ASTAR_SLPX_ADDR);
		let astar_slpx_account_id = sp_runtime::AccountId32::from_ss58check(
			"g96o4GVpsAop1MJiArnmUYtXUjEisfkbfcpsuqmXrS28MEr",
		)
		.unwrap();
		assert_ok!(Slpx::add_whitelist(
			RuntimeOrigin::root(),
			SupportChain::Astar,
			astar_slpx_addr
		));
		assert_eq!(
			WhitelistAccountId::<Test>::get(SupportChain::Astar).to_vec(),
			vec![astar_slpx_account_id]
		);

		assert_noop!(
			Slpx::add_whitelist(RuntimeOrigin::root(), SupportChain::Astar, astar_slpx_addr),
			Error::<Test>::AccountAlreadyExists
		);
	});
}

#[test]
fn remove_whitelist() {
	new_test_ext().execute_with(|| {
		let astar_slpx_addr = H160::from_slice(&ASTAR_SLPX_ADDR);
		let astar_slpx_account_id = sp_runtime::AccountId32::from_ss58check(
			"g96o4GVpsAop1MJiArnmUYtXUjEisfkbfcpsuqmXrS28MEr",
		)
		.unwrap();
		assert_ok!(Slpx::add_whitelist(
			RuntimeOrigin::root(),
			SupportChain::Astar,
			astar_slpx_addr
		));
		assert_eq!(
			WhitelistAccountId::<Test>::get(SupportChain::Astar).to_vec(),
			vec![astar_slpx_account_id]
		);

		assert_ok!(Slpx::remove_whitelist(
			RuntimeOrigin::root(),
			SupportChain::Astar,
			astar_slpx_addr
		));
		assert_eq!(WhitelistAccountId::<Test>::get(SupportChain::Astar).to_vec(), vec![]);
	});
}

#[test]
fn remove_whitelist_account_id_not_in_whitelist() {
	new_test_ext().execute_with(|| {
		let astar_slpx_addr = H160::from_slice(&ASTAR_SLPX_ADDR);
		assert_noop!(
			Slpx::remove_whitelist(RuntimeOrigin::root(), SupportChain::Astar, astar_slpx_addr),
			Error::<Test>::AccountNotFound
		);
	});
}

#[test]
fn test_execution_fee_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Currencies::deposit(CurrencyId::Token2(0), &ALICE, 50 * 1_000_000_000));

		assert_ok!(Slpx::set_execution_fee(
			RuntimeOrigin::root(),
			CurrencyId::Token2(0),
			10 * 1_000_000_000
		));
		assert_eq!(ExecutionFee::<Test>::get(CurrencyId::Token2(0)), Some(10 * 1_000_000_000));

		let balance_exclude_fee =
			Slpx::charge_execution_fee(CurrencyId::Token2(0), 50 * 1_000_000_000, &ALICE).unwrap();
		assert_eq!(balance_exclude_fee, 40 * 1_000_000_000);

		assert_ok!(Slpx::set_transfer_to_fee(
			RuntimeOrigin::root(),
			SupportChain::Moonbeam,
			10 * 1_000_000_000
		));
		assert_eq!(TransferToFee::<Test>::get(SupportChain::Moonbeam), Some(10 * 1_000_000_000));
	});
}

#[test]
fn test_get_default_fee() {
	new_test_ext().execute_with(|| {
		assert_eq!(Slpx::get_default_fee(BNC), 10_000_000_000u128);
		assert_eq!(Slpx::get_default_fee(CurrencyId::Token(TokenSymbol::KSM)), 10_000_000_000u128);
		assert_eq!(
			Slpx::get_default_fee(CurrencyId::Token(TokenSymbol::MOVR)),
			10_000_000_000_000_000u128
		);
		assert_eq!(Slpx::get_default_fee(CurrencyId::VToken(TokenSymbol::KSM)), 10_000_000_000u128);
		assert_eq!(
			Slpx::get_default_fee(CurrencyId::VToken(TokenSymbol::MOVR)),
			10_000_000_000_000_000u128
		);
	});
}

#[test]
fn test_ed() {
	new_test_ext().execute_with(|| {
		assert_ok!(Currencies::deposit(
			CurrencyId::Native(TokenSymbol::BNC),
			&ALICE,
			50 * 1_000_000_000
		));
		assert_ok!(Currencies::deposit(
			CurrencyId::Token(TokenSymbol::KSM),
			&ALICE,
			50 * 1_000_000_000
		));

		assert_eq!(
			Currencies::free_balance(CurrencyId::Native(TokenSymbol::BNC), &ALICE),
			50 * 1_000_000_000
		);
		assert_eq!(
			Currencies::free_balance(CurrencyId::Token(TokenSymbol::KSM), &ALICE),
			50 * 1_000_000_000
		);

		assert_ok!(Currencies::transfer(
			RawOrigin::Signed(ALICE).into(),
			BOB,
			CurrencyId::Native(TokenSymbol::BNC),
			50 * 1_000_000_000
		));
		assert_ok!(Currencies::transfer(
			RawOrigin::Signed(ALICE).into(),
			BOB,
			CurrencyId::Token(TokenSymbol::KSM),
			50 * 1_000_000_000
		));
	});
}

#[test]
fn test_selector() {
	let mut selector = [0; 4];
	let mut sha3 = tiny_keccak::Keccak::v256();
	sha3.update(b"setTokenAmount(bytes2,uint256,uint256)");
	sha3.finalize(&mut selector);

	assert_eq!([154, 65, 185, 36], selector);
	println!("{:?}", selector);
	println!("{:?}", hex::encode(selector));
	assert_eq!("9a41b924", hex::encode(selector));
}

#[test]
fn test_ethereum_call() {
	new_test_ext().execute_with(|| {
		// b"setTokenAmount(bytes2,uint256,bytes2,uint256)"
		assert_eq!("9a41b9240001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007b00000000000000000000000000000000000000000000000000000000000001c8", hex::encode(Slpx::encode_ethereum_call(BNC, 123u128, 456u128)));

		println!("{:?}", hex::encode(Slpx::encode_ethereum_call(BNC, 123u128, 456u128)));
		let addr: [u8; 20] = hex!["ae0daa9bfc50f03ce23d30c796709a58470b5f42"];
		let r = EthereumXcmTransaction::V2(EthereumXcmTransactionV2 {
			gas_limit: U256::from(720000),
			action: TransactionAction::Call(H160::from(addr)),
			value: U256::zero(),
			input: Slpx::encode_ethereum_call(BNC, 123u128, 456u128).try_into().unwrap(),
			access_list: None,
		});
		let call = MoonbeamCall::EthereumXcm(EthereumXcmCall::Transact(r));
		println!("{}", hex::encode(call.encode()));
		assert_eq!("6d000180fc0a000000000000000000000000000000000000000000000000000000000000ae0daa9bfc50f03ce23d30c796709a58470b5f42000000000000000000000000000000000000000000000000000000000000000091019a41b9240001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007b00000000000000000000000000000000000000000000000000000000000001c800", hex::encode(Slpx::encode_transact_call(H160::from(addr), BNC, 123u128, 456u128).unwrap()));
	})
}

#[test]
fn test_set_currency_ethereum_call_switch() {
	new_test_ext().execute_with(|| {
		assert_ok!(Slpx::support_xcm_oracle(RuntimeOrigin::root(), BNC, true));
		assert_eq!(CurrencyIdList::<Test>::get().to_vec(), vec![BNC]);

		assert_ok!(Slpx::support_xcm_oracle(RuntimeOrigin::root(), KSM, true));
		assert_eq!(CurrencyIdList::<Test>::get().to_vec(), vec![BNC, KSM]);

		assert_ok!(Slpx::support_xcm_oracle(RuntimeOrigin::root(), BNC, false));
		assert_eq!(CurrencyIdList::<Test>::get().to_vec(), vec![KSM]);
	})
}

#[test]
fn test_set_ethereum_call_configration() {
	new_test_ext().execute_with(|| {
		assert_ok!(Slpx::set_xcm_oracle_configuration(
			RuntimeOrigin::root(),
			1_000_000_000_000_000_000u128,
			Weight::default(),
			5u32.into(),
			H160::from(hex!["ae0daa9bfc50f03ce23d30c796709a58470b5f42"])
		));

		assert_eq!(
			XcmEthereumCallConfiguration::<Test>::get().unwrap(),
			EthereumCallConfiguration {
				xcm_fee: 1_000_000_000_000_000_000u128,
				xcm_weight: Weight::default(),
				period: 5u32.into(),
				last_block: 0u32.into(),
				contract: H160::from(hex!["ae0daa9bfc50f03ce23d30c796709a58470b5f42"]),
			}
		);

		assert_ok!(Slpx::set_xcm_oracle_configuration(
			RuntimeOrigin::root(),
			1u128,
			Weight::default(),
			10u32.into(),
			H160::from(hex!["ae0daa9bfc50f03ce23d30c796709a58470b5f42"])
		));

		assert_eq!(
			XcmEthereumCallConfiguration::<Test>::get().unwrap(),
			EthereumCallConfiguration {
				xcm_fee: 1u128,
				xcm_weight: Weight::default(),
				period: 10u32.into(),
				last_block: 0u32.into(),
				contract: H160::from(hex!["ae0daa9bfc50f03ce23d30c796709a58470b5f42"]),
			}
		);
	})
}

#[test]
fn test_set_currency_to_support_xcm_fee() {
	new_test_ext().execute_with(|| {
		assert_ok!(Slpx::set_currency_support_xcm_fee(RuntimeOrigin::root(), BNC, true));
		assert_eq!(SupportXcmFeeList::<Test>::get().to_vec(), vec![BNC]);

		assert_ok!(Slpx::set_currency_support_xcm_fee(RuntimeOrigin::root(), KSM, true));
		assert_eq!(SupportXcmFeeList::<Test>::get().to_vec(), vec![BNC, KSM]);

		assert_ok!(Slpx::set_currency_support_xcm_fee(RuntimeOrigin::root(), BNC, false));
		assert_eq!(SupportXcmFeeList::<Test>::get().to_vec(), vec![KSM]);
	})
}

#[test]
fn test_add_order() {
	new_test_ext().execute_with(|| {
		WhitelistAccountId::<Test>::insert(
			SupportChain::Astar,
			BoundedVec::try_from(vec![ALICE]).unwrap(),
		);

		let source_chain_caller = H160::default();
		assert_ok!(Slpx::mint(
			RuntimeOrigin::signed(ALICE),
			source_chain_caller,
			DOT,
			TargetChain::Astar(source_chain_caller),
			BoundedVec::default()
		));
		assert_eq!(OrderQueue::<Test>::get().len(), 1usize);
		assert_ok!(Slpx::redeem(
			RuntimeOrigin::signed(ALICE),
			source_chain_caller,
			VDOT,
			TargetChain::Astar(source_chain_caller)
		));
		assert_eq!(OrderQueue::<Test>::get().len(), 2usize);
		assert_ok!(Slpx::force_add_order(
			RuntimeOrigin::root(),
			OrderCaller::Evm(source_chain_caller),
			ALICE,
			VDOT,
			TargetChain::Astar(source_chain_caller),
			BoundedVec::default(),
			0
		));
		assert_eq!(OrderQueue::<Test>::get().len(), 3usize);

		println!("{:?}", OrderQueue::<Test>::get());
	})
}

#[test]
fn test_mint_with_channel_id() {
	new_test_ext().execute_with(|| {
		WhitelistAccountId::<Test>::insert(
			SupportChain::Astar,
			BoundedVec::try_from(vec![ALICE]).unwrap(),
		);

		let source_chain_caller = H160::default();
		assert_ok!(Slpx::mint_with_channel_id(
			RuntimeOrigin::signed(ALICE),
			source_chain_caller,
			DOT,
			TargetChain::Astar(source_chain_caller),
			BoundedVec::default(),
			0u32
		));
		assert_eq!(OrderQueue::<Test>::get().len(), 1usize);
		assert_ok!(Slpx::redeem(
			RuntimeOrigin::signed(ALICE),
			source_chain_caller,
			VDOT,
			TargetChain::Astar(source_chain_caller)
		));
		assert_eq!(OrderQueue::<Test>::get().len(), 2usize);
	})
}

#[test]
fn test_hook() {
	new_test_ext().execute_with(|| {
		WhitelistAccountId::<Test>::insert(
			SupportChain::Astar,
			BoundedVec::try_from(vec![ALICE]).unwrap(),
		);
		let source_chain_caller = H160::default();
		assert_ok!(Slpx::mint(
			RuntimeOrigin::signed(ALICE),
			source_chain_caller,
			DOT,
			TargetChain::Astar(source_chain_caller),
			BoundedVec::default()
		));
		assert_eq!(OrderQueue::<Test>::get().len(), 1usize);
		<frame_system::Pallet<Test>>::set_block_number(2u32.into());

		assert_ok!(Tokens::set_balance(
			RuntimeOrigin::root(),
			OrderQueue::<Test>::get()[0].derivative_account.clone(),
			DOT,
			10_000_000_000_000_000_000,
			0
		));

		let current_block = <frame_system::Pallet<Test>>::block_number();
		Slpx::on_idle(current_block, Weight::default());
		assert_eq!(OrderQueue::<Test>::get().len(), 0usize);

		println!("{}", Currencies::free_balance(VDOT, &BOB));
	})
}
