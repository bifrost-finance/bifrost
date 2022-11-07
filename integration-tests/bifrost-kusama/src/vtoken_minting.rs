use crate::{kusama_integration_tests::*, kusama_test_net::*};
use bifrost_asset_registry::AssetIdMaps;
use bifrost_kusama_runtime::{Runtime, VtokenMinting};
use frame_support::{assert_ok, dispatch::RawOrigin};
use sp_runtime::{traits::AccountIdConversion, Permill};
use xcm_emulator::TestExt;

/*
set_unlock_duration unlock_duration TimeUnit::Era(28)
set_minimum_mint minimum_mint 0.5 KSM
set_minimum_redeem minimum_redeem 0 KSM
add_support_rebond_token token_to_rebond Some(0)
remove_support_rebond_token token_to_rebond
set_fees fees (0%,0.1%)
set_hook_iteration_limit hook_iteration_limit 10
set_unlocking_total unlocking_total
set_min_time_unit min_time_unit TimeUnit::Era(4362)

mint
redeem
 */

#[test]
fn set_unlock_duration_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			assert_ok!(VtokenMinting::set_unlock_duration(
				RawOrigin::Root.into(),
				CurrencyId::Token(TokenSymbol::KSM),
				TimeUnit::Era(28),
			));
			assert_eq!(
				VtokenMinting::unlock_duration(CurrencyId::Token(TokenSymbol::KSM)),
				Some(TimeUnit::Era(28))
			);
		});
	})
}

#[test]
fn set_minimum_mint_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			assert_ok!(VtokenMinting::set_minimum_mint(
				RawOrigin::Root.into(),
				CurrencyId::Token(TokenSymbol::KSM),
				50_000_000_000,
			));
			assert_eq!(
				VtokenMinting::minimum_mint(CurrencyId::Token(TokenSymbol::KSM)),
				50_000_000_000
			);
			assert_eq!(AssetIdMaps::<Runtime>::check_vtoken_registered(TokenSymbol::KSM), true);
			assert_eq!(AssetIdMaps::<Runtime>::check_token_registered(TokenSymbol::KSM), true);
		});
	})
}

#[test]
fn set_minimum_redeem_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			assert_ok!(VtokenMinting::set_minimum_redeem(
				RawOrigin::Root.into(),
				CurrencyId::Token(TokenSymbol::KSM),
				10_000,
			));
			assert_eq!(VtokenMinting::minimum_redeem(CurrencyId::Token(TokenSymbol::KSM)), 10_000);
		});
	})
}

#[test]
fn add_support_rebond_token_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			assert_eq!(VtokenMinting::token_to_rebond(CurrencyId::Token(TokenSymbol::KSM)), None);
			assert_ok!(VtokenMinting::add_support_rebond_token(
				RawOrigin::Root.into(),
				CurrencyId::Token(TokenSymbol::KSM),
			));
			assert_eq!(
				VtokenMinting::token_to_rebond(CurrencyId::Token(TokenSymbol::KSM)),
				Some(0)
			);
		});
	})
}

#[test]
fn remove_support_rebond_token_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			assert_eq!(VtokenMinting::token_to_rebond(CurrencyId::Token(TokenSymbol::KSM)), None);
			assert_ok!(VtokenMinting::add_support_rebond_token(
				RawOrigin::Root.into(),
				CurrencyId::Token(TokenSymbol::KSM),
			));
			assert_eq!(
				VtokenMinting::token_to_rebond(CurrencyId::Token(TokenSymbol::KSM)),
				Some(0)
			);
			assert_ok!(VtokenMinting::remove_support_rebond_token(
				RawOrigin::Root.into(),
				CurrencyId::Token(TokenSymbol::KSM),
			));
			assert_eq!(VtokenMinting::token_to_rebond(CurrencyId::Token(TokenSymbol::KSM)), None);
		});
	})
}

#[test]
fn set_fees_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			assert_ok!(VtokenMinting::set_fees(
				RawOrigin::Root.into(),
				Permill::from_perthousand(0),
				Permill::from_perthousand(1),
			));
			assert_eq!(
				VtokenMinting::fees(),
				(Permill::from_perthousand(0), Permill::from_perthousand(1))
			);
			println!("{:#?}", Permill::from_perthousand(1));
		});
	})
}

