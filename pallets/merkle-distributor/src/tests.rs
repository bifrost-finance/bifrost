use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use hex_literal::hex;
use orml_traits::MultiCurrency;
use sp_core::H256;
use sp_runtime::AccountId32;

#[test]
fn one_level_merkel_tree_proof_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(MdPallet::create_merkle_distributor(
            Origin::root(),
            H256::from(&hex!(
                "056980ee78588f3d5ceab5645b2dc2838c19f938151bc1c70547664c6bf57932"
            )),
            Vec::from("test"),
            CURRENCY_TEST1,
            1_000_000_000 * UNIT,
        ));

        assert_ok!(MdPallet::charge(Origin::signed(ALICE), 0,));

        let mut proof = Vec::<H256>::new();
        proof.push(H256::from(&hex!(
            "5d6763b1aaa996a5854b019d1bd087543a1c5977d0d8c448380ca6b953007b78"
        )));
        assert_ok!(MdPallet::claim(
            Origin::signed(ALICE),
            0,
            0,
            BOB,
            10_000_000_000_000_000_000,
            proof
        ));

        assert_eq!(
            Tokens::free_balance(CURRENCY_TEST1, &BOB),
            10_000_000_000_000_000_000
        );
    })
}

#[test]
fn set_claimed_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(MdPallet::create_merkle_distributor(
            Origin::root(),
            H256::from(&hex!(
                "056980ee78588f3d5ceab5645b2dc2838c19f938151bc1c70547664c6bf57932"
            )),
            Vec::from("test"),
            CURRENCY_TEST1,
            100 * UNIT,
        ));

        assert_ok!(MdPallet::create_merkle_distributor(
            Origin::root(),
            H256::from(&hex!(
                "056980ee78588f3d5ceab5645b2dc2838c19f938151bc1c70547664c6bf57932"
            )),
            Vec::from("test2"),
            CURRENCY_TEST1,
            100 * UNIT,
        ));

        for i in 0u32..20000 {
            MdPallet::set_claimed(0, i);
            MdPallet::set_claimed(1, i);
        }

        for i in 0u32..20000 {
            assert!(MdPallet::is_claimed(0, i));
            assert!(MdPallet::is_claimed(1, i));
        }
    })
}

#[test]
fn no_set_claimed_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(MdPallet::create_merkle_distributor(
            Origin::root(),
            H256::from(&hex!(
                "056980ee78588f3d5ceab5645b2dc2838c19f938151bc1c70547664c6bf57932"
            )),
            Vec::from("test"),
            CURRENCY_TEST1,
            100 * UNIT,
        ));

        assert_ok!(MdPallet::create_merkle_distributor(
            Origin::root(),
            H256::from(&hex!(
                "056980ee78588f3d5ceab5645b2dc2838c19f938151bc1c70547664c6bf57932"
            )),
            Vec::from("test2"),
            CURRENCY_TEST1,
            100 * UNIT,
        ));

        for i in 0u32..2000 {
            MdPallet::set_claimed(0, i);
            MdPallet::set_claimed(1, i);
        }

        for i in 2000u32..20000 {
            assert_eq!(MdPallet::is_claimed(0, i), false);
            assert_eq!(MdPallet::is_claimed(1, i), false);
        }
    })
}

