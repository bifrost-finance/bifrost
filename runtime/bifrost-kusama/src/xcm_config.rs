// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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

use assets_common::matching::{FromSiblingParachain, IsForeignConcreteAsset};
use super::*;
use bifrost_asset_registry::{AssetIdMaps, FixedRateOfAsset};
use bifrost_primitives::{AccountId, CurrencyId, CurrencyIdMapping, TokenSymbol};
pub use bifrost_xcm_interface::traits::{parachains, XcmBaseWeight};
pub use cumulus_primitives_core::ParaId;
use frame_support::{
	ensure, parameter_types,
	sp_runtime::traits::{CheckedConversion, Convert},
	traits::Get,
};
use parity_scale_codec::{Decode, Encode};
pub use polkadot_parachain_primitives::primitives::Sibling;
use sp_std::{convert::TryFrom, marker::PhantomData};
pub use xcm_builder::{
	AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, DescribeAllTerminal, DescribeFamily, EnsureXcmOrigin,
	FixedRateOfFungible, FixedWeightBounds, HashedDescription, IsConcrete, ParentAsSuperuser,
	ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeRevenue,
	TakeWeightCredit,
};
use xcm_executor::traits::{MatchesFungible, ShouldExecute};

// orml imports
use bifrost_currencies::BasicCurrencyAdapter;
use bifrost_runtime_common::currency_adapter::{
	BifrostDropAssets, DepositToAlternative, MultiCurrencyAdapter,
};
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId as CumulusParaId};
use frame_support::traits::{ContainsPair, Equals, ProcessMessageError, TransformOrigin};
use orml_traits::{
	currency::MutationHooks,
	location::{RelativeReserveProvider, Reserve},
};
pub use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key, MultiCurrency};
use pallet_xcm::{EnsureXcm, XcmPassthrough};
use parachains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};
use parachains_common::xcm_config::{AllSiblingSystemParachains, ConcreteAssetFromSystem, RelayOrOtherSystemParachains};
use polkadot_runtime_common::xcm_sender::{ExponentialPrice, NoPriceForMessageDelivery};
use sp_core::bounded::BoundedVec;
use xcm::v4::{prelude::*, Location};
use xcm_builder::{FrameTransactionalProcessor, FungibleAdapter, GlobalConsensusParachainConvertsFor, MintLocation, TrailingSetTopicAsId, WithUniqueTopic, XcmFeeManagerFromComponents, XcmFeeToAccount};
use xcm_executor::traits::Properties;

/// Bifrost Asset Matcher
pub struct BifrostAssetMatcher<CurrencyId, CurrencyIdConvert>(
	PhantomData<(CurrencyId, CurrencyIdConvert)>,
);

impl<CurrencyId, CurrencyIdConvert, Amount> MatchesFungible<Amount>
	for BifrostAssetMatcher<CurrencyId, CurrencyIdConvert>
