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
	AllowTopLevelPaidExecutionFrom, CurrencyAdapter, EnsureXcmOrigin, FixedRateOfFungible,
	FixedWeightBounds, IsConcrete, ParentAsSuperuser, ParentIsPreset, RelayChainAsNative,
	SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
	SignedToAccountId32, SovereignSignedViaLocation, TakeRevenue, TakeWeightCredit,
};
use xcm_executor::traits::{MatchesFungible, ShouldExecute};

// orml imports
use bifrost_currencies::BasicCurrencyAdapter;
use bifrost_runtime_common::currency_adapter::{
	BifrostDropAssets, DepositToAlternative, MultiCurrencyAdapter,
};
use cumulus_primitives_core::ParaId as CumulusParaId;
use frame_support::traits::{ContainsPair, ProcessMessageError};
use orml_traits::{
	currency::MutationHooks,
	location::{RelativeReserveProvider, Reserve},
};
pub use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key, MultiCurrency};
use pallet_xcm::XcmPassthrough;
use polkadot_runtime_common::xcm_sender::NoPriceForMessageDelivery;
use sp_core::bounded::BoundedVec;
use xcm::v3::prelude::*;
use xcm_builder::{Account32Hash, FrameTransactionalProcessor, TrailingSetTopicAsId};
use xcm_executor::traits::Properties;

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
			if CurrencyIdConvert::convert(*location).is_some() {
				return CheckedConversion::checked_from(*amount);
			}
		}
		None
	}
}

/// A `FilterAssetLocation` implementation. Filters multi native assets whose
/// reserve is same with `origin`.
pub struct MultiNativeAsset<ReserveProvider>(PhantomData<ReserveProvider>);
impl<ReserveProvider> ContainsPair<MultiAsset, MultiLocation> for MultiNativeAsset<ReserveProvider>
where
	ReserveProvider: Reserve,
{
	fn contains(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		if let Some(ref reserve) = ReserveProvider::reserve(asset) {
			if reserve == origin {
				return true;
			}
		}
		false
	}
}

fn native_currency_location(id: CurrencyId) -> MultiLocation {
	MultiLocation::new(0, X1(Junction::from(BoundedVec::try_from(id.encode()).unwrap())))
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
		X1(AccountId32 { network: None, id: account.into() }).into()
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
				Some(native_currency_location(id)),
			// Karura currencyId types
			Token(KAR) => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::karura::ID),
					Junction::from(
						BoundedVec::try_from(parachains::karura::KAR_KEY.to_vec()).unwrap(),
					),
				),
			)),
			Stable(KUSD) => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::karura::ID),
					Junction::from(
						BoundedVec::try_from(parachains::karura::KUSD_KEY.to_vec()).unwrap(),
					),
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

		if let Some(currency_id) = AssetIdMaps::<Runtime>::get_currency_id(location) {
			return Some(currency_id);
		}

		match location {
			MultiLocation { parents, interior } if parents == 1 => match interior {
				X2(Parachain(id), GeneralKey { data, length }) if id == parachains::karura::ID =>
					if data[..length as usize] == parachains::karura::KAR_KEY.to_vec() {
						Some(Token(KAR))
					} else if data[..length as usize] == parachains::karura::KUSD_KEY.to_vec() {
						Some(Stable(KUSD))
					} else {
						None
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
				{
					if key == parachains::Statemine::RMRK_ID as u128 {
						Some(Token(RMRK))
					} else {
						None
					}
				},
				X1(Parachain(id)) if id == parachains::phala::ID => Some(Token(PHA)),
				X2(Parachain(id), PalletInstance(index))
					if ((id == parachains::moonriver::ID) &&
						(index == parachains::moonriver::PALLET_ID)) =>
					Some(Token(MOVR)),
				_ => None,
			},
			MultiLocation { parents, interior } if parents == 0 => match interior {
				X1(GeneralKey { data, length }) => {
					// decode the general key
					let key = &data[..length as usize];
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
			},
			_ => None,
		}
	}
}