#[test]
fn one_hundred_element_merkle_proof_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(MdPallet::create_merkle_distributor(
            Origin::root(),
            H256::from(&hex!(
                "c5a4b4dbe724bfb5aac5879fa145e98686e3e77aacacfc7e6dbea5daa587af3f"
            )),
            Vec::from("test"),
            CURRENCY_TEST1,
            1_000 * UNIT,
        ));

        let holder = AccountId32::new([109, 111, 100, 108, 122, 108, 107, 47, 109, 100, 42, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        assert_ok!(MdPallet::charge(Origin::signed(ALICE), 0,));

        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder), 1_000 * UNIT);

        // owner = bmtJRHykbp8zN33Hd58poQjMrPXaUiJTkY4XYJvxuz3yTQV
        let owner_0: AccountId = AccountId32::new([
            2, 59, 109, 111, 93, 159, 178, 207, 61, 193, 214, 44, 30, 24, 172, 6, 166, 86, 208, 19,
            81, 244, 212, 48, 252, 107, 222, 166, 182, 88, 246, 56,
        ]);
        let owner_0_proof = vec![
            H256::from(hex![
                "fb4c1fdb961b33fe34628c4a3a99f05d26c06f053000f0eab04ddd2b7857b29d"
            ]),
            H256::from(hex![
                "db9586d9476f100d3d63c9fd04925abe451eee1416358de45576cedce9c7b197"
            ]),
            H256::from(hex![
                "0564e3219c5663052dbc56d34a194628e134eb3852025202acacfa5be20995a2"
            ]),
            H256::from(hex![
                "246dcb49ecfe475d689d26a428d7904a28689c72fb35229ac5484ea9b08baefb"
            ]),
        ];

        assert_ok!(MdPallet::claim(
            Origin::signed(ALICE),
            0,
            1,
            owner_0.clone(),
            291,
            owner_0_proof
        ));

        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &owner_0), 291);
        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder), 1_000 * UNIT - 291);

        //owner_1 = bugDULrUyEM2u86pJQDeshjasknTEjyvepwgSj8DQMwF9B1
        let owner_1: AccountId = AccountId32::new([
            8, 44, 181, 61, 98, 153, 220, 3, 62, 70, 125, 224, 7, 191, 213, 196, 192, 210, 65, 53,
            170, 133, 210, 241, 217, 131, 0, 143, 247, 143, 187, 102,
        ]);

        let owner_1_proof = vec![
            H256::from(hex![
                "56d2dd4584750f5caba96b0d173c3153690217083f90b3225bf20613f37cfc6e"
            ]),
            H256::from(hex![
                "16c669ec7494718083a0c90dda957e52f949279c0b6d256efa932823c24c274b"
            ]),
            H256::from(hex![
                "f69d09074895fb0871a765aee5af6979811de9b50227c84bd82c827f7bdc0319"
            ]),
            H256::from(hex![
                "7a287f4a7e94d1dc13ad4ba4a05c319f39dcf65ba3f3c78c0a1bb228f76a2614"
            ]),
            H256::from(hex![
                "7503cfd3964854d9a83faf828b385a81b20e0ff662ffe0c989c486ccd628c8a1"
            ]),
            H256::from(hex![
                "e77f4b91745d7a37dc482e1ff09fb06b335ccccd65f55146ce0b5341e1476273"
            ]),
            H256::from(hex![
                "86ceded7066a10bdb521f5d2d28c037565f85e17e411019e88f4337140ac3b97"
            ]),
        ];

        assert_ok!(MdPallet::claim(
            Origin::signed(ALICE),
            0,
            2,
            owner_1.clone(),
            291,
            owner_1_proof
        ));

        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &owner_1), 291);
        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder), 1_000 * UNIT - 291 * 2);
    })
}

#[test]
fn claim_other_reward_merkle_proof_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(MdPallet::create_merkle_distributor(
            Origin::root(),
            H256::from(&hex!(
                "c5a4b4dbe724bfb5aac5879fa145e98686e3e77aacacfc7e6dbea5daa587af3f"
            )),
            Vec::from("test"),
            CURRENCY_TEST1,
            1_000 * UNIT,
        ));

        let holder = AccountId32::new([109, 111, 100, 108, 122, 108, 107, 47, 109, 100, 42, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        assert_ok!(MdPallet::charge(Origin::signed(ALICE), 0,));

        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder), 1_000 * UNIT);

        // proof owner = bmtJRHykbp8zN33Hd58poQjMrPXaUiJTkY4XYJvxuz3yTQV
        let proof = vec![
            H256::from(hex![
                "fb4c1fdb961b33fe34628c4a3a99f05d26c06f053000f0eab04ddd2b7857b29d"
            ]),
            H256::from(hex![
                "db9586d9476f100d3d63c9fd04925abe451eee1416358de45576cedce9c7b197"
            ]),
            H256::from(hex![
                "0564e3219c5663052dbc56d34a194628e134eb3852025202acacfa5be20995a2"
            ]),
            H256::from(hex![
                "246dcb49ecfe475d689d26a428d7904a28689c72fb35229ac5484ea9b08baefb"
            ]),
        ];

        assert_noop!(
            MdPallet::claim(Origin::signed(ALICE), 0, 1, BOB, 291, proof),
            Error::<Runtime>::MerkleVerifyFailed
        );

        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &BOB), 0);
        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder), 1_000 * UNIT);
    })
}