where
	CurrencyIdConvert: Convert<Location, Option<CurrencyId>>,
	Amount: TryFrom<u128>,
{
	fn matches_fungible(a: &Asset) -> Option<Amount> {
		if let (Fungible(ref amount), AssetId(ref location)) = (&a.fun, &a.id) {
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
impl<ReserveProvider> ContainsPair<Asset, Location> for MultiNativeAsset<ReserveProvider>
where
	ReserveProvider: Reserve,
{
	fn contains(asset: &Asset, origin: &Location) -> bool {
		if let Some(ref reserve) = ReserveProvider::reserve(asset) {
			if reserve == origin {
				return true;
			}
		}
		false
	}
}

fn native_currency_location(id: CurrencyId) -> Location {
	Location::new(0, [Junction::from(BoundedVec::try_from(id.encode()).unwrap())])
}

impl<T: Get<ParaId>> Convert<Asset, Option<CurrencyId>> for BifrostCurrencyIdConvert<T> {
	fn convert(asset: Asset) -> Option<CurrencyId> {
		if let Asset { id: AssetId(id), fun: xcm::v4::Fungibility::Fungible(_) } = asset {
			Self::convert(id)
		} else {
			None
		}
	}
}

pub struct BifrostAccountIdToLocation;
impl Convert<AccountId, Location> for BifrostAccountIdToLocation {
	fn convert(account: AccountId) -> Location {
		[AccountId32 { network: None, id: account.into() }].into()
	}
}

pub struct BifrostCurrencyIdConvert<T>(sp_std::marker::PhantomData<T>);
impl<T: Get<ParaId>> Convert<CurrencyId, Option<Location>> for BifrostCurrencyIdConvert<T> {
	fn convert(id: CurrencyId) -> Option<Location> {
		use CurrencyId::*;
		use TokenSymbol::*;

		if let Some(id) = AssetIdMaps::<Runtime>::get_location(id) {
			return Some(id);
		}

		match id {
			Token(KSM) => Some(Location::parent()),
			Native(ASG) | Native(BNC) | VSToken(KSM) | Token(ZLK) =>
				Some(native_currency_location(id)),
			// Karura currencyId types
			Token(KAR) => Some(Location::new(
				1,
				[
					Parachain(parachains::karura::ID),
					Junction::from(
						BoundedVec::try_from(parachains::karura::KAR_KEY.to_vec()).unwrap(),
					),
				],
			)),
			Stable(KUSD) => Some(Location::new(
				1,
				[
					Parachain(parachains::karura::ID),
					Junction::from(
						BoundedVec::try_from(parachains::karura::KUSD_KEY.to_vec()).unwrap(),
					),
				],
			)),
			Token(RMRK) => Some(Location::new(
				1,
				[
					Parachain(parachains::Statemine::ID),
					PalletInstance(parachains::Statemine::PALLET_ID),
					GeneralIndex(parachains::Statemine::RMRK_ID as u128),
				],
			)),
			// Phala Native token
			Token(PHA) => Some(Location::new(1, [Parachain(parachains::phala::ID)])),
			// Moonriver Native token
			Token(MOVR) => Some(Location::new(
				1,
				[
					Parachain(parachains::moonriver::ID),
					PalletInstance(parachains::moonriver::PALLET_ID.into()),
				],
			)),
			_ => None,
		}
	}
}

impl<T: Get<ParaId>> Convert<Location, Option<CurrencyId>> for BifrostCurrencyIdConvert<T> {
	fn convert(location: Location) -> Option<CurrencyId> {
		use CurrencyId::*;
		use TokenSymbol::*;

		if location == Location::parent() {
			return Some(Token(KSM));
		}

		if let Some(currency_id) = AssetIdMaps::<Runtime>::get_currency_id(location.clone()) {
			return Some(currency_id);
		}

		match location.unpack() {
			(1, [Parachain(id), GeneralKey { data, length }]) if *id == parachains::karura::ID =>
				if data[..*length as usize] == parachains::karura::KAR_KEY.to_vec() {
					Some(Token(KAR))
				} else if data[..*length as usize] == parachains::karura::KUSD_KEY.to_vec() {
					Some(Stable(KUSD))
				} else {
					None
				},
			(1, [Parachain(id), GeneralIndex(key)]) if *id == parachains::Statemine::ID => {
				if *key == parachains::Statemine::RMRK_ID as u128 {
					Some(Token(RMRK))
				} else {
					None
				}
			},
			(1, [Parachain(id), PalletInstance(index), GeneralIndex(key)])
				if (*id == parachains::Statemine::ID &&
					*index == parachains::Statemine::PALLET_ID) =>
			{
				if *key == parachains::Statemine::RMRK_ID as u128 {
					Some(Token(RMRK))
				} else {
					None
				}
			},
			(1, [Parachain(id)]) if *id == parachains::phala::ID => Some(Token(PHA)),
			(1, [Parachain(id), PalletInstance(index)])
				if (*id == parachains::moonriver::ID) &&
					(*index == parachains::moonriver::PALLET_ID) =>
				Some(Token(MOVR)),
			(0, [GeneralKey { data, length }]) => {
				// decode the general key
				let key = &data[..*length as usize];
				if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
					match currency_id {
						Native(ASG) | Native(BNC) | VToken(KSM) | VSToken(KSM) | Token(ZLK) =>
							Some(currency_id),
						_ => None,
					}
				} else {
					None
				}
			},
			_ => None,
		}
	}
}

parameter_types! {
	pub const KsmLocation: Location = Location::parent();
	pub const RelayNetwork: NetworkId = Westend;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub SelfParaChainId: CumulusParaId = ParachainInfo::parachain_id();
		pub UniversalLocation: InteriorLocation =
		[GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into())].into();
	pub CheckAccount: AccountId = PolkadotXcm::check_account();
	pub LocalCheckAccount: (AccountId, MintLocation) = (CheckAccount::get(), MintLocation::Local);
	pub BncLocation: Location = Location::new(0, [Junction::from(BoundedVec::try_from(NativeCurrencyId::get().encode()).unwrap())]);
}

/// Type for specifying how a `Location` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch RuntimeOrigin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
	// Foreign locations alias into accounts according to a hash of their standard description.
	HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
	// Different global consensus parachain sovereign account.
	// (Used for over-bridge transfers and reserve processing)
	GlobalConsensusParachainConvertsFor<UniversalLocation, AccountId>,
);

