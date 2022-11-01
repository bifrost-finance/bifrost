// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use super::*;
use bifrost_asset_registry::AssetIdMaps;
use codec::{Decode, Encode};
pub use cumulus_primitives_core::ParaId;
use frame_support::{
	parameter_types,
	sp_runtime::traits::{CheckedConversion, Convert},
	traits::Get,
};
use node_primitives::{AccountId, CurrencyId, CurrencyIdMapping, TokenSymbol};
pub use polkadot_parachain::primitives::Sibling;
use sp_std::{convert::TryFrom, marker::PhantomData};
pub use xcm_builder::{
	AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, CurrencyAdapter, EnsureXcmOrigin, FixedRateOfFungible,
	FixedWeightBounds, IsConcrete, LocationInverter, ParentAsSuperuser, ParentIsPreset,
	RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeRevenue,
	TakeWeightCredit,
};
use xcm_executor::traits::{FilterAssetLocation, MatchesFungible};
pub use xcm_interface::traits::{parachains, XcmBaseWeight};

// orml imports
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::location::Reserve;
pub use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key, MultiCurrency};
use orml_xcm_support::{DepositToAlternative, MultiCurrencyAdapter};
use pallet_xcm::XcmPassthrough;

parameter_types! {
	pub const KsmLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = prod_or_test!(NetworkId::Kusama, NetworkId::Any);
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub SelfParaChainId: CumulusParaId = ParachainInfo::parachain_id();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, Origin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, Origin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
	// Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
	// transaction from the Root origin.
	ParentAsSuperuser<Origin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, Origin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<Origin>,
);

parameter_types! {
	// One XCM operation is 200_000_000 weight, cross-chain transfer ~= 2x of transfer = 3_000_000_000
	pub UnitWeightCost: Weight = 200_000_000;
	pub const MaxInstructions: u32 = 100;
}

match_types! {
	pub type ParentOrParentsExecutivePlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Executive, .. }) }
	};
}

pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	AllowKnownQueryResponses<PolkadotXcm>,
	AllowSubscriptionsFrom<Everything>,
);

pub type BifrostAssetTransactor = MultiCurrencyAdapter<
	Currencies,
	UnknownTokens,
	BifrostAssetMatcher<CurrencyId, BifrostCurrencyIdConvert<SelfParaChainId>>,
	AccountId,
	LocationToAccountId,
	CurrencyId,
	BifrostCurrencyIdConvert<SelfParaChainId>,
	DepositToAlternative<BifrostTreasuryAccount, Currencies, CurrencyId, AccountId, Balance>,
>;

parameter_types! {
	pub KsmPerSecond: (AssetId, u128) = (MultiLocation::parent().into(), ksm_per_second::<Runtime>());
	pub VsksmPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(SelfParaId::get()), GeneralKey((CurrencyId::VSToken(TokenSymbol::KSM).encode()).try_into().unwrap()))
		).into(),
		ksm_per_second::<Runtime>()
	);
	pub VsksmNewPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			0,
			X1(GeneralKey((CurrencyId::VSToken(TokenSymbol::KSM).encode()).try_into().unwrap()))
		).into(),
		ksm_per_second::<Runtime>()
	);
	pub BncPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(SelfParaId::get()), GeneralKey((NativeCurrencyId::get().encode()).try_into().unwrap()))
		).into(),
		// BNC:KSM = 80:1
		ksm_per_second::<Runtime>() * 80
	);
	pub BncNewPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			0,
			X1(GeneralKey((NativeCurrencyId::get().encode()).try_into().unwrap()))
		).into(),
		// BNC:KSM = 80:1
		ksm_per_second::<Runtime>() * 80
	);
	pub ZlkPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(SelfParaId::get()), GeneralKey(CurrencyId::Token(TokenSymbol::ZLK).encode().try_into().unwrap()))
		).into(),
		// ZLK:KSM = 150:1
		//ZLK has a decimal of 18, while KSM is 12.
		ksm_per_second::<Runtime>() * 150 * 1_000_000
	);
	pub ZlkNewPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			0,
			X1(GeneralKey((CurrencyId::Token(TokenSymbol::ZLK).encode()).try_into().unwrap()))
		).into(),
		// ZLK:KSM = 150:1
		//ZLK has a decimal of 18, while KSM is 12.
		ksm_per_second::<Runtime>() * 150 * 1_000_000
	);
	pub KarPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::karura::ID), GeneralKey((parachains::karura::KAR_KEY.to_vec()).try_into().unwrap()))
		).into(),
		// KAR:KSM = 100:1
		ksm_per_second::<Runtime>() * 100
	);
	pub KusdPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::karura::ID), GeneralKey((parachains::karura::KUSD_KEY.to_vec()).try_into().unwrap()))
		).into(),
		// kUSD:KSM = 400:1
		ksm_per_second::<Runtime>() * 400
	);
	pub PhaPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X1(Parachain(parachains::phala::ID)),
		).into(),
		// PHA:KSM = 400:1
		ksm_per_second::<Runtime>() * 400
	);
	pub RmrkPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::Statemine::ID), GeneralIndex(parachains::Statemine::RMRK_ID.into()))
		).into(),
		// rmrk:KSM = 10:1
		ksm_per_second::<Runtime>() * 10 / 100 //rmrk currency decimal as 10
	);
	pub RmrkNewPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X3(Parachain(parachains::Statemine::ID), PalletInstance(parachains::Statemine::PALLET_ID),GeneralIndex(parachains::Statemine::RMRK_ID.into()))
		).into(),
		// rmrk:KSM = 10:1
		ksm_per_second::<Runtime>() * 10 / 100 //rmrk currency decimal as 10
	);
	pub MovrPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::moonriver::ID), PalletInstance(parachains::moonriver::PALLET_ID.into()))
		).into(),
		// MOVR:KSM = 2.67:1
		ksm_per_second::<Runtime>() * 267 * 10_000 //movr currency decimal as 18
	);
	pub BasePerSecond: u128 = ksm_per_second::<Runtime>();
}

