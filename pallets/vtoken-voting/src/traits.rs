use crate::{BalanceOf, Config, CurrencyIdOf, DerivativeIndex};
use xcm::v3::MultiLocation;

pub trait DerivativeAccountHandler<T: Config> {
	fn check_derivative_index_exists(
		token: CurrencyIdOf<T>,
		derivative_index: DerivativeIndex,
	) -> bool;

	fn get_multilocation(
		token: CurrencyIdOf<T>,
		derivative_index: DerivativeIndex,
	) -> Option<MultiLocation>;

	fn get_stake_info(
		token: CurrencyIdOf<T>,
		derivative_index: DerivativeIndex,
	) -> Option<(BalanceOf<T>, BalanceOf<T>)>;
}
