#![cfg_attr(not(feature = "std"), no_std)]

use bifrost_primitives::{Balance, CurrencyId, PriceDetail};
use num_bigint::{BigUint, ToBigUint};

pub mod evm;
pub mod lend_market;

pub use lend_market::*;

pub trait EmergencyCallFilter<Call> {
	fn contains(call: &Call) -> bool;
}

pub trait PriceFeeder {
	fn get_price(asset_id: &CurrencyId) -> Option<PriceDetail>;
	fn get_normal_price(asset_id: &CurrencyId) -> Option<u128>;
	fn get_oracle_amount_by_currency_and_amount_in(
		currency_in: &CurrencyId,
		amount_in: Balance,
		currency_out: &CurrencyId,
	) -> Option<Balance>;
}

pub trait EmergencyPriceFeeder<CurrencyId, Price> {
	fn set_emergency_price(asset_id: CurrencyId, price: Price);
	fn reset_emergency_price(asset_id: CurrencyId);
}

pub trait ConvertToBigUint {
	fn get_big_uint(&self) -> BigUint;
}

impl ConvertToBigUint for u128 {
	fn get_big_uint(&self) -> BigUint {
		self.to_biguint().unwrap()
	}
}

pub trait OnExchangeRateChange<CurrencyId> {
	fn on_exchange_rate_change(currency_id: &CurrencyId);
}

#[impl_trait_for_tuples::impl_for_tuples(3)]
impl<CurrencyId> OnExchangeRateChange<CurrencyId> for Tuple {
	fn on_exchange_rate_change(currency_id: &CurrencyId) {
		for_tuples!( #(
            Tuple::on_exchange_rate_change(currency_id);
        )* );
	}
}
