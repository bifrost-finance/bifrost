use super::*;
use crate::{Pallet, TotalOrderInfos};

pub fn migrate_orders<T: Config<I>, I: 'static>() -> Weight {
	// get current orders in list
	let order_iter = TotalOrderInfos::<T, I>::iter();
	let mut ok_count = 0;
	let mut err_count = 0;

	for (order_id, order_info) in order_iter {
		let owner = order_info.owner;
		let order_type = order_info.order_type;
		let vsbond = order_info.vsbond;
		let token = T::InvoicingCurrency::get();

		let (token_to_transfer, amount_to_transfer) = match order_type {
			OrderType::Buy => (token, order_info.remain_price),
			OrderType::Sell => (vsbond, order_info.remain),
		};

		let rs =
			T::MultiCurrency::ensure_can_withdraw(token_to_transfer, &owner, amount_to_transfer);

		let module_account: AccountIdOf<T> = T::PalletId::get().into_account();

		match rs {
			// keep the order and transfer the amount to the module account
			Ok(_) => {
				ok_count += 1;
				T::MultiCurrency::transfer(
					token_to_transfer,
					&owner,
					&module_account,
					amount_to_transfer,
				);
			},
			// cancel the order
			Err(_) => {
				err_count += 1;
				TotalOrderInfos::<T, I>::remove(order_id);
				Pallet::<T, I>::try_to_remove_order_id(owner, order_type, order_id);
			},
		}
	}

	// one storage read + two account balance changes
	let ok_weight =
		ok_count.saturating_mul(T::DbWeight::get().reads(1) + T::DbWeight::get().writes(2));
	let err_weight =
		err_count.saturating_mul(T::DbWeight::get().reads(1) + T::WeightInfo::revoke_order());

	ok_weight + err_weight
}