pub struct ToTreasury;
impl TakeRevenue for ToTreasury {
	fn take_revenue(revenue: MultiAsset) {
		if let MultiAsset { id: Concrete(location), fun: Fungible(amount) } = revenue {
			if let Some(currency_id) =
				BifrostCurrencyIdConvert::<SelfParaChainId>::convert(location)
			{
				let _ = Currencies::deposit(currency_id, &BifrostTreasuryAccount::get(), amount);
			}
		}
	}
}

pub type Trader = (
	FixedRateOfFungible<KsmPerSecond, ToTreasury>,
	FixedRateOfFungible<VsksmPerSecond, ToTreasury>,
	FixedRateOfFungible<VsksmNewPerSecond, ToTreasury>,
	FixedRateOfFungible<BncPerSecond, ToTreasury>,
	FixedRateOfFungible<BncNewPerSecond, ToTreasury>,
	FixedRateOfFungible<ZlkPerSecond, ToTreasury>,
	FixedRateOfFungible<ZlkNewPerSecond, ToTreasury>,
	FixedRateOfFungible<KarPerSecond, ToTreasury>,
	FixedRateOfFungible<KusdPerSecond, ToTreasury>,
	FixedRateOfFungible<PhaPerSecond, ToTreasury>,
	FixedRateOfFungible<RmrkPerSecond, ToTreasury>,
	FixedRateOfFungible<RmrkNewPerSecond, ToTreasury>,
	FixedRateOfFungible<MovrPerSecond, ToTreasury>,
	FixedRateOfAsset<Runtime, BasePerSecond, ToTreasury>,
);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type AssetClaims = PolkadotXcm;
	type AssetTransactor = BifrostAssetTransactor;
	type AssetTrap = PolkadotXcm;
	type Barrier = Barrier;
	type Call = Call;
	type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
	type IsTeleporter = ();
	type LocationInverter = LocationInverter<Ancestry>;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type ResponseHandler = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type Trader = Trader;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type XcmSender = XcmRouter;
}

/// Local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
	type Event = Event;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type LocationInverter = LocationInverter<Ancestry>;
	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type XcmExecuteFilter = Nothing;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmReserveTransferFilter = Everything;
	type XcmRouter = XcmRouter;
	type XcmTeleportFilter = Nothing;
	type Origin = Origin;
	type Call = Call;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type ChannelInfo = ParachainSystem;
	type Event = Event;
	type VersionWrapper = PolkadotXcm;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = ();
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type Event = Event;
	type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl orml_currencies::Config for Runtime {
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
	type WeightInfo = ();
}

parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		match currency_id {
			&CurrencyId::Native(TokenSymbol::BNC) => 10 * milli::<Runtime>(NativeCurrencyId::get()),   // 0.01 BNC
			&CurrencyId::Stable(TokenSymbol::KUSD) => 10 * millicent::<Runtime>(StableCurrencyId::get()),
			&CurrencyId::Token(TokenSymbol::KSM) => 10 * millicent::<Runtime>(RelayCurrencyId::get()),  // 0.0001 KSM
			&CurrencyId::Token(TokenSymbol::KAR) => 10 * millicent::<Runtime>(CurrencyId::Token(TokenSymbol::KAR)),
			&CurrencyId::Token(TokenSymbol::DOT) => 1 * cent::<Runtime>(PolkadotCurrencyId::get()),  // DOT has a decimals of 10e10, 0.01 DOT
			&CurrencyId::Token(TokenSymbol::ZLK) => 1 * micro::<Runtime>(CurrencyId::Token(TokenSymbol::ZLK)),	// ZLK has a decimals of 10e18
			&CurrencyId::Token(TokenSymbol::PHA) => 4 * cent::<Runtime>(CurrencyId::Token(TokenSymbol::PHA)),	// 0.04 PHA, PHA has a decimals of 10e12.
			&CurrencyId::VSToken(TokenSymbol::KSM) => 10 * millicent::<Runtime>(RelayCurrencyId::get()),
			&CurrencyId::VSToken(TokenSymbol::DOT) => 1 * cent::<Runtime>(PolkadotCurrencyId::get()),
			&CurrencyId::VSBond(TokenSymbol::BNC, ..) => 10 * millicent::<Runtime>(NativeCurrencyId::get()),
			&CurrencyId::VSBond(TokenSymbol::KSM, ..) => 10 * millicent::<Runtime>(RelayCurrencyId::get()),
			&CurrencyId::VSBond(TokenSymbol::DOT, ..) => 1 * cent::<Runtime>(PolkadotCurrencyId::get()),
			&CurrencyId::LPToken(..) => 10 * millicent::<Runtime>(NativeCurrencyId::get()),
			&CurrencyId::StableLpToken(..) => 10 * millicent::<Runtime>(NativeCurrencyId::get()),
			&CurrencyId::VToken(TokenSymbol::KSM) => 10 * millicent::<Runtime>(RelayCurrencyId::get()),  // 0.0001 vKSM
			&CurrencyId::Token(TokenSymbol::RMRK) => 1 * micro::<Runtime>(CurrencyId::Token(TokenSymbol::RMRK)),
			&CurrencyId::Token(TokenSymbol::MOVR) => 1 * micro::<Runtime>(CurrencyId::Token(TokenSymbol::MOVR)),	// MOVR has a decimals of 10e18
			&CurrencyId::VToken(TokenSymbol::MOVR) => 1 * micro::<Runtime>(CurrencyId::Token(TokenSymbol::MOVR)),	// MOVR has a decimals of 10e18
			CurrencyId::ForeignAsset(foreign_asset_id) => {
				AssetIdMaps::<Runtime>::get_asset_metadata(AssetIds::ForeignAssetId(*foreign_asset_id)).
					map_or(Balance::max_value(), |metatata| metatata.minimal_balance)
			},
			_ => AssetIdMaps::<Runtime>::get_currency_metadata(*currency_id)
				.map_or(Balance::max_value(), |metatata| metatata.minimal_balance)
		}
	};
}

pub struct DustRemovalWhitelist;
impl Contains<AccountId> for DustRemovalWhitelist {
	fn contains(a: &AccountId) -> bool {
		AccountIdConversion::<AccountId>::into_account_truncating(&TreasuryPalletId::get()).eq(a) ||
			AccountIdConversion::<AccountId>::into_account_truncating(&BifrostCrowdloanId::get())
				.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(
			&BifrostSalpLiteCrowdloanId::get(),
		)
		.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(
			&LighteningRedeemPalletId::get(),
		)
		.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(
			&VsbondAuctionPalletId::get(),
		)
		.eq(a) || LiquidityMiningPalletId::get().check_sub_account::<PoolId>(a) ||
			LiquidityMiningDOTPalletId::get().check_sub_account::<PoolId>(a) ||
			AccountIdConversion::<AccountId>::into_account_truncating(
				&ParachainStakingPalletId::get(),
			)
			.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(
			&BifrostVsbondPalletId::get(),
		)
		.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(
			&SlpEntrancePalletId::get(),
		)
		.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(&SlpExitPalletId::get())
			.eq(a) || FarmingKeeperPalletId::get().check_sub_account::<PoolId>(a) ||
			FarmingRewardIssuerPalletId::get().check_sub_account::<PoolId>(a) ||
			AccountIdConversion::<AccountId>::into_account_truncating(
				&SystemStakingPalletId::get(),
			)
			.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(&BuybackPalletId::get())
			.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(
			&SystemMakerPalletId::get(),
		)
		.eq(a) || FeeSharePalletId::get().check_sub_account::<DistributionId>(a)
	}
}

