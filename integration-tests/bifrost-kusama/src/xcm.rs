use crate::{kusama_integration_tests::*, kusama_test_net::*};
use bifrost_kusama_runtime::Runtime;
use codec::Encode;
use frame_support::{assert_ok, dispatch::RawOrigin, traits::Currency};
use xcm::latest::prelude::*;
use xcm_emulator::{Limited, TestExt};

#[test]
fn transact_transfer_call_from_relaychain_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			pallet_balances::Pallet::<Runtime>::make_free_balance_be(
				&AccountId::from(ALICE),
				1000 * 1_000_000_000_000,
			);
		});

		let alice = Junctions::X1(Junction::AccountId32 { network: NetworkId::Kusama, id: ALICE });
		let call = Call::Balances(pallet_balances::Call::<Runtime>::transfer {
			dest: MultiAddress::Id(AccountId::from(BOB)),
			value: 500 * 1_000_000_000_000,
		});
		let assets: MultiAsset = (Parent, 1_000_000_000_000).into();

		KusamaNet::execute_with(|| {
			let xcm = vec![
				WithdrawAsset(assets.clone().into()),
				BuyExecution { fees: assets, weight_limit: Limited(1_000_000_000_000) },
				Transact {
					origin_type: OriginKind::SovereignAccount,
					require_weight_at_most: (1_000_000_000_000) as u64 / 10 as u64,
					call: call.encode().into(),
				},
				DepositAsset {
					assets: All.into(),
					max_assets: 1,
					beneficiary: { (1, alice.clone()).into() },
				},
			];
			assert_ok!(pallet_xcm::Pallet::<kusama_runtime::Runtime>::send_xcm(
				alice,
				Parachain(2001).into(),
				Xcm(xcm)
			));
		});

		Bifrost::execute_with(|| {
			use Event;
			use System;
			assert_eq!(
				9988297130000,
				orml_tokens::Pallet::<Runtime>::free_balance(
					CurrencyId::Token(TokenSymbol::KSM),
					&AccountId::from(ALICE)
				)
			);
			assert_eq!(
				500 * 1_000_000_000_000,
				pallet_balances::Pallet::<Runtime>::free_balance(&AccountId::from(ALICE))
			);
			assert_eq!(
				500 * 1_000_000_000_000,
				pallet_balances::Pallet::<Runtime>::free_balance(&AccountId::from(BOB))
			);
			System::assert_has_event(Event::Balances(pallet_balances::Event::Transfer {
				from: AccountId::from(ALICE),
				to: AccountId::from(BOB),
				amount: 500 * 1_000_000_000_000,
			}));
		});
	})
}

#[test]
fn transact_salp_contribute_call_from_relaychain_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			assert_ok!(Salp::create(
				RawOrigin::Root.into(),
				3_000,
				100 * 1_000_000_000_000,
				1,
				SlotLength::get()
			));
			assert_ok!(Salp::funds(3_000).ok_or(()));
		});

		let alice = Junctions::X1(Junction::AccountId32 { network: NetworkId::Kusama, id: ALICE });
		let call = Call::Salp(bifrost_salp::Call::<Runtime>::contribute {
			index: 3000,
			value: 1 * 1_000_000_000_000,
		});
		let assets: MultiAsset = (Parent, 1_000_000_000_000).into();

		KusamaNet::execute_with(|| {
			let xcm = vec![
				WithdrawAsset(assets.clone().into()),
				BuyExecution { fees: assets, weight_limit: Limited(1_000_000_000_000) },
				Transact {
					origin_type: OriginKind::SovereignAccount,
					require_weight_at_most: (1_000_000_000_000) / 10 as u64,
					call: call.encode().into(),
				},
				DepositAsset {
					assets: All.into(),
					max_assets: 1,
					beneficiary: { (1, alice.clone()).into() },
				},
			];
			assert_ok!(pallet_xcm::Pallet::<kusama_runtime::Runtime>::send_xcm(
				alice,
				Parachain(2001).into(),
				Xcm(xcm)
			));
		});
	})
}