#[test]
fn claim_towice_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(MdPallet::create_merkle_distributor(
            Origin::root(),
            H256::from(&hex!(
                "056980ee78588f3d5ceab5645b2dc2838c19f938151bc1c70547664c6bf57932"
            )),
            Vec::from("test"),
            CURRENCY_TEST1,
            1_000_000_000 * UNIT,
        ));

        assert_ok!(MdPallet::charge(Origin::signed(ALICE), 0,));

        let holder = AccountId32::new([109, 111, 100, 108, 122, 108, 107, 47, 109, 100, 42, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder), 1_000_000_000 * UNIT);

        let mut proof = Vec::<H256>::new();
        proof.push(H256::from(&hex!(
            "5d6763b1aaa996a5854b019d1bd087543a1c5977d0d8c448380ca6b953007b78"
        )));

        assert_ok!(MdPallet::claim(
            Origin::signed(ALICE),
            0,
            0,
            BOB,
            10_000_000 * UNIT,
            proof.clone()
        ));

        assert_eq!(
            Tokens::free_balance(CURRENCY_TEST1, &BOB),
            10_000_000 * UNIT
        );

        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder), 1_000_000_000 * UNIT - 10_000_000 * UNIT);

        assert_noop!(MdPallet::claim(
            Origin::signed(BOB),
            0,
            0,
            BOB,
            10_000_000 * UNIT,
            proof
        ), Error::<Runtime>::Claimed);

        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder), 1_000_000_000 * UNIT - 10_000_000 * UNIT);

        assert_eq!(
            Tokens::free_balance(CURRENCY_TEST1, &BOB),
            10_000_000 * UNIT
        );
    })
}

#[test]
fn claim_use_worng_index_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(MdPallet::create_merkle_distributor(
            Origin::root(),
            H256::from(&hex!(
                "056980ee78588f3d5ceab5645b2dc2838c19f938151bc1c70547664c6bf57932"
            )),
            Vec::from("test"),
            CURRENCY_TEST1,
            1_000_000_000 * UNIT,
        ));

        assert_ok!(MdPallet::charge(Origin::signed(ALICE), 0,));

        let holder = AccountId32::new([109, 111, 100, 108, 122, 108, 107, 47, 109, 100, 42, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder), 1_000_000_000 * UNIT);

        let mut proof = Vec::<H256>::new();
        proof.push(H256::from(&hex!(
            "5d6763b1aaa996a5854b019d1bd087543a1c5977d0d8c448380ca6b953007b78"
        )));

        assert_noop!(MdPallet::claim(
            Origin::signed(ALICE),
            0,
            2,
            BOB,
            10_000_000 * UNIT,
            proof
        ), Error::<Runtime>::MerkleVerifyFailed);

        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder), 1_000_000_000 * UNIT);

        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &BOB), 0);
    })
}

#[test]
fn claim_use_worng_amount_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(MdPallet::create_merkle_distributor(
            Origin::root(),
            H256::from(&hex!(
                "056980ee78588f3d5ceab5645b2dc2838c19f938151bc1c70547664c6bf57932"
            )),
            Vec::from("test"),
            CURRENCY_TEST1,
            1_000_000_000 * UNIT,
        ));

        assert_ok!(MdPallet::charge(Origin::signed(ALICE), 0,));

        let holder = AccountId32::new([109, 111, 100, 108, 122, 108, 107, 47, 109, 100, 42, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder), 1_000_000_000 * UNIT);

        let mut proof = Vec::<H256>::new();
        proof.push(H256::from(&hex!(
            "5d6763b1aaa996a5854b019d1bd087543a1c5977d0d8c448380ca6b953007b78"
        )));

        assert_noop!(MdPallet::claim(
            Origin::signed(ALICE),
            0,
            1,
            BOB,
            10_000_000 * UNIT + 10,
            proof
        ), Error::<Runtime>::MerkleVerifyFailed);

        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder), 1_000_000_000 * UNIT);

        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &BOB), 0);
    })
}