parameter_types! {
	pub const KsmLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub SelfParaChainId: CumulusParaId = ParachainInfo::parachain_id();
	pub UniversalLocation: InteriorMultiLocation = X2(GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into()));
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch RuntimeOrigin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
	// TODO: Generate remote accounts according to polkadot standards
	// Derives a private `Account32` by hashing `("multiloc", received multilocation)`
	Account32Hash<RelayNetwork, AccountId>,
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

match_types! {
	pub type ParentOrParentsExecutivePlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Executive, .. }) }
	};
}

/// Barrier allowing a top level paid message with DescendOrigin instruction
pub const DEFAULT_PROOF_SIZE: u64 = 64 * 1024;
pub const DEFAULT_REF_TIMR: u64 = 10_000_000_000;
pub struct AllowTopLevelPaidExecutionDescendOriginFirst<T>(PhantomData<T>);
impl<T: Contains<MultiLocation>> ShouldExecute for AllowTopLevelPaidExecutionDescendOriginFirst<T> {
	fn should_execute<Call>(
		origin: &MultiLocation,
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
	pub KsmPerSecond: (AssetId, u128, u128) = (MultiLocation::parent().into(), ksm_per_second::<Runtime>(),0);
	pub VksmPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			0,
			X1(Junction::from(BoundedVec::try_from(CurrencyId::VToken(TokenSymbol::KSM).encode()).unwrap())),
		).into(),
		ksm_per_second::<Runtime>(),
		0
	);
	pub VsksmPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(SelfParaId::get()), Junction::from(BoundedVec::try_from(CurrencyId::VSToken(TokenSymbol::KSM).encode()).unwrap()))

		).into(),
		ksm_per_second::<Runtime>(),
		0
	);
	pub VsksmNewPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			0,
			X1(Junction::from(BoundedVec::try_from(CurrencyId::VSToken(TokenSymbol::KSM).encode()).unwrap()))
		).into(),
		ksm_per_second::<Runtime>(),
		0
	);
	pub BncPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(SelfParaId::get()), Junction::from(BoundedVec::try_from(NativeCurrencyId::get().encode()).unwrap()))
		).into(),
		// BNC:KSM = 80:1
		ksm_per_second::<Runtime>() * 80,
		0
	);
	pub BncNewPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			0,
			X1(Junction::from(BoundedVec::try_from(NativeCurrencyId::get().encode()).unwrap()))
		).into(),
		// BNC:KSM = 80:1
		ksm_per_second::<Runtime>() * 80,
		0
	);

	pub ZlkPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(SelfParaId::get()), Junction::from(BoundedVec::try_from(CurrencyId::Token(TokenSymbol::ZLK).encode()).unwrap()))
		).into(),
		// ZLK:KSM = 150:1
		//ZLK has a decimal of 18, while KSM is 12.
		ksm_per_second::<Runtime>() * 150 * 1_000_000,
		0
	);
	pub ZlkNewPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			0,
			X1(Junction::from(BoundedVec::try_from(CurrencyId::Token(TokenSymbol::ZLK).encode()).unwrap()))
		).into(),
		// ZLK:KSM = 150:1
		//ZLK has a decimal of 18, while KSM is 12.
		ksm_per_second::<Runtime>() * 150 * 1_000_000,
		0
	);
	pub KarPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::karura::ID), Junction::from(BoundedVec::try_from(parachains::karura::KAR_KEY.to_vec()).unwrap()))
		).into(),
		// KAR:KSM = 100:1
		ksm_per_second::<Runtime>() * 100,
		0
	);
	pub KusdPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::karura::ID), Junction::from(BoundedVec::try_from(parachains::karura::KUSD_KEY.to_vec()).unwrap()))
		).into(),
		// kUSD:KSM = 400:1
		ksm_per_second::<Runtime>() * 400,
		0
	);
	pub PhaPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			1,
			X1(Parachain(parachains::phala::ID)),
		).into(),
		// PHA:KSM = 400:1
		ksm_per_second::<Runtime>() * 400,
		0
	);
	pub RmrkPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::Statemine::ID), GeneralIndex(parachains::Statemine::RMRK_ID.into()))
		).into(),
		// rmrk:KSM = 10:1
		ksm_per_second::<Runtime>() * 10 / 100, //rmrk currency decimal as 10
		0
	);
	pub RmrkNewPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			1,
			X3(Parachain(parachains::Statemine::ID), PalletInstance(parachains::Statemine::PALLET_ID),GeneralIndex(parachains::Statemine::RMRK_ID.into()))
		).into(),
		// rmrk:KSM = 10:1
		ksm_per_second::<Runtime>() * 10 / 100, //rmrk currency decimal as 10
		0
	);
	pub MovrPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::moonriver::ID), PalletInstance(parachains::moonriver::PALLET_ID.into()))
		).into(),
		// MOVR:KSM = 2.67:1
		ksm_per_second::<Runtime>() * 267 * 10_000, //movr currency decimal as 18
		0
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
			RuntimeCall::Bounties(
				pallet_bounties::Call::propose_bounty { .. } |
				pallet_bounties::Call::approve_bounty { .. } |
				pallet_bounties::Call::propose_curator { .. } |
				pallet_bounties::Call::unassign_curator { .. } |
				pallet_bounties::Call::accept_curator { .. } |
				pallet_bounties::Call::award_bounty { .. } |
				pallet_bounties::Call::claim_bounty { .. } |
				pallet_bounties::Call::close_bounty { .. },
			) |
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

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type AssetClaims = PolkadotXcm;
	type AssetTransactor = BifrostAssetTransactor;
	type AssetTrap = BifrostDropAssets<ToTreasury>;
	type Barrier = Barrier;
	type RuntimeCall = RuntimeCall;
	type IsReserve = MultiNativeAsset<RelativeReserveProvider>;
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type ResponseHandler = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type Trader = Trader;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmSender = XcmRouter;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = ConstU32<8>;
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = SafeCallFilter;
	type AssetLocker = ();
	type AssetExchanger = ();
	type FeeManager = ();
	type MessageExporter = ();
	type Aliasers = Nothing;
	type TransactionalProcessor = FrameTransactionalProcessor;
}