parameter_types! {
	pub BifrostTreasuryAccount: AccountId = TreasuryPalletId::get().into_account_truncating();
}

impl orml_tokens::Config for Runtime {
	type Amount = Amount;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type DustRemovalWhitelist = DustRemovalWhitelist;
	type Event = Event;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type OnDust = orml_tokens::TransferDust<Runtime, BifrostTreasuryAccount>;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type OnNewTokenAccount = ();
	type OnKilledTokenAccount = ();
}

parameter_types! {
	pub SelfLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(ParachainInfo::get().into())));
	pub RelayXcmBaseWeight: u64 = milli::<Runtime>(RelayCurrencyId::get()) as u64;
	pub const MaxAssetsForTransfer: usize = 2;
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: MultiLocation| -> Option<u128> {
		Some(u128::MAX)
	};
}

impl orml_xtokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = BifrostCurrencyIdConvert<ParachainInfo>;
	type AccountIdToMultiLocation = BifrostAccountIdToMultiLocation;
	type LocationInverter = LocationInverter<Ancestry>;
	type SelfLocation = SelfLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type BaseXcmWeight = RelayXcmBaseWeight;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type MinXcmFee = ParachainMinFee;
	type MultiLocationsFilter = Everything;
	type ReserveProvider = AbsoluteReserveProvider;
}

impl orml_unknown_tokens::Config for Runtime {
	type Event = Event;
}

impl orml_xcm::Config for Runtime {
	type Event = Event;
	type SovereignOrigin = MoreThanHalfCouncil;
}

parameter_types! {
	pub ParachainAccount: AccountId = ParachainInfo::get().into_account_truncating();
	pub ContributionWeight:XcmBaseWeight = RelayXcmBaseWeight::get().into();
	pub UmpTransactFee: Balance = prod_or_test!(milli::<Runtime>(RelayCurrencyId::get()),milli::<Runtime>(RelayCurrencyId::get()) * 100);
	pub StatemineTransferFee: Balance = milli::<Runtime>(RelayCurrencyId::get()) * 4;
	pub StatemineTransferWeight:XcmBaseWeight = (RelayXcmBaseWeight::get() * 4).into();
}

impl xcm_interface::Config for Runtime {
	type Event = Event;
	type UpdateOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type MultiCurrency = Currencies;
	type RelayNetwork = RelayNetwork;
	type RelaychainCurrencyId = RelayCurrencyId;
	type ParachainSovereignAccount = ParachainAccount;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type AccountIdToMultiLocation = BifrostAccountIdToMultiLocation;
	type StatemineTransferWeight = StatemineTransferWeight;
	type StatemineTransferFee = StatemineTransferFee;
	type ContributionWeight = ContributionWeight;
	type ContributionFee = UmpTransactFee;
}

/// Bifrost Asset Matcher
pub struct BifrostAssetMatcher<CurrencyId, CurrencyIdConvert>(
	PhantomData<(CurrencyId, CurrencyIdConvert)>,
);

impl<CurrencyId, CurrencyIdConvert, Amount> MatchesFungible<Amount>
	for BifrostAssetMatcher<CurrencyId, CurrencyIdConvert>
where
	CurrencyIdConvert: Convert<MultiLocation, Option<CurrencyId>>,
	Amount: TryFrom<u128>,
{
	fn matches_fungible(a: &MultiAsset) -> Option<Amount> {
		if let (Fungible(ref amount), Concrete(ref location)) = (&a.fun, &a.id) {
			if CurrencyIdConvert::convert(location.clone()).is_some() {
				return CheckedConversion::checked_from(*amount);
			}
		}
		None
	}
}

/// A `FilterAssetLocation` implementation. Filters multi native assets whose
/// reserve is same with `origin`.
pub struct MultiNativeAsset<ReserveProvider>(PhantomData<ReserveProvider>);
impl<ReserveProvider> FilterAssetLocation for MultiNativeAsset<ReserveProvider>
where
	ReserveProvider: Reserve,
{
	fn filter_asset_location(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		if let Some(ref reserve) = ReserveProvider::reserve(asset) {
			if reserve == origin {
				return true;
			}
		}
		false
	}
}

fn native_currency_location(id: CurrencyId, para_id: ParaId) -> MultiLocation {
	MultiLocation::new(
		1,
		X2(Parachain(para_id.into()), GeneralKey((id.encode()).try_into().unwrap())),
	)
}

