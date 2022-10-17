use crate::kusama_integration_tests::*;
use frame_support::PalletId;
use polkadot_parachain::primitives::Id as ParaId;
use sp_runtime::{app_crypto::Ss58Codec, traits::AccountIdConversion};

pub const TREASURY_PALLET_ID: PalletId = PalletId(*b"bf/trsry");
pub const BIFROST_CROWDLOAN_ID: PalletId = PalletId(*b"bf/salp#");
pub const BIFROST_SALP_LITE_CROWDLOAN_ID: PalletId = PalletId(*b"bf/salpl");
pub const LIQUIDITY_MINING_PALLET_ID: PalletId = PalletId(*b"bf/lm###");
pub const LIQUIDITY_MINING_DOTPALLET_ID: PalletId = PalletId(*b"bf/lmdot");
pub const LIGHTENING_REDEEM_PALLET_ID: PalletId = PalletId(*b"bf/ltnrd");
pub const MERKLE_DIRTRIBUTOR_PALLET_ID: PalletId = PalletId(*b"bf/mklds");
pub const VSBOND_AUCTION_PALLET_ID: PalletId = PalletId(*b"bf/vsbnd");
pub const PARACHAIN_STAKING_PALLET_ID: PalletId = PalletId(*b"bf/stake");
pub const BIFROST_VSBOND_PALLET_ID: PalletId = PalletId(*b"bf/salpb");
pub const SLP_ENTRANCE_PALLET_ID: PalletId = PalletId(*b"bf/vtkin");
pub const SLP_EXIT_PALLET_ID: PalletId = PalletId(*b"bf/vtout");
pub const STABLE_AMM_PALLET_ID: PalletId = PalletId(*b"bf/stamm");
pub const FARMING_KEEPER_PALLET_ID: PalletId = PalletId(*b"bf/fmkpr");
pub const FARMING_REWARD_ISSUER_PALLET_ID: PalletId = PalletId(*b"bf/fmrir");
pub const SYSTEM_STAKING_PALLET_ID: PalletId = PalletId(*b"bf/sysst");
pub const BUYBACK_PALLET_ID: PalletId = PalletId(*b"bf/salpc");
pub const SYSTEM_MAKER_PALLET_ID: PalletId = PalletId(*b"bf/sysmk");

#[test]
fn parachain_account_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		assert_eq!(
			<ParaId as AccountIdConversion<AccountId>>::into_account_truncating(&ParaId::from(
				2001
			)),
			AccountId::from_ss58check("F7fq1jMmNj5j2jAHcBxgM26JzUn2N4duXu1U4UZNdkfZEPV").unwrap()
		);
		assert_eq!(
			<ParaId as AccountIdConversion<AccountId>>::into_account_truncating(&ParaId::from(
				2030
			)),
			AccountId::from_ss58check("eGJryu57ZpFQjFRiya9nGcqiGG2RZeGWuXMENq4Na7jFNjs").unwrap()
		);
	})
}

#[test]
fn pallet_id_account_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&TREASURY_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGNYdM3UjBNxcBNBqGxtz3SEEfydKragtL4pJ4F").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&BIFROST_CROWDLOAN_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGNQGSkjt2tjezJRdzUJNGgdgAihwxKQsa4LdPN").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&BIFROST_SALP_LITE_CROWDLOAN_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGNQGSkmPV2DLuQyBiG1dA5eMWAvRFGkQjZ7YDU").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&LIQUIDITY_MINING_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGMTkdAd9Z5P96SQ4UheKhx4pWNg5Pu734mRHbm").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&LIQUIDITY_MINING_DOTPALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGMTm5SYswh7EECLDTdu9gFbjt2HALzH1gazWUG").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&LIGHTENING_REDEEM_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGMTyaXsM7F3G8oR4bAgy6qU27xfobQkzN5CH3h").unwrap()
		);
		//
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&MERKLE_DIRTRIBUTOR_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGMbYEwSupkwsPfjzzSGSf9MWxCHf1mEUyUyc4Y").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&VSBOND_AUCTION_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGNpLKqNkBEJLLkTkSzHt41QDhqiECF5shU8Qsp").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&PARACHAIN_STAKING_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGNQr7VZ48fkCX5vkt1H16F8Np9g2hYssRXHZJF").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&BIFROST_VSBOND_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGNQGSkmBWD7AfnKVcXzjriDdZs2ezEjKfsR4h5").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&SLP_ENTRANCE_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGNpNATVMhrmGeJj8drJWWGXrHtVo4Vym2cUnSz").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&SLP_EXIT_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGNpNC5wLwUw92XQSHZ9tyrU5SYVDmRyodCrdAR").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&STABLE_AMM_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGNQr7Vjo4uHNjwii1g4zfTHyWC5iBMrQj7R4P2").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&FARMING_KEEPER_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGLejANY2XNJzg7B8cB4mBx8Rbw4tXHpY6GK5YE").unwrap()
		);
		//
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&FARMING_REWARD_ISSUER_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGLejDBGEgnWnj8mNePufttUVCjF3snTttG6SDZ").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&SYSTEM_STAKING_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGNR17nzbZNJxo7G9mYziLiJcujnWXCNB2CUakX").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&BUYBACK_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGNQGSkmChh1z6Lz4d7CRneZWAJ22p98daSPJsF").unwrap()
		);
		assert_eq!(
			<PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&SYSTEM_MAKER_PALLET_ID
			),
			AccountId::from_ss58check("eCSrvbA5gGNR17nSgitMhUyrqX8e4a8wuk5Q7UKUBhxdi5S").unwrap()
		);
	})
}
