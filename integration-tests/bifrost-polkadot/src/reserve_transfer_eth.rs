use bifrost_asset_registry::AssetMetadata;
use bifrost_polkadot_runtime::{AssetRegistry, RuntimeOrigin};
use bifrost_primitives::CurrencyId;
use frame_support::{assert_ok, traits::Currency};
use hex_literal::hex;
use integration_tests_common::{
	AssetHubPolkadot, AssetHubPolkadotAlice, BifrostPolkadot, BifrostPolkadotAlice,
};
use polkadot_parachain_primitives::primitives::Sibling;
use sp_runtime::{traits::AccountIdConversion, MultiAddress};
use xcm::{
	latest::{MultiLocation, Outcome::Complete},
	prelude::*,
	VersionedMultiLocation,
};
use xcm_emulator::{bx, Parachain, TestExt};

const ETH_ADDRESS: [u8; 20] = hex!["c9f05326311bc2a55426761bec20057685fb80f7"];

#[test]
fn cross_eth() {
	BifrostPolkadot::execute_with(|| {
		let metadata = AssetMetadata {
			name: b"ETH".to_vec(),
			symbol: b"ETH".to_vec(),
			decimals: 18,
			minimal_balance: 10,
		};

		assert_ok!(AssetRegistry::register_token_metadata(
			RuntimeOrigin::root(),
			bx!(metadata.clone())
		));

		let location = VersionedMultiLocation::V3(MultiLocation::new(
			2,
			X2(
				GlobalConsensus(Ethereum { chain_id: 11155111 }),
				AccountKey20 { network: None, key: ETH_ADDRESS },
			),
		));

		assert_ok!(AssetRegistry::register_multilocation(
			RuntimeOrigin::root(),
			CurrencyId::Token2(0),
			bx!(location.clone()),
			Weight::zero()
		));
	});

	AssetHubPolkadot::execute_with(|| {
		use asset_hub_polkadot_runtime::{
			Balances, ForeignAssets, Runtime, RuntimeEvent, RuntimeOrigin, System,
		};

		let sibling_account = Sibling::from(2030).into_account_truncating();

		// need to have some KSM to be able to receive user assets
		Balances::make_free_balance_be(&sibling_account, 10 * 1_000_000_000_000u128);

		assert_ok!(ForeignAssets::force_create(
			RuntimeOrigin::root(),
			MultiLocation::new(
				2,
				X2(
					GlobalConsensus(Ethereum { chain_id: 11155111 }),
					AccountKey20 { network: None, key: ETH_ADDRESS }
				)
			),
			MultiAddress::Id(AssetHubPolkadotAlice::get()),
			true,
			1u128
		));

		assert_ok!(ForeignAssets::force_set_metadata(
			RuntimeOrigin::root(),
			MultiLocation::new(
				2,
				X2(
					GlobalConsensus(Ethereum { chain_id: 11155111 }),
					AccountKey20 { network: None, key: ETH_ADDRESS }
				)
			),
			b"ETH".to_vec(),
			b"ETH".to_vec(),
			18u8,
			false
		));

		assert_ok!(ForeignAssets::mint(
			RuntimeOrigin::signed(AssetHubPolkadotAlice::get()),
			MultiLocation::new(
				2,
				X2(
					GlobalConsensus(Ethereum { chain_id: 11155111 }),
					AccountKey20 { network: None, key: ETH_ADDRESS }
				)
			),
			MultiAddress::Id(AssetHubPolkadotAlice::get()),
			100 * 1_000_000_000_000_000_000u128
		));
		assert!(System::events().iter().any(|r| matches!(
			r.event,
			RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued {
				asset_id: _,
				owner: _,
				amount: 100_000_000_000_000_000_000u128
			})
		)));

		let assets = MultiAssets::from(vec![MultiAsset::from((
			Concrete(MultiLocation::new(
				2,
				X2(
					GlobalConsensus(Ethereum { chain_id: 11155111 }),
					AccountKey20 { network: None, key: ETH_ADDRESS },
				),
			)),
			Fungibility::from(10 * 1_000_000_000_000_000_000u128),
		))]);

		assert_ok!(pallet_xcm::Pallet::<Runtime>::limited_reserve_transfer_assets(
			RuntimeOrigin::signed(AssetHubPolkadotAlice::get()),
			bx!(AssetHubPolkadot::sibling_location_of(BifrostPolkadot::para_id()).into()),
			bx!(AccountId32 { network: None, id: BifrostPolkadotAlice::get().into() }.into()),
			Box::new(VersionedMultiAssets::V3(assets)),
			0,
			Unlimited,
		));
		assert!(System::events().iter().any(|r| matches!(
			r.event,
			RuntimeEvent::ForeignAssets(pallet_assets::Event::Transferred {
				asset_id: _,
				from: _,
				to: _,
				amount: 10_000_000_000_000_000_000u128
			})
		)));
		assert!(System::events().iter().any(|r| matches!(
			&r.event,
			RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Attempted { outcome: Complete(_) })
		)));
		System::reset_events();
	});

	BifrostPolkadot::execute_with(|| {
		use bifrost_polkadot_runtime::{RuntimeEvent, System};
		assert!(System::events().iter().any(|r| matches!(
			&r.event,
			RuntimeEvent::Tokens(orml_tokens::Event::Deposited {
				currency_id: CurrencyId::Token2(0),
				who: _,
				amount: 10_000_000_000_000_000_000u128
			})
		)));
		System::reset_events();
	});

	// TODO:
	// BifrostPolkadot::execute_with(|| {
	// 	use bifrost_polkadot_runtime::{System, RuntimeEvent, XTokens, Runtime};
	// 	let assets = MultiAssets::from(vec![MultiAsset::from((
	// 		Concrete(MultiLocation::new(2, X2(GlobalConsensus(Ethereum { chain_id: 11155111 }),
	// AccountKey20 { network: None, key: ETH_ADDRESS }))),
	// 		Fungibility::from(1_000_000_000_000_000_000u128),
	// 	))]);
	// 	assert_ok!(pallet_xcm::Pallet::<Runtime>::limited_teleport_assets(
	// 		RuntimeOrigin::signed(AssetHubPolkadotAlice::get()),
	// 		bx!(BifrostPolkadot::sibling_location_of(AssetHubPolkadot::para_id()).into()),
	// 		bx!(AccountId32 { network: None, id: AssetHubPolkadotAlice::get().into() }.into()),
	// 		Box::new(VersionedMultiAssets::V3(assets)),
	// 		0,
	// 		Unlimited,
	// 	));
	// })

	// AsserHubKusama::execute_with(|| {
	// 		use statemine_runtime::*;
	// 		println!("{:?}", System::events());
	//
	// 		// assert Bob has 5 statemine asset
	// 		assert_eq!(Assets::balance(1984, AccountId::from(BOB)), 5 * USDT);
	//
	// 		assert!(System::events().iter().any(|r| matches!(
	// 			r.event,
	// 			RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success {
	// 				message_hash: _,
	// 				weight: _
	// 			})
	// 		)));
	// 	});
}