#[test]
fn set_hook_iteration_limit_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			assert_ok!(VtokenMinting::set_hook_iteration_limit(RawOrigin::Root.into(), 10));
			assert_eq!(VtokenMinting::hook_iteration_limit(), 10);
		});
	})
}

#[test]
fn set_unlocking_total_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			assert_ok!(VtokenMinting::set_unlocking_total(
				RawOrigin::Root.into(),
				CurrencyId::Token(TokenSymbol::KSM),
				10_000_000_000,
			));
			assert_eq!(
				VtokenMinting::unlocking_total(CurrencyId::Token(TokenSymbol::KSM)),
				10_000_000_000
			);
		});
	})
}

#[test]
fn set_min_time_unit_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			assert_ok!(VtokenMinting::set_min_time_unit(
				RawOrigin::Root.into(),
				CurrencyId::Token(TokenSymbol::KSM),
				TimeUnit::Era(4362),
			));
			assert_eq!(
				VtokenMinting::min_time_unit(CurrencyId::Token(TokenSymbol::KSM)),
				TimeUnit::Era(4362)
			);
		});
	})
}

#[test]
fn mint_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			assert_eq!(VtokenMinting::token_pool(CurrencyId::Token(TokenSymbol::KSM)), 0);
			assert_eq!(Currencies::total_issuance(CurrencyId::VToken(TokenSymbol::KSM)), 0);
			assert_eq!(
				VtokenMinting::fees(),
				(Permill::from_perthousand(0), Permill::from_perthousand(0))
			);

			assert_ok!(VtokenMinting::mint(
				RawOrigin::Signed(AccountId::new(ALICE)).into(),
				CurrencyId::Token(TokenSymbol::KSM),
				5_000_000_000_000,
			));

			//check balance
			let entrance_account: AccountId = SlpEntrancePalletId::get().into_account_truncating();
			assert_eq!(
				Currencies::free_balance(
					CurrencyId::VToken(TokenSymbol::KSM),
					&AccountId::from(ALICE)
				),
				5_000_000_000_000
			);
			assert_eq!(
				Currencies::free_balance(CurrencyId::Token(TokenSymbol::KSM), &entrance_account),
				5_000_000_000_000
			);
			assert_eq!(
				VtokenMinting::token_pool(CurrencyId::Token(TokenSymbol::KSM)),
				5_000_000_000_000
			);
		});
	})
}

#[test]
fn redeem_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		Bifrost::execute_with(|| {
			pub const FEE: Permill = Permill::from_percent(2);
			assert_ok!(VtokenMinting::set_fees(RawOrigin::Root.into(), FEE, FEE));
			assert_ok!(VtokenMinting::set_unlock_duration(
				RawOrigin::Root.into(),
				CurrencyId::Token(TokenSymbol::KSM),
				TimeUnit::Era(1)
			));
			assert_ok!(VtokenMinting::update_ongoing_time_unit(
				CurrencyId::Token(TokenSymbol::KSM),
				TimeUnit::Era(1)
			));
			assert_ok!(VtokenMinting::set_minimum_redeem(
				RawOrigin::Root.into(),
				CurrencyId::Token(TokenSymbol::KSM),
				2 * 1_000_000_000_000
			));
			assert_ok!(VtokenMinting::mint(
				RawOrigin::Signed(AccountId::new(ALICE)).into(),
				CurrencyId::Token(TokenSymbol::KSM),
				5 * 1_000_000_000_000
			));
			assert_eq!(
				VtokenMinting::token_pool(CurrencyId::Token(TokenSymbol::KSM)),
				5 * 1_000_000_000_000 - 5 * 20_000_000_000
			); // 1000 + 980 - 98 - 196

			assert_ok!(VtokenMinting::redeem(
				RawOrigin::Signed(AccountId::new(ALICE)).into(),
				CurrencyId::VToken(TokenSymbol::KSM),
				1 * 1_000_000_000_000
			));
			assert_eq!(
				VtokenMinting::token_pool(CurrencyId::Token(TokenSymbol::KSM)),
				5 * 1_000_000_000_000 - 5 * 20_000_000_000 - 1_000_000_000_000 + 20_000_000_000
			);
		});
	})
}
