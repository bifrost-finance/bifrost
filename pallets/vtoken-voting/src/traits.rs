use crate::{BalanceOf, Config, CurrencyIdOf};
use xcm::v3::Weight as XcmWeight;

pub trait XcmDestWeightAndFeeHandler<T: Config> {
	fn get_vote(token: CurrencyIdOf<T>) -> Option<(XcmWeight, BalanceOf<T>)>;

	fn get_remove_vote(token: CurrencyIdOf<T>) -> Option<(XcmWeight, BalanceOf<T>)>;
}
