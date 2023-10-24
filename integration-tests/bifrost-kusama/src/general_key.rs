use bifrost_primitives::{CurrencyId, TokenSymbol};
use codec::Encode;
use sp_runtime::BoundedVec;
use xcm::prelude::*;

#[test]
fn dollar_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		let id = CurrencyId::Token(TokenSymbol::KSM);
		assert_eq!(
			Junction::from(BoundedVec::try_from(id.encode()).unwrap()),
			GeneralKey {
				length: 2,
				data: [
					2, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
					0, 0, 0, 0, 0, 0
				]
			}
		);
	});
}
