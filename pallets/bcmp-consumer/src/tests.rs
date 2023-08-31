use frame_support::{assert_noop, assert_ok};
use frame_support::sp_runtime::Percent;
use sp_core::{ByteArray, H256};
use pallet_bcmp::fee::GasConfig;
use pallet_bcmp::{Message, Role};
use crate::mock::*;
use sp_core::crypto::AccountId32;
use crate::{Error, Payload};
use pallet_bcmp::ConsumerLayer;

const SOURCE_ANCHOR: H256 = H256{0: [126u8, 110, 34, 168, 219, 139, 100, 140, 226, 72, 191, 237, 236, 186, 67, 113, 237, 34, 73, 74, 11, 120, 210, 51, 152, 152, 96, 33, 185, 27, 201, 162] };

#[test]
fn test_send_message() {
	new_test_ext().execute_with(|| {
		let dst_anchor = H256::from([1u8; 32]);
		// bcmp env
		assert_ok!(Bcmp::set_whitelist(RuntimeOrigin::root(), Role::Admin, ALICE));
		assert_ok!(Bcmp::set_chain_id(RuntimeOrigin::signed(ALICE), 31337));
		assert_ok!(Bcmp::register_anchor(RuntimeOrigin::signed(ALICE), SOURCE_ANCHOR, vec![1, 2, 3]));
		assert_ok!(Bcmp::enable_path(RuntimeOrigin::signed(ALICE), 31337, dst_anchor, SOURCE_ANCHOR));
		let new_config = GasConfig {
			chain_id: 31337,
			gas_per_byte: 1,
			base_gas_amount: 1,
			gas_price: 1,
			price_ratio: Percent::from_percent(10),
			protocol_ratio: Percent::from_percent(20),
		};
		assert_ok!(Bcmp::set_fee_config(RuntimeOrigin::signed(ALICE), new_config));
		let receiver = [0u8; 32].to_vec();
		assert_ok!(
            BcmpConsumer::send_message(
                RuntimeOrigin::signed(ALICE),
                500,
                100,
                31337,
                receiver,
            )
        );
		assert_eq!(Balances::free_balance(&ALICE), 4400);
		assert_eq!(Balances::free_balance(&BcmpConsumer::resource_account()), 500);
	})
}

#[test]
fn test_receive_message() {
	new_test_ext().execute_with(|| {
		// mock send
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(BOB), BcmpConsumer::resource_account(), 100));

		let mut amount = 60u128.to_be_bytes().to_vec();
		let mut payload = [0u8; 16].to_vec();
		payload.append(&mut amount);
		payload.append(&mut <AccountId32 as AsRef<[u8; 32]>>::as_ref(&ALICE).to_vec());
		let mut message = Message {
			uid: H256::zero(),
			cross_type: H256{0: [150u8, 108, 99, 209, 73, 57, 236, 154, 206, 45, 199, 68, 245, 234, 151, 14, 28, 198, 242, 15, 18, 175, 239, 220, 223, 245, 142, 213, 211, 33, 99, 126] },
			src_anchor: H256::zero(),
			extra_fee: vec![],
			dst_anchor: SOURCE_ANCHOR,
			payload: vec![],
		};
		assert_noop!(BcmpConsumer::receive_op(&message), Error::<Test>::InvalidPayloadLength);
		message.payload = payload;
		// test_tuple_consumer, should print "call pallet consumer1"
		assert_ok!(<(Consumer1<Test>, BcmpConsumer) as ConsumerLayer<Test>>::match_consumer(&SOURCE_ANCHOR,&message));
		assert_eq!(Balances::free_balance(&ALICE), 5060);
		assert_eq!(Balances::free_balance(&BcmpConsumer::resource_account()), 40);
	})
}

#[test]
fn test_payload_encode_and_decode() {
	new_test_ext().execute_with(|| {
		let payload = BcmpConsumer::eth_api_encode(100, &ALICE.as_slice());
		let expect_bytes = [0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, ];
		assert_eq!(payload, expect_bytes.to_vec());

		let expect_struct = Payload {
			amount: 100,
			receiver: ALICE
		};
		assert_eq!(expect_struct, BcmpConsumer::parse_payload(&expect_bytes).unwrap());
	})
}