#[test]
fn create_multi_merkle_distributor_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(MdPallet::create_merkle_distributor(
            Origin::root(),
            H256::from(&hex!(
                "056980ee78588f3d5ceab5645b2dc2838c19f938151bc1c70547664c6bf57932"
            )),
            Vec::from("test"),
            CURRENCY_TEST1,
            1_000_000_000 * UNIT,
        ));

        assert_ok!(MdPallet::create_merkle_distributor(
            Origin::root(),
            H256::from(&hex!(
                "c5a4b4dbe724bfb5aac5879fa145e98686e3e77aacacfc7e6dbea5daa587af3f"
            )),
            Vec::from("test"),
            CURRENCY_TEST1,
            1_000 * UNIT,
        ));

        let holder_0 = AccountId32::new([109, 111, 100, 108, 122, 108, 107, 47, 109, 100, 42, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let holder_1 = AccountId32::new([109, 111, 100, 108, 122, 108, 107, 47, 109, 100, 42, 42, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder_0), 0);
        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder_1), 0);

        assert_ok!(MdPallet::charge(Origin::signed(ALICE), 0,));
        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder_0), 1_000_000_000 * UNIT);
        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder_1), 0);

        assert_ok!(MdPallet::charge(Origin::signed(ALICE), 1,));
        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder_0), 1_000_000_000 * UNIT);
        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder_1), 1_000 * UNIT);

        let mut proof = Vec::<H256>::new();
        proof.push(H256::from(&hex!(
            "5d6763b1aaa996a5854b019d1bd087543a1c5977d0d8c448380ca6b953007b78"
        )));
        assert_ok!(MdPallet::claim(
            Origin::signed(ALICE),
            0,
            0,
            BOB,
            10_000_000_000_000_000_000,
            proof
        ));

        assert_eq!(
            Tokens::free_balance(CURRENCY_TEST1, &BOB),
            10_000_000 * UNIT
        );

        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder_1), 1_000 * UNIT);
    })
}

#[test]
fn charger_must_has_enough_currency_should_work(){
    new_test_ext().execute_with(|| {
        assert_ok!(MdPallet::create_merkle_distributor(
            Origin::root(),
            H256::from(&hex!(
                "056980ee78588f3d5ceab5645b2dc2838c19f938151bc1c70547664c6bf57932"
            )),
            Vec::from("test"),
            CURRENCY_TEST1,
            1_000_000_000 * UNIT,
        ));

        MdPallet::charge(Origin::signed(BOB), 0,);

        let holder_0 = AccountId32::new([109, 111, 100, 108, 122, 108, 107, 47, 109, 100, 42, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder_0), 0);
    })
}

#[test]
fn charge_towice_should_not_work(){
    new_test_ext().execute_with(|| {
        assert_ok!(MdPallet::create_merkle_distributor(
            Origin::root(),
            H256::from(&hex!(
                "056980ee78588f3d5ceab5645b2dc2838c19f938151bc1c70547664c6bf57932"
            )),
            Vec::from("test"),
            CURRENCY_TEST1,
            1_000_000_000 * UNIT,
        ));
        assert_ok!(MdPallet::charge(Origin::signed(ALICE), 0,));
        let holder_0 = AccountId32::new([109, 111, 100, 108, 122, 108, 107, 47, 109, 100, 42, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(Tokens::free_balance(CURRENCY_TEST1, &holder_0), 1_000_000_000 * UNIT);

        assert_noop!(MdPallet::charge(Origin::signed(ALICE), 0,), Error::<Runtime>::Charged);
    })
}