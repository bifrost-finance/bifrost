use bifrost_primitives::{CurrencyId, TokenSymbol};
use codec::Encode;
use integration_tests_common::BifrostKusama;
use sp_runtime::BoundedVec;
use xcm::prelude::*;
use xcm_emulator::TestExt;

#[test]
fn dollar_should_work() {
	BifrostKusama::execute_with(|| {
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