/// This is the type we use to convert an (incoming) XCM origin into a local `RuntimeOrigin`
/// instance, ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind`
/// which can biases the kind of local `RuntimeOrigin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	// Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
	// transaction from the Root origin.
	ParentAsSuperuser<RuntimeOrigin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `RuntimeOrigin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	// One XCM operation is 200_000_000 weight, cross-chain transfer ~= 2x of transfer = 3_000_000_000
	pub UnitWeightCost: Weight = Weight::from_parts(200_000_000, 0);
	pub const MaxInstructions: u32 = 100;
}

/// Barrier allowing a top level paid message with DescendOrigin instruction
pub const DEFAULT_PROOF_SIZE: u64 = 64 * 1024;
pub const DEFAULT_REF_TIMR: u64 = 10_000_000_000;
pub struct AllowTopLevelPaidExecutionDescendOriginFirst<T>(PhantomData<T>);
impl<T: Contains<Location>> ShouldExecute for AllowTopLevelPaidExecutionDescendOriginFirst<T> {
	fn should_execute<Call>(
		origin: &Location,
		message: &mut [Instruction<Call>],
		max_weight: Weight,
		_weight_credit: &mut Properties,
	) -> Result<(), ProcessMessageError> {
		log::trace!(
			target: "xcm::barriers",
			"AllowTopLevelPaidExecutionDescendOriginFirst origin:
			{:?}, message: {:?}, max_weight: {:?}, weight_credit: {:?}",
			origin, message, max_weight, _weight_credit,
		);
		ensure!(T::contains(origin), ProcessMessageError::Unsupported);
		let mut iter = message.iter_mut();
		// Make sure the first instruction is DescendOrigin
		iter.next()
			.filter(|instruction| matches!(instruction, DescendOrigin(_)))
			.ok_or(ProcessMessageError::Unsupported)?;

		// Then WithdrawAsset
		iter.next()
			.filter(|instruction| matches!(instruction, WithdrawAsset(_)))
			.ok_or(ProcessMessageError::Unsupported)?;

		// Then BuyExecution
		let i = iter.next().ok_or(ProcessMessageError::Unsupported)?;
		match i {
			BuyExecution { weight_limit: Limited(ref mut weight), .. } => {
				if weight.all_gte(max_weight) {
					weight.set_ref_time(max_weight.ref_time());
					weight.set_proof_size(max_weight.proof_size());
				};
			},
			BuyExecution { ref mut weight_limit, .. } if weight_limit == &Unlimited => {
				*weight_limit = Limited(max_weight);
			},
			_ => {},
		};

		// Then Transact
		let i = iter.next().ok_or(ProcessMessageError::Unsupported)?;
		match i {
			Transact { ref mut require_weight_at_most, .. } => {
				let weight = Weight::from_parts(DEFAULT_REF_TIMR, DEFAULT_PROOF_SIZE);
				*require_weight_at_most = weight;
				Ok(())
			},
			_ => Err(ProcessMessageError::Unsupported),
		}
	}
}

pub type Barrier = TrailingSetTopicAsId<(
	// Weight that is paid for may be consumed.
	TakeWeightCredit,
	// Expected responses are OK.
	AllowKnownQueryResponses<PolkadotXcm>,
	// If the message is one that immediately attemps to pay for execution, then allow it.
	AllowTopLevelPaidExecutionFrom<Everything>,
	// Subscriptions for version tracking are OK.
	AllowSubscriptionsFrom<Everything>,
	// Barrier allowing a top level paid message with DescendOrigin instruction
	AllowTopLevelPaidExecutionDescendOriginFirst<Everything>,
)>;

/// Our asset transactor. This is what allows us to interest with the runtime facilities from the
/// point of view of XCM-only concepts like `Location` and `Asset`.
///
/// Ours is only aware of the Balances pallet, which is mapped to `RocLocation`.
pub type LocalAssetTransactor = FungibleAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<BncLocation>,
	// We can convert the Locations with our converter above:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We track our teleports in/out to keep total issuance correct.
	(),
>;

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
	pub KsmPerSecond: (AssetId, u128, u128) = (Location::parent().into(), ksm_per_second::<Runtime>(),0);
	pub VksmPerSecond: (AssetId, u128,u128) = (
		Location::new(
			0,
			[Junction::from(BoundedVec::try_from(CurrencyId::VToken(TokenSymbol::KSM).encode()).unwrap())],
		).into(),
		ksm_per_second::<Runtime>(),
		0
	);
	pub VsksmPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(SelfParaId::get()), Junction::from(BoundedVec::try_from(CurrencyId::VSToken(TokenSymbol::KSM).encode()).unwrap())]
		).into(),
		ksm_per_second::<Runtime>(),
		0
	);
	pub VsksmNewPerSecond: (AssetId, u128,u128) = (
		Location::new(
			0,
			[Junction::from(BoundedVec::try_from(CurrencyId::VSToken(TokenSymbol::KSM).encode()).unwrap())]
		).into(),
		ksm_per_second::<Runtime>(),
		0
	);
	pub BncPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(SelfParaId::get()), Junction::from(BoundedVec::try_from(NativeCurrencyId::get().encode()).unwrap())]
		).into(),
		// BNC:KSM = 80:1
		ksm_per_second::<Runtime>() * 80,
		0
	);
	pub BncNewPerSecond: (AssetId, u128,u128) = (
		Location::new(
			0,
			[Junction::from(BoundedVec::try_from(NativeCurrencyId::get().encode()).unwrap())]
		).into(),
		// BNC:KSM = 80:1
		ksm_per_second::<Runtime>() * 80,
		0
	);

	pub ZlkPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(SelfParaId::get()), Junction::from(BoundedVec::try_from(CurrencyId::Token(TokenSymbol::ZLK).encode()).unwrap())]
		).into(),
		// ZLK:KSM = 150:1
		//ZLK has a decimal of 18, while KSM is 12.
		ksm_per_second::<Runtime>() * 150 * 1_000_000,
		0
	);
	pub ZlkNewPerSecond: (AssetId, u128,u128) = (
		Location::new(
			0,
			[Junction::from(BoundedVec::try_from(CurrencyId::Token(TokenSymbol::ZLK).encode()).unwrap())]
		).into(),
		// ZLK:KSM = 150:1
		//ZLK has a decimal of 18, while KSM is 12.
		ksm_per_second::<Runtime>() * 150 * 1_000_000,
		0
	);
	pub KarPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(parachains::karura::ID), Junction::from(BoundedVec::try_from(parachains::karura::KAR_KEY.to_vec()).unwrap())]
		).into(),
		// KAR:KSM = 100:1
		ksm_per_second::<Runtime>() * 100,
		0
	);
	pub KusdPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(parachains::karura::ID), Junction::from(BoundedVec::try_from(parachains::karura::KUSD_KEY.to_vec()).unwrap())]
		).into(),
		// kUSD:KSM = 400:1
		ksm_per_second::<Runtime>() * 400,
		0
	);
	pub PhaPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(parachains::phala::ID)],
		).into(),
		// PHA:KSM = 400:1
		ksm_per_second::<Runtime>() * 400,
		0
	);
	pub RmrkPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(parachains::Statemine::ID), GeneralIndex(parachains::Statemine::RMRK_ID.into())]
		).into(),
		// rmrk:KSM = 10:1
		ksm_per_second::<Runtime>() * 10 / 100, //rmrk currency decimal as 10
		0
	);
	pub RmrkNewPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(parachains::Statemine::ID), PalletInstance(parachains::Statemine::PALLET_ID),GeneralIndex(parachains::Statemine::RMRK_ID.into())]
		).into(),
		// rmrk:KSM = 10:1
		ksm_per_second::<Runtime>() * 10 / 100, //rmrk currency decimal as 10
		0
	);
	pub MovrPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(parachains::moonriver::ID), PalletInstance(parachains::moonriver::PALLET_ID.into())]
		).into(),
		// MOVR:KSM = 2.67:1
		ksm_per_second::<Runtime>() * 267 * 10_000, //movr currency decimal as 18
		0
	);
	pub BasePerSecond: u128 = ksm_per_second::<Runtime>();
}

pub struct ToTreasury;
impl TakeRevenue for ToTreasury {
	fn take_revenue(revenue: Asset) {
		if let Asset { id: AssetId(location), fun: xcm::v4::Fungibility::Fungible(amount) } =
			revenue
		{
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
	FixedRateOfFungible<VksmPerSecond, ToTreasury>,
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

/// A call filter for the XCM Transact instruction. This is a temporary measure until we properly
/// account for proof size weights.
///
/// Calls that are allowed through this filter must:
/// 1. Have a fixed weight;
/// 2. Cannot lead to another call being made;
/// 3. Have a defined proof size weight, e.g. no unbounded vecs in call parameters.
pub struct SafeCallFilter;
impl Contains<RuntimeCall> for SafeCallFilter {
	fn contains(call: &RuntimeCall) -> bool {
		#[cfg(feature = "runtime-benchmarks")]
		{
			if matches!(call, RuntimeCall::System(frame_system::Call::remark_with_event { .. })) {
				return true;
			}
		}

		match call {
			RuntimeCall::System(
				frame_system::Call::kill_prefix { .. } | frame_system::Call::set_heap_pages { .. },
			) |
			RuntimeCall::Timestamp(..) |
			RuntimeCall::Indices(..) |
			RuntimeCall::Balances(..) |
			RuntimeCall::ParachainStaking(..) |
			RuntimeCall::Session(pallet_session::Call::purge_keys { .. }) |
			RuntimeCall::Treasury(..) |
			RuntimeCall::Utility(pallet_utility::Call::as_derivative { .. }) |
			RuntimeCall::Identity(
				pallet_identity::Call::add_registrar { .. } |
				pallet_identity::Call::set_identity { .. } |
				pallet_identity::Call::clear_identity { .. } |
				pallet_identity::Call::request_judgement { .. } |
				pallet_identity::Call::cancel_request { .. } |
				pallet_identity::Call::set_fee { .. } |
				pallet_identity::Call::set_account_id { .. } |
				pallet_identity::Call::set_fields { .. } |
				pallet_identity::Call::provide_judgement { .. } |
				pallet_identity::Call::kill_identity { .. } |
				pallet_identity::Call::add_sub { .. } |
				pallet_identity::Call::rename_sub { .. } |
				pallet_identity::Call::remove_sub { .. } |
				pallet_identity::Call::quit_sub { .. },
			) |
			RuntimeCall::Vesting(..) |
			RuntimeCall::PolkadotXcm(pallet_xcm::Call::limited_reserve_transfer_assets { .. }) |
			RuntimeCall::Proxy(..) |
			RuntimeCall::Tokens(
				orml_tokens::Call::transfer { .. } |
				orml_tokens::Call::transfer_all { .. } |
				orml_tokens::Call::transfer_keep_alive { .. }
			) |
			// Bifrost moudule
			RuntimeCall::Farming(
				bifrost_farming::Call::claim { .. } |
				bifrost_farming::Call::deposit { .. } |
				bifrost_farming::Call::withdraw { .. } |
				bifrost_farming::Call::withdraw_claim { .. }
			) |
			RuntimeCall::Salp(
				bifrost_salp::Call::contribute { .. } |
				bifrost_salp::Call::batch_unlock { .. } |
				bifrost_salp::Call::redeem { .. } |
				bifrost_salp::Call::unlock { .. } |
				bifrost_salp::Call::unlock_by_vsbond { .. } |
				bifrost_salp::Call::unlock_vstoken { .. }
			) |
			RuntimeCall::VSBondAuction(
				bifrost_vsbond_auction::Call::clinch_order { .. } |
				bifrost_vsbond_auction::Call::create_order { .. } |
				bifrost_vsbond_auction::Call::partial_clinch_order { .. } |
				bifrost_vsbond_auction::Call::revoke_order { .. }
			) |
			RuntimeCall::VstokenConversion(
				bifrost_vstoken_conversion::Call::vsbond_convert_to_vstoken { .. } |
				bifrost_vstoken_conversion::Call::vstoken_convert_to_vsbond { .. }
			) |
			RuntimeCall::VtokenMinting(
				bifrost_vtoken_minting::Call::mint { .. } |
				bifrost_vtoken_minting::Call::rebond { .. } |
				bifrost_vtoken_minting::Call::rebond_by_unlock_id { .. } |
				bifrost_vtoken_minting::Call::redeem { .. }
			) |
			RuntimeCall::XcmInterface(
				bifrost_xcm_interface::Call::transfer_statemine_assets { .. }
			) |
			RuntimeCall::Slpx(..) |
			RuntimeCall::ZenlinkProtocol(
				zenlink_protocol::Call::add_liquidity { .. } |
				zenlink_protocol::Call::remove_liquidity { .. } |
				zenlink_protocol::Call::transfer { .. }
			) |
			RuntimeCall::ZenlinkStableAMM(
				zenlink_stable_amm::Call::remove_liquidity_one_currency { .. } |
				zenlink_stable_amm::Call::remove_pool_and_base_pool_liquidity_one_currency { .. } |
				zenlink_stable_amm::Call::swap { .. } |
				zenlink_stable_amm::Call::swap_pool_to_base { .. } |
				zenlink_stable_amm::Call::swap_meta_pool_underlying { .. } |
				zenlink_stable_amm::Call::withdraw_admin_fee { .. }
			) => true,
			_ => false,
		}
	}
}

/// Asset filter that allows all assets from a certain location matching asset id.
pub struct AssetPrefixFrom<Prefix, Origin>(PhantomData<(Prefix, Origin)>);
impl<Prefix, Origin> ContainsPair<Asset, Location> for AssetPrefixFrom<Prefix, Origin>
where
	Prefix: Get<Location>,
	Origin: Get<Location>,
{
	fn contains(asset: &Asset, origin: &Location) -> bool {
		let loc = Origin::get();
		&loc == origin &&
			matches!(asset, Asset { id: AssetId(asset_loc), fun: Fungible(_a) }
			if asset_loc.starts_with(&Prefix::get()))
	}
}

/// Asset filter that allows native/relay asset if coming from a certain location.
pub struct NativeAssetFrom<T>(PhantomData<T>);
impl<T: Get<Location>> ContainsPair<Asset, Location> for NativeAssetFrom<T> {
	fn contains(asset: &Asset, origin: &Location) -> bool {
		let loc = T::get();
		&loc == origin &&
			matches!(asset, Asset { id: AssetId(asset_loc), fun: Fungible(_a) }
			if *asset_loc == Location::from(Parent))
	}
}

parameter_types! {
  	/// Location of Asset Hub
  	pub AssetHubLocation: Location = (Parent, Parachain(1000)).into();
	pub EthereumLocation: Location = Location::new(2, [GlobalConsensus(Ethereum { chain_id: 1 })]);
}

/// Cases where a remote origin is accepted as trusted Teleporter for a given asset:
///
/// - WND with the parent Relay Chain and sibling system parachains; and
/// - Sibling parachains' assets from where they originate (as `ForeignCreators`).
pub type TrustedTeleporters = (
	ConcreteAssetFromSystem<WestendLocation>,
	IsForeignConcreteAsset<FromSiblingParachain<parachain_info::Pallet<Runtime>>>,
);

pub type WaivedLocations = (
	RelayOrOtherSystemParachains<AllSiblingSystemParachains, Runtime>,
	// Equals<RelayTreasuryLocation>,
	// FellowshipEntities,
	// AmbassadorEntities,
);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type AssetClaims = PolkadotXcm;
	type AssetTransactor = (LocalAssetTransactor, BifrostAssetTransactor);
	type AssetTrap = BifrostDropAssets<ToTreasury>;
	type Barrier = Barrier;
	type RuntimeCall = RuntimeCall;
	type IsReserve = Everything;
	type IsTeleporter = Everything;
	type UniversalLocation = UniversalLocation;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type ResponseHandler = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type Trader = Trader;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmSender = XcmRouter;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = ConstU32<8>;
	type UniversalAliases = bridging::to_rococo::UniversalAliases;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = SafeCallFilter;
	type AssetLocker = ();
	type AssetExchanger = ();
	type FeeManager = ();
	type MessageExporter = ();
	type Aliasers = Nothing;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type XcmRecorder = PolkadotXcm;
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
}

/// Local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// For routing XCM messages which do not cross local consensus boundary.
type LocalXcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = WithUniqueTopic<(
	LocalXcmRouter,
	// Router which wraps and sends xcm to BridgeHub to be delivered to the Polkadot
	// GlobalConsensus
	ToPolkadotXcmRouter,
)>;

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<Location> = Some(Parent.into());
}

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type UniversalLocation = UniversalLocation;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmExecuteFilter = Everything;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmExecutor = bifrost_primitives::DoNothingExecuteXcm;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmReserveTransferFilter = Everything;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmRouter = bifrost_primitives::DoNothingRouter;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmRouter = XcmRouter;
	type XcmTeleportFilter = Everything;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = ();
	type MaxLockers = ConstU32<8>;
	type WeightInfo = weights::pallet_xcm::WeightInfo<Runtime>;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

parameter_types! {
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
		/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: AssetId = AssetId(WestendLocation::get());
	/// The base fee for the message delivery fees.
	pub const BaseDeliveryFee: u128 = 10_000_000_000u128.saturating_mul(3);
}

pub type PriceForSiblingParachainDelivery = ExponentialPrice<
	FeeAssetId,
	BaseDeliveryFee,
	TransactionByteFee,
	XcmpQueue,
>;

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type ChannelInfo = ParachainSystem;
	type RuntimeEvent = RuntimeEvent;
	type VersionWrapper = PolkadotXcm;
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
	type MaxInboundSuspended = ConstU32<1_000>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = cumulus_pallet_xcmp_queue::weights::SubstrateWeight<Runtime>;
	type PriceForSiblingDelivery = PriceForSiblingParachainDelivery;
	type MaxActiveOutboundChannels = ConstU32<128>;
	type MaxPageSize = ConstU32<{ 103 * 1024 }>;
}

parameter_types! {
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
	pub MessageQueueIdleServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
}

impl pallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_message_queue::weights::SubstrateWeight<Self>;
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor =
		pallet_message_queue::mock_helpers::NoopMessageProcessor<AggregateMessageOrigin>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		xcm_executor::XcmExecutor<XcmConfig>,
		RuntimeCall,
	>;
	type Size = u32;
	type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
	type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
	type HeapSize = ConstU32<{ 64 * 1024 }>;
	type MaxStale = ConstU32<8>;
	type ServiceWeight = MessageQueueServiceWeight;
	type IdleMaxServiceWeight = MessageQueueIdleServiceWeight;
}

// orml runtime start

impl bifrost_currencies::Config for Runtime {
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
	type WeightInfo = weights::bifrost_currencies::WeightInfo<Runtime>;
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
			&CurrencyId::LPToken(..) => 1 * micro::<Runtime>(NativeCurrencyId::get()),
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
		.eq(a) || FeeSharePalletId::get().check_sub_account::<DistributionId>(a) ||
			a.eq(&ZenklinkFeeAccount::get()) ||
			AccountIdConversion::<AccountId>::into_account_truncating(&CommissionPalletId::get())
				.eq(a)
	}
}

parameter_types! {
	pub BifrostTreasuryAccount: AccountId = TreasuryPalletId::get().into_account_truncating();
	// gVLo8SqxQsm11cXpkFJnaqXhAd6qtxwi2DhxfUFE7pSiyoi
	pub ZenklinkFeeAccount: AccountId = hex!["d2ca9ceb400cc68dcf58de4871bd261406958fd17338d2d82ad2592db62e6a2a"].into();
}

pub struct CurrencyHooks;
impl MutationHooks<AccountId, CurrencyId, Balance> for CurrencyHooks {
	type OnDust = orml_tokens::TransferDust<Runtime, BifrostTreasuryAccount>;
	type OnSlash = ();
	type PreDeposit = ();
	type PostDeposit = ();
	type PreTransfer = ();
	type PostTransfer = ();
	type OnNewTokenAccount = ();
	type OnKilledTokenAccount = ();
}

impl orml_tokens::Config for Runtime {
	type Amount = Amount;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type DustRemovalWhitelist = DustRemovalWhitelist;
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = weights::orml_tokens::WeightInfo<Runtime>;
	type CurrencyHooks = CurrencyHooks;
}

parameter_types! {
	pub SelfLocation: Location = Location::new(1, [Parachain(ParachainInfo::get().into())]);
	pub SelfRelativeLocation: Location = Location::here();
	pub const BaseXcmWeight: Weight = Weight::from_parts(1000_000_000u64, 0);
	pub const MaxAssetsForTransfer: usize = 2;
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: Location| -> Option<u128> {
		Some(u128::MAX)
	};
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = BifrostCurrencyIdConvert<ParachainInfo>;
	type AccountIdToLocation = BifrostAccountIdToLocation;
	type UniversalLocation = UniversalLocation;
	type SelfLocation = SelfRelativeLocation;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmExecutor = bifrost_primitives::DoNothingExecuteXcm;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type MinXcmFee = ParachainMinFee;
	type LocationsFilter = Everything;
	type ReserveProvider = RelativeReserveProvider;
	type RateLimiter = ();
	type RateLimiterId = ();
}

impl orml_unknown_tokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}

impl orml_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SovereignOrigin = MoreThanHalfCouncil;
}

parameter_types! {
	pub ParachainAccount: AccountId = ParachainInfo::get().into_account_truncating();
	pub const WestendLocation: Location = Location::parent();
}

impl bifrost_xcm_interface::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type UpdateOrigin = TechAdminOrCouncil;
	type MultiCurrency = Currencies;
	type RelayNetwork = RelayNetwork;
	type RelaychainCurrencyId = RelayCurrencyId;
	type ParachainSovereignAccount = ParachainAccount;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmExecutor = bifrost_primitives::DoNothingExecuteXcm;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type AccountIdToLocation = BifrostAccountIdToLocation;
	type SalpHelper = Salp;
	type ParachainId = SelfParaChainId;
	type CallBackTimeOut = ConstU32<10>;
	type CurrencyIdConvert = AssetIdMaps<Runtime>;
}

/// XCM router instance to BridgeHub with bridging capabilities for `Rococo` global
/// consensus with dynamic fees and back-pressure.
pub type ToPolkadotXcmRouterInstance = pallet_xcm_bridge_hub_router::Instance1;
impl pallet_xcm_bridge_hub_router::Config<ToPolkadotXcmRouterInstance> for Runtime {
	type WeightInfo = weights::pallet_xcm_bridge_hub_router::WeightInfo<Runtime>;

	type UniversalLocation = xcm_config::UniversalLocation;
	type BridgedNetworkId = xcm_config::bridging::to_rococo::RococoNetwork;
	type Bridges = xcm_config::bridging::NetworkExportTable;
	type DestinationVersion = PolkadotXcm;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type BridgeHubOrigin = EnsureXcm<Equals<xcm_config::bridging::SiblingBridgeHub>>;
	#[cfg(feature = "runtime-benchmarks")]
	type BridgeHubOrigin = frame_support::traits::EitherOfDiverse<
		// for running benchmarks
		EnsureRoot<AccountId>,
		// for running tests with `--feature runtime-benchmarks`
		EnsureXcm<Equals<xcm_config::bridging::SiblingBridgeHub>>,
	>;

	type ToBridgeHubSender = XcmpQueue;
	type WithBridgeHubChannel =
	cumulus_pallet_xcmp_queue::bridging::InAndOutXcmpChannelStatusProvider<
		xcm_config::bridging::SiblingBridgeHubParaId,
		Runtime,
	>;

	type ByteFee = xcm_config::bridging::XcmBridgeHubRouterByteFee;
	type FeeAsset = xcm_config::bridging::XcmBridgeHubRouterFeeAssetId;
}

/// All configuration related to bridging
pub mod bridging {
	use super::*;
	use assets_common::matching;
	use sp_std::collections::btree_set::BTreeSet;
	use xcm_builder::NetworkExportTableItem;

	parameter_types! {
		/// Base price of every byte of the Westend -> Rococo message. Can be adjusted via
		/// governance `set_storage` call.
		///
		/// Default value is our estimation of the:
		///
		/// 1) an approximate cost of XCM execution (`ExportMessage` and surroundings) at Westend bridge hub;
		///
		/// 2) the approximate cost of Westend -> Rococo message delivery transaction on Rococo Bridge Hub,
		///    converted into WNDs using 1:1 conversion rate;
		///
		/// 3) the approximate cost of Westend -> Rococo message confirmation transaction on Westend Bridge Hub.
		pub storage XcmBridgeHubRouterBaseFee: Balance =
			bp_bridge_hub_westend::BridgeHubWestendBaseXcmFeeInWnds::get()
				.saturating_add(bp_bridge_hub_rococo::BridgeHubRococoBaseDeliveryFeeInRocs::get())
				.saturating_add(bp_bridge_hub_westend::BridgeHubWestendBaseConfirmationFeeInWnds::get());
		/// Price of every byte of the Westend -> Rococo message. Can be adjusted via
		/// governance `set_storage` call.
		pub storage XcmBridgeHubRouterByteFee: Balance = TransactionByteFee::get();

		pub SiblingBridgeHubParaId: u32 = bp_bridge_hub_westend::BRIDGE_HUB_WESTEND_PARACHAIN_ID;
		pub SiblingBridgeHub: Location = Location::new(1, [Parachain(SiblingBridgeHubParaId::get())]);
		/// Router expects payment with this `AssetId`.
		/// (`AssetId` has to be aligned with `BridgeTable`)
		pub XcmBridgeHubRouterFeeAssetId: AssetId = WestendLocation::get().into();

		pub BridgeTable: sp_std::vec::Vec<NetworkExportTableItem> =
			sp_std::vec::Vec::new().into_iter()
			.chain(to_rococo::BridgeTable::get())
			.collect();
	}

	pub type NetworkExportTable = xcm_builder::NetworkExportTable<BridgeTable>;

	pub mod to_rococo {
		use bifrost_primitives::BNC;
		use super::*;

		parameter_types! {
			pub SiblingBridgeHubWithBridgeHubRococoInstance: Location = Location::new(
				1,
				[
					Parachain(SiblingBridgeHubParaId::get()),
					PalletInstance(bp_bridge_hub_westend::WITH_BRIDGE_WESTEND_TO_ROCOCO_MESSAGES_PALLET_INDEX)
				]
			);

			pub const RococoNetwork: NetworkId = NetworkId::Rococo;
			pub BifrostPolkadot: Location = Location::new(2, [GlobalConsensus(RococoNetwork::get()), Parachain(2030)]);
			pub RocLocation: Location = Location::new(2, [GlobalConsensus(RococoNetwork::get())]);

			pub BncLocation: Location = Location::new(2, [GlobalConsensus(RococoNetwork::get()), Parachain(2030), Junction::from(BoundedVec::try_from(BNC.encode()).unwrap())]);
			pub BncFromBifrostPolkadot: (AssetFilter, Location) = (
				Wild(AllOf { fun: WildFungible, id: AssetId(BncLocation::get()) }),
				BifrostPolkadot::get()
			);
			pub RocFromBifrostPolkadot: (AssetFilter, Location) = (
				Wild(AllOf { fun: WildFungible, id: AssetId(RocLocation::get()) }),
				BifrostPolkadot::get()
			);

			/// Set up exporters configuration.
			/// `Option<Asset>` represents static "base fee" which is used for total delivery fee calculation.
			pub BridgeTable: sp_std::vec::Vec<NetworkExportTableItem> = sp_std::vec![
				NetworkExportTableItem::new(
					RococoNetwork::get(),
					Some(sp_std::vec![
						BifrostPolkadot::get().interior.split_global().expect("invalid configuration for BifrostPolkadot").1,
					]),
					SiblingBridgeHub::get(),
					// base delivery fee to local `BridgeHub`
					Some((
						XcmBridgeHubRouterFeeAssetId::get(),
						XcmBridgeHubRouterBaseFee::get(),
					).into())
				)
			];

			/// Universal aliases
			pub UniversalAliases: BTreeSet<(Location, Junction)> = BTreeSet::from_iter(
				sp_std::vec![
					(SiblingBridgeHubWithBridgeHubRococoInstance::get(), GlobalConsensus(RococoNetwork::get()))
				]
			);
		}

		impl Contains<(Location, Junction)> for UniversalAliases {
			fn contains(alias: &(Location, Junction)) -> bool {
				UniversalAliases::get().contains(alias)
			}
		}

		/// Reserve locations filter for `xcm_executor::Config::IsReserve`.
		/// Locations from which the runtime accepts reserved assets.
		pub type IsTrustedBridgedReserveLocationForConcreteAsset =
		matching::IsTrustedBridgedReserveLocationForConcreteAsset<
			UniversalLocation,
			(
				// allow receive ROC from BifrostPolkadot
				xcm_builder::Case<BncFromBifrostPolkadot>,
				xcm_builder::Case<RocFromBifrostPolkadot>,
				// and nothing else
			),
		>;

		impl Contains<RuntimeCall> for ToPolkadotXcmRouter {
			fn contains(call: &RuntimeCall) -> bool {
				matches!(
					call,
					RuntimeCall::ToPolkadotXcmRouter(
						pallet_xcm_bridge_hub_router::Call::report_bridge_status { .. }
					)
				)
			}
		}
	}

	/// Benchmarks helper for bridging configuration.
	#[cfg(feature = "runtime-benchmarks")]
	pub struct BridgingBenchmarksHelper;

	#[cfg(feature = "runtime-benchmarks")]
	impl BridgingBenchmarksHelper {
		pub fn prepare_universal_alias() -> Option<(Location, Junction)> {
			let alias =
				to_rococo::UniversalAliases::get().into_iter().find_map(|(location, junction)| {
					match to_rococo::SiblingBridgeHubWithBridgeHubRococoInstance::get()
						.eq(&location)
					{
						true => Some((location, junction)),
						false => None,
					}
				});
			assert!(alias.is_some(), "we expect here BridgeHubWestend to Rococo mapping at least");
			Some(alias.unwrap())
		}
	}
}
