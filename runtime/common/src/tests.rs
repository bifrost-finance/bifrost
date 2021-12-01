use node_primitives::{CurrencyId, TokenSymbol};

use crate::{cent, micro};

#[test]
fn cal_currency_unit_by_decimal_should_work() {
	assert_eq!(1 * cent(CurrencyId::Token(TokenSymbol::DOT)), 100_000_000);
	assert_eq!(1 * micro(CurrencyId::Token(TokenSymbol::ZLK)), 1_000_000_000_000);
}
