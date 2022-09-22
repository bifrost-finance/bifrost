use crate::kusama_test_net::Bifrost;
use bifrost_asset_registry::AssetMetadata;
use bifrost_kusama_runtime::{AssetRegistry, Runtime};
use bifrost_runtime_common::{cent, dollar, micro, microcent, milli, millicent};
use frame_support::assert_ok;
use node_primitives::{CurrencyId, TokenInfo, TokenSymbol::*, TryConvertFrom};
use sp_runtime::AccountId32;
use xcm_emulator::TestExt;

const DECIMAL_18: u128 = 1_000_000_000_000_000_000;
const DECIMAL_12: u128 = 1_000_000_000_000;
const DECIMAL_10: u128 = 10_000_000_000;

const DOT_TOKEN_ID: u8 = 0;
const GLMR_TOKEN_ID: u8 = 1;
const DOT_MINIMAL_BALANCE: u128 = 1_000_000;
const GLMR_MINIMAL_BALANCE: u128 = 1_000_000_000_000;

#[test]
fn dollar_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(ASG)), DECIMAL_12);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(BNC)), DECIMAL_12);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(KUSD)), DECIMAL_12);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(DOT)), DECIMAL_10);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(KSM)), DECIMAL_12);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(ETH)), DECIMAL_18);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(KAR)), DECIMAL_12);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(ZLK)), DECIMAL_18);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(PHA)), DECIMAL_12);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(RMRK)), DECIMAL_10);
		assert_eq!(dollar::<Runtime>(CurrencyId::Token(MOVR)), DECIMAL_18);
	});
}

#[test]
fn milli_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		assert_eq!(milli::<Runtime>(CurrencyId::Token(ASG)), DECIMAL_12 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(BNC)), DECIMAL_12 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(KUSD)), DECIMAL_12 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(DOT)), DECIMAL_10 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(KSM)), DECIMAL_12 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(ETH)), DECIMAL_18 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(KAR)), DECIMAL_12 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(ZLK)), DECIMAL_18 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(PHA)), DECIMAL_12 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(RMRK)), DECIMAL_10 / 1000);
		assert_eq!(milli::<Runtime>(CurrencyId::Token(MOVR)), DECIMAL_18 / 1000);
	});
}

#[test]
fn micro_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		assert_eq!(micro::<Runtime>(CurrencyId::Token(ASG)), DECIMAL_12 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(BNC)), DECIMAL_12 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(KUSD)), DECIMAL_12 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(DOT)), DECIMAL_10 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(KSM)), DECIMAL_12 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(ETH)), DECIMAL_18 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(KAR)), DECIMAL_12 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(ZLK)), DECIMAL_18 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(PHA)), DECIMAL_12 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(RMRK)), DECIMAL_10 / 1_000_000);
		assert_eq!(micro::<Runtime>(CurrencyId::Token(MOVR)), DECIMAL_18 / 1_000_000);
	});
}

#[test]
fn cent_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		assert_eq!(cent::<Runtime>(CurrencyId::Token(ASG)), DECIMAL_12 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(BNC)), DECIMAL_12 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(KUSD)), DECIMAL_12 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(DOT)), DECIMAL_10 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(KSM)), DECIMAL_12 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(ETH)), DECIMAL_18 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(KAR)), DECIMAL_12 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(ZLK)), DECIMAL_18 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(PHA)), DECIMAL_12 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(RMRK)), DECIMAL_10 / 100);
		assert_eq!(cent::<Runtime>(CurrencyId::Token(MOVR)), DECIMAL_18 / 100);
	});
}

#[test]
fn millicent_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(ASG)), DECIMAL_12 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(BNC)), DECIMAL_12 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(KUSD)), DECIMAL_12 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(DOT)), DECIMAL_10 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(KSM)), DECIMAL_12 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(ETH)), DECIMAL_18 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(KAR)), DECIMAL_12 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(ZLK)), DECIMAL_18 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(PHA)), DECIMAL_12 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(RMRK)), DECIMAL_10 / 100_000);
		assert_eq!(millicent::<Runtime>(CurrencyId::Token(MOVR)), DECIMAL_18 / 100_000);
	});
}

#[test]
fn microcent_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(ASG)), DECIMAL_12 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(BNC)), DECIMAL_12 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(KUSD)), DECIMAL_12 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(DOT)), DECIMAL_10 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(KSM)), DECIMAL_12 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(ETH)), DECIMAL_18 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(KAR)), DECIMAL_12 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(ZLK)), DECIMAL_18 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(PHA)), DECIMAL_12 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(RMRK)), DECIMAL_10 / 100_000_000);
		assert_eq!(microcent::<Runtime>(CurrencyId::Token(MOVR)), DECIMAL_18 / 100_000_000);
	});
}