impl<T: Get<ParaId>> Convert<MultiAsset, Option<CurrencyId>> for BifrostCurrencyIdConvert<T> {
	fn convert(asset: MultiAsset) -> Option<CurrencyId> {
		if let MultiAsset { id: Concrete(id), fun: Fungible(_) } = asset {
			Self::convert(id)
		} else {
			None
		}
	}
}

pub struct BifrostAccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for BifrostAccountIdToMultiLocation {
	fn convert(account: AccountId) -> MultiLocation {
		X1(AccountId32 { network: NetworkId::Any, id: account.into() }).into()
	}
}

pub struct BifrostCurrencyIdConvert<T>(sp_std::marker::PhantomData<T>);
impl<T: Get<ParaId>> Convert<CurrencyId, Option<MultiLocation>> for BifrostCurrencyIdConvert<T> {
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		use CurrencyId::*;
		use TokenSymbol::*;

		if let Some(id) = AssetIdMaps::<Runtime>::get_multi_location(id) {
			return Some(id);
		}

		match id {
			Token(KSM) => Some(MultiLocation::parent()),
			Native(ASG) | Native(BNC) | VSToken(KSM) | Token(ZLK) =>
				Some(native_currency_location(id, T::get())),
			// Karura currencyId types
			Token(KAR) => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::karura::ID),
					GeneralKey((parachains::karura::KAR_KEY.to_vec()).try_into().unwrap()),
				),
			)),
			Stable(KUSD) => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::karura::ID),
					GeneralKey((parachains::karura::KUSD_KEY.to_vec()).try_into().unwrap()),
				),
			)),
			Token(RMRK) => Some(MultiLocation::new(
				1,
				X3(
					Parachain(parachains::Statemine::ID),
					PalletInstance(parachains::Statemine::PALLET_ID),
					GeneralIndex(parachains::Statemine::RMRK_ID as u128),
				),
			)),
			// Phala Native token
			Token(PHA) => Some(MultiLocation::new(1, X1(Parachain(parachains::phala::ID)))),
			// Moonriver Native token
			Token(MOVR) => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::moonriver::ID),
					PalletInstance(parachains::moonriver::PALLET_ID.into()),
				),
			)),
			_ => None,
		}
	}
}

impl<T: Get<ParaId>> Convert<MultiLocation, Option<CurrencyId>> for BifrostCurrencyIdConvert<T> {
	fn convert(location: MultiLocation) -> Option<CurrencyId> {
		use CurrencyId::*;
		use TokenSymbol::*;

		if location == MultiLocation::parent() {
			return Some(Token(KSM));
		}

		if let Some(currency_id) = AssetIdMaps::<Runtime>::get_currency_id(location.clone()) {
			return Some(currency_id);
		}

		match location {
			MultiLocation { parents, interior } if parents == 1 => match interior {
				X2(Parachain(id), GeneralKey(key)) if ParaId::from(id) == T::get() => {
					// decode the general key
					if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
						match currency_id {
							Native(ASG) | Native(BNC) | VSToken(KSM) | Token(ZLK) =>
								Some(currency_id),
							_ => None,
						}
					} else {
						None
					}
				},
				X2(Parachain(id), GeneralKey(key)) if id == parachains::karura::ID => {
					if key == parachains::karura::KAR_KEY.to_vec() {
						Some(Token(KAR))
					} else if key == parachains::karura::KUSD_KEY.to_vec() {
						Some(Stable(KUSD))
					} else {
						None
					}
				},
				X2(Parachain(id), GeneralIndex(key)) if id == parachains::Statemine::ID => {
					if key == parachains::Statemine::RMRK_ID as u128 {
						Some(Token(RMRK))
					} else {
						None
					}
				},
				X3(Parachain(id), PalletInstance(index), GeneralIndex(key))
					if (id == parachains::Statemine::ID &&
						index == parachains::Statemine::PALLET_ID) =>
					if key == parachains::Statemine::RMRK_ID as u128 {
						Some(Token(RMRK))
					} else {
						None
					},
				X1(Parachain(id)) if id == parachains::phala::ID => Some(Token(PHA)),
				X2(Parachain(id), PalletInstance(index))
					if ((id == parachains::moonriver::ID) &&
						(index == parachains::moonriver::PALLET_ID)) =>
					Some(Token(MOVR)),
				_ => None,
			},
			MultiLocation { parents, interior } if parents == 0 => match interior {
				X1(GeneralKey(key)) => {
					// decode the general key
					if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
						match currency_id {
							Native(ASG) | Native(BNC) | VSToken(KSM) | Token(ZLK) =>
								Some(currency_id),
							_ => None,
						}
					} else {
						None
					}
				},
				_ => None,
			},
			_ => None,
		}
	}
}