/// Local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<MultiLocation> = Some(Parent.into());
}

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type UniversalLocation = UniversalLocation;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmExecuteFilter = Nothing;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmExecutor = bifrost_primitives::DoNothingExecuteXcm;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmReserveTransferFilter = Everything;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmRouter = bifrost_primitives::DoNothingRouter;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmRouter = XcmRouter;
	type XcmTeleportFilter = Nothing;
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
	#[cfg(feature = "runtime-benchmarks")]
	type ReachableDest = ReachableDest;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type ChannelInfo = ParachainSystem;
	type RuntimeEvent = RuntimeEvent;
	type VersionWrapper = PolkadotXcm;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = cumulus_pallet_xcmp_queue::weights::SubstrateWeight<Runtime>;
	type PriceForSiblingDelivery = NoPriceForMessageDelivery<ParaId>;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type XcmExecutor = XcmExecutor<XcmConfig>;
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
	pub SelfLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(ParachainInfo::get().into())));
	pub SelfRelativeLocation: MultiLocation = MultiLocation::here();
	pub const BaseXcmWeight: Weight = Weight::from_parts(1000_000_000u64, 0);
	pub const MaxAssetsForTransfer: usize = 2;
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: MultiLocation| -> Option<u128> {
		Some(u128::MAX)
	};
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = BifrostCurrencyIdConvert<ParachainInfo>;
	type AccountIdToMultiLocation = BifrostAccountIdToMultiLocation;
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
	type MultiLocationsFilter = Everything;
	type ReserveProvider = RelativeReserveProvider;
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
	type AccountIdToMultiLocation = BifrostAccountIdToMultiLocation;
	type SalpHelper = Salp;
	type ParachainId = SelfParaChainId;
	type CallBackTimeOut = ConstU32<10>;
	type CurrencyIdConvert = AssetIdMaps<Runtime>;
}
