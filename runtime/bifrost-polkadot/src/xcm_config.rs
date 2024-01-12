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

use assets_common::matching::IsForeignConcreteAsset;
use polkadot_runtime_common::xcm_sender::{ExponentialPrice, NoPriceForMessageDelivery};
use xcm_builder::{NetworkExportTableItem, NetworkExportTable, DenyReserveTransferToRelayChain, WithComputedOrigin, XcmFeeManagerFromComponents, XcmFeeToAccount, LocalMint, NativeAsset};
use super::*;
use sp_runtime::SaturatedConversion;
use frame_support::pallet_prelude::PalletInfoAccess;
use bifrost_asset_registry::AssetIdMaps;
use bifrost_primitives::{
	AccountId, CurrencyId, CurrencyIdMapping, TokenSymbol, DOT_TOKEN_ID, GLMR_TOKEN_ID,
};
use bifrost_primitives::BNC;
use westend_runtime_constants::system_parachain;
pub use bifrost_xcm_interface::traits::{parachains, XcmBaseWeight};
pub use cumulus_primitives_core::ParaId;
use frame_support::{
	ensure,
	sp_runtime::traits::{CheckedConversion, Convert},
	traits::{ContainsPair, Get, ProcessMessageError},
};
use parachains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};
use frame_support::traits::TransformOrigin;
use cumulus_primitives_core::AggregateMessageOrigin;
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
use orml_traits::location::Reserve;
pub use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key, MultiCurrency};
use pallet_xcm::XcmPassthrough;
use parachains_common::xcm_config::{AssetFeeAsExistentialDepositMultiplier, ConcreteAssetFromSystem, RelayOrOtherSystemParachains};
use sp_core::bounded::BoundedVec;
use xcm_builder::{Account32Hash, AllowExplicitUnpaidExecutionFrom, FungiblesAdapter, GlobalConsensusParachainConvertsFor, NoChecking, StartsWith, StartsWithExplicitGlobalConsensus, TrailingSetTopicAsId, WithUniqueTopic};

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
			Token2(DOT_TOKEN_ID) => Some(MultiLocation::parent()),
			Native(BNC) => Some(native_currency_location(id)),
			// Moonbeam Native token
			Token2(GLMR_TOKEN_ID) => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::moonbeam::ID),
					PalletInstance(parachains::moonbeam::PALLET_ID.into()),
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
			return Some(Token2(DOT_TOKEN_ID));
		}

		if let Some(currency_id) = AssetIdMaps::<Runtime>::get_currency_id(location) {
			return Some(currency_id);
		}

		match location {
			MultiLocation { parents, interior } if parents == 1 => match interior {
				X2(Parachain(id), PalletInstance(index))
					if ((id == parachains::moonbeam::ID) &&
						(index == parachains::moonbeam::PALLET_ID)) =>
					Some(Token2(GLMR_TOKEN_ID)),
				X2(Parachain(id), GeneralKey { data, length })
					if (id == u32::from(ParachainInfo::parachain_id())) =>
				{
					let key = &data[..length as usize];
					if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
						match currency_id {
							Native(BNC) => Some(currency_id),
							_ => None,
						}
					} else {
						None
					}
				},
				_ => None,
			},
			MultiLocation { parents, interior } if parents == 0 => match interior {
				X1(GeneralKey { data, length }) => {
					// decode the general key
					let key = &data[..length as usize];
					if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
						match currency_id {
							Native(BNC) => Some(currency_id),
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
	pub const DotLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Rococo;
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
	// Derives a private `Account32` by hashing `("multiloc", received multilocation)`
	Account32Hash<RelayNetwork, AccountId>,
	// Different global consensus parachain sovereign account.
	// (Used for over-bridge transfers and reserve processing)
	GlobalConsensusParachainConvertsFor<UniversalLocation, AccountId>,
);

pub type ForeignCreatorsSovereignAccountOf = (
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RelayNetwork, AccountId>,
	ParentIsPreset<AccountId>,
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
	pub UniversalLocationNetworkId: NetworkId = UniversalLocation::get().global_consensus().unwrap();
	pub FeeAssetId: AssetId = Concrete(xcm_config::TokenLocation::get());
		/// The base fee for the message delivery fees.
	pub const BaseDeliveryFee: u128 = CENTS.saturating_mul(3);
	pub RelayTreasuryLocation: MultiLocation = (Parent, PalletInstance(18)).into();
	pub tLocation: MultiLocation = MultiLocation::new(2, X3(GlobalConsensus(NetworkId::Westend), Parachain(2001), Junction::from(BoundedVec::try_from(BNC.encode()).unwrap())));
	pub bncLocation: MultiLocation = MultiLocation::new(0, X1(Junction::from(BoundedVec::try_from(BNC.encode()).unwrap())));
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

pub struct AllXcm<T>(PhantomData<T>);
impl<T: Contains<MultiLocation>> ShouldExecute for AllXcm<T> {
	fn should_execute<Call>(
		origin: &MultiLocation,
		message: &mut [Instruction<Call>],
		max_weight: Weight,
		_weight_credit: &mut Properties,
	) -> Result<(), ProcessMessageError> {
		log::debug!(
			target: "xcm::barriers",
			"AllXcm origin:
			{:?}, message: {:?}, max_weight: {:?}, weight_credit: {:?}",
			origin, message, max_weight, _weight_credit,
		);
		Ok(())
	}
}

match_types! {
	pub type ParentOrParentsPlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { .. }) }
	};
}

pub type Barrier = TrailingSetTopicAsId<
	xcm_builder::DenyThenTry<
		DenyReserveTransferToRelayChain,
		(
			AllXcm<Everything>,
			TakeWeightCredit,
			// Expected responses are OK.
			AllowKnownQueryResponses<PolkadotXcm>,
			// Allow XCMs with some computed origins to pass through.
			WithComputedOrigin<
				(
					// If the message is one that immediately attempts to pay for execution, then
					// allow it.
					AllowTopLevelPaidExecutionFrom<Everything>,
					// Parent, its pluralities (i.e. governance bodies), relay treasury pallet and
					// BridgeHub get free execution.
					AllowExplicitUnpaidExecutionFrom<(
						ParentOrParentsPlurality,
						Equals<RelayTreasuryLocation>,
						Equals<bridging::SiblingBridgeHub>,
					)>,
					// Subscriptions for version tracking are OK.
					AllowSubscriptionsFrom<Everything>,
				),
				UniversalLocation,
				ConstU32<8>,
			>,
		),
	>,
>;

// pub type Barrier = TrailingSetTopicAsId<(
// 	// Weight that is paid for may be consumed.
// 	TakeWeightCredit,
// 	// Expected responses are OK.
// 	AllowKnownQueryResponses<PolkadotXcm>,
// 	// If the message is one that immediately attemps to pay for execution, then allow it.
// 	AllowTopLevelPaidExecutionFrom<Everything>,
// 	// Subscriptions for version tracking are OK.
// 	AllowSubscriptionsFrom<Everything>,
// 	// Barrier allowing a top level paid message with DescendOrigin instruction
// 	AllowTopLevelPaidExecutionDescendOriginFirst<Everything>,
// 	AllowExplicitUnpaidExecutionFrom<(
// 		Equals<bridging::SiblingBridgeHub>,
// 	)>,
// )>;

/// Means for transacting the native currency on this chain.
pub type CurrencyTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<bncLocation>,
	// Convert an XCM MultiLocation into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports of `Balances`.
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

/// Means for transacting assets on this chain.
pub type AssetTransactors = (CurrencyTransactor, BifrostAssetTransactor);

match_types! {
	pub type SystemParachains: impl Contains<MultiLocation> = {
		MultiLocation {
			parents: 1,
			interior: X1(Parachain(
				system_parachain::ASSET_HUB_ID |
				system_parachain::BRIDGE_HUB_ID
			)),
		}
	};
}

/// Locations that will not be charged fees in the executor,
/// either execution or delivery.
/// We only waive fees for system functions, which these locations represent.
pub type WaivedLocations = (RelayOrOtherSystemParachains<SystemParachains, Runtime>, Equals<RelayTreasuryLocation>);

parameter_types! {
	pub kPerSecond: (AssetId, u128, u128) = (MultiLocation::new(2, X3(GlobalConsensus(NetworkId::Westend), Parachain(2001), Junction::from(BoundedVec::try_from(BNC.encode()).unwrap()))).into(), dot_per_second::<Runtime>(),0);
	pub XcmAssetFeesReceiver: Option<AccountId> = Authorship::author();
	pub DotPerSecond: (AssetId,u128, u128) = (MultiLocation::parent().into(), dot_per_second::<Runtime>(),0);
	pub BncPerSecond: (AssetId,u128, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(SelfParaId::get()), Junction::from(BoundedVec::try_from(NativeCurrencyId::get().encode()).unwrap())),
		).into(),
		// BNC:DOT = 80:1
		dot_per_second::<Runtime>() * 80,
		0
	);
	pub BncNewPerSecond: (AssetId,u128, u128) = (
		MultiLocation::new(
			0,
			X1(Junction::from(BoundedVec::try_from(NativeCurrencyId::get().encode()).unwrap()))
		).into(),
		// BNC:DOT = 80:1
		dot_per_second::<Runtime>() * 80,
	0
	);
	pub ZlkPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(SelfParaId::get()), Junction::from(BoundedVec::try_from(CurrencyId::Token(TokenSymbol::ZLK).encode()).unwrap()))
		).into(),
		// ZLK:KSM = 150:1
		dot_per_second::<Runtime>() * 150 * 1_000_000,
	0
	);
	pub ZlkNewPerSecond: (AssetId, u128,u128) = (
		MultiLocation::new(
			0,
			X1(Junction::from(BoundedVec::try_from(CurrencyId::Token(TokenSymbol::ZLK).encode()).unwrap()))
		).into(),
		// ZLK:KSM = 150:1
		dot_per_second::<Runtime>() * 150 * 1_000_000,
	0
	);
	pub BasePerSecond: u128 = dot_per_second::<Runtime>();
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
	FixedRateOfFungible<kPerSecond, ToTreasury>,
	FixedRateOfFungible<BncPerSecond, ToTreasury>,
	FixedRateOfFungible<BncNewPerSecond, ToTreasury>,
	FixedRateOfFungible<DotPerSecond, ToTreasury>,
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
			RuntimeCall::TokenConversion(
				bifrost_vstoken_conversion::Call::vsbond_convert_to_vstoken { .. } |
				bifrost_vstoken_conversion::Call::vstoken_convert_to_vsbond { .. }
			) |
			RuntimeCall::VeMinting(
				bifrost_ve_minting::Call::increase_amount { .. } |
				bifrost_ve_minting::Call::increase_unlock_time { .. } |
				bifrost_ve_minting::Call::withdraw { .. } |
				bifrost_ve_minting::Call::get_rewards { .. }
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
			) => true,
			_ => false,
		}
	}
}

/// Cases where a remote origin is accepted as trusted Teleporter for a given asset:
///
/// - ROC with the parent Relay Chain and sibling system parachains; and
/// - Sibling parachains' assets from where they originate (as `ForeignCreators`).
pub type TrustedTeleporters = (
	ConcreteAssetFromSystem<TokenLocation>,
	IsForeignConcreteAsset<FromSiblingParachain<parachain_info::Pallet<Runtime>>>,
);

parameter_types! {
	pub bbLocation: MultiLocation = MultiLocation::new(0, X1(Junction::from(BoundedVec::try_from(BNC.encode()).unwrap())));
	pub AssetHubLocation: MultiLocation = MultiLocation::new(2, X2(GlobalConsensus(NetworkId::Westend), Parachain(2001)));
	pub TrappistNative: MultiAssetFilter = Wild(AllOf { fun: WildFungible, id: Concrete(bbLocation::get()) });
	pub AssetHubTrustedTeleporter: (MultiAssetFilter, MultiLocation) = (TrappistNative::get(), AssetHubLocation::get());
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type AssetClaims = PolkadotXcm;
	type AssetTransactor = AssetTransactors;
	type AssetTrap = BifrostDropAssets<ToTreasury>;
	type Barrier = Barrier;
	type RuntimeCall = RuntimeCall;
	type IsReserve = (bridging::to_westend::IsTrustedBridgedReserveLocationForConcreteAsset,);
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
	type UniversalAliases = (bridging::to_westend::UniversalAliases,);
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type AssetLocker = ();
	type AssetExchanger = ();
	type FeeManager = XcmFeeManagerFromComponents<
		WaivedLocations,
		XcmFeeToAccount<Self::AssetTransactor, AccountId, BifrostTreasuryAccount>,
	>;
	type MessageExporter = ();
	type Aliasers = Nothing;
}

/// Local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

pub type PriceForParentDelivery =  ExponentialPrice<FeeAssetId, BaseDeliveryFee, TransactionByteFee, ParachainSystem>;

/// For routing XCM messages which do not cross local consensus boundary.
type LocalXcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, PriceForParentDelivery>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = WithUniqueTopic<(
	LocalXcmRouter,
	// Router which wraps and sends xcm to BridgeHub to be delivered to the Westend
	// GlobalConsensus
	ToWestendXcmRouter,
)>;

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
	type XcmExecuteFilter = Everything;
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
			&CurrencyId::Token2(DOT_TOKEN_ID) => 1_000_000,  // DOT
			&CurrencyId::LPToken(..) => 1 * micro::<Runtime>(NativeCurrencyId::get()),
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
			&BifrostVsbondPalletId::get(),
		)
		.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(
			&SlpEntrancePalletId::get(),
		)
		.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(&SlpExitPalletId::get())
			.eq(a) || FarmingKeeperPalletId::get().check_sub_account::<PoolId>(a) ||
			FarmingRewardIssuerPalletId::get().check_sub_account::<PoolId>(a) ||
			AccountIdConversion::<AccountId>::into_account_truncating(&BuybackPalletId::get())
				.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(
			&SystemMakerPalletId::get(),
		)
		.eq(a) || FeeSharePalletId::get().check_sub_account::<DistributionId>(a) ||
			a.eq(&ZenklinkFeeAccount::get())
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
	pub const TokenLocation: MultiLocation = MultiLocation::parent();
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
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type AccountIdToMultiLocation = BifrostAccountIdToMultiLocation;
	type SalpHelper = Salp;
	type ParachainId = SelfParaChainId;
	type CallBackTimeOut = ConstU32<10>;
	type CurrencyIdConvert = AssetIdMaps<Runtime>;
}

/// All configuration related to bridging
pub mod bridging {
	use super::*;
	use assets_common::matching;
	use sp_std::collections::btree_set::BTreeSet;

	// common/shared parameters
	parameter_types! {
		/// Base price of every byte of the Rococo -> Westend message. Can be adjusted via
		/// governance `set_storage` call.
		///
		/// Default value is our estimation of the:
		///
		/// 1) an approximate cost of XCM execution (`ExportMessage` and surroundings) at Rococo bridge hub;
		///
		/// 2) the approximate cost of Rococo -> Westend message delivery transaction on Westend Bridge Hub,
		///    converted into ROCs using 1:1 conversion rate;
		///
		/// 3) the approximate cost of Rococo -> Westend message confirmation transaction on Rococo Bridge Hub.
		pub storage XcmBridgeHubRouterBaseFee: Balance =
			bp_bridge_hub_rococo::BridgeHubRococoBaseXcmFeeInRocs::get()
				.saturating_add(bp_bridge_hub_westend::BridgeHubWestendBaseDeliveryFeeInWnds::get())
				.saturating_add(bp_bridge_hub_rococo::BridgeHubRococoBaseConfirmationFeeInRocs::get());
		/// Price of every byte of the Rococo -> Westend message. Can be adjusted via
		/// governance `set_storage` call.
		pub storage XcmBridgeHubRouterByteFee: Balance = TransactionByteFee::get();

		pub SiblingBridgeHubParaId: u32 = bp_bridge_hub_rococo::BRIDGE_HUB_ROCOCO_PARACHAIN_ID;
		pub SiblingBridgeHub: MultiLocation = MultiLocation::new(1, X1(Parachain(SiblingBridgeHubParaId::get())));
		/// Router expects payment with this `AssetId`.
		/// (`AssetId` has to be aligned with `BridgeTable`)
		pub XcmBridgeHubRouterFeeAssetId: AssetId = TokenLocation::get().into();

		pub BridgeTable: sp_std::vec::Vec<NetworkExportTableItem> =
			sp_std::vec::Vec::new().into_iter()
			.chain(to_westend::BridgeTable::get())
			.collect();
	}

	pub type NetworkExportTable = xcm_builder::NetworkExportTable<BridgeTable>;

	pub mod to_westend {
		use bifrost_primitives::currency::VMOVR;
		use super::*;

		parameter_types! {
			pub SiblingBridgeHubWithBridgeHubWestendInstance: MultiLocation = MultiLocation::new(
				1,
				X2(
					Parachain(SiblingBridgeHubParaId::get()),
					PalletInstance(bp_bridge_hub_rococo::WITH_BRIDGE_ROCOCO_TO_WESTEND_MESSAGES_PALLET_INDEX)
				)
			);

			pub const WestendNetwork: NetworkId = NetworkId::Westend;
			pub BifrostKusama: MultiLocation = MultiLocation::new(2, X2(GlobalConsensus(WestendNetwork::get()), Parachain(2001)));
			pub WndLocation: MultiLocation = MultiLocation::new(2, X1(GlobalConsensus(WestendNetwork::get())));
			pub BncLocation: MultiLocation = MultiLocation::new(2, X3(GlobalConsensus(WestendNetwork::get()), Parachain(2001), Junction::from(BoundedVec::try_from(BNC.encode()).unwrap())));
			pub vMovrLocation: MultiLocation = MultiLocation::new(2, X3(GlobalConsensus(WestendNetwork::get()), Parachain(2001), Junction::from(BoundedVec::try_from(VMOVR.encode()).unwrap())));

			pub BncFromBifrostKusama: (MultiAssetFilter, MultiLocation) = (
				Wild(AllOf { fun: WildFungible, id: Concrete(BncLocation::get()) }),
				BifrostKusama::get()
			);

			pub vMovrFromBifrostKusama: (MultiAssetFilter, MultiLocation) = (
				Wild(AllOf { fun: WildFungible, id: Concrete(vMovrLocation::get()) }),
				BifrostKusama::get()
			);

			pub KsmFromBifrostKusama: (MultiAssetFilter, MultiLocation) = (
				Wild(AllOf { fun: WildFungible, id: Concrete(WndLocation::get()) }),
				BifrostKusama::get()
			);

			/// Set up exporters configuration.
			/// `Option<MultiAsset>` represents static "base fee" which is used for total delivery fee calculation.
			pub BridgeTable: sp_std::vec::Vec<NetworkExportTableItem> = sp_std::vec![
				NetworkExportTableItem::new(
					WestendNetwork::get(),
					Some(sp_std::vec![
						BifrostKusama::get().interior.split_global().expect("invalid configuration for BifrostKusama").1,
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
			pub UniversalAliases: BTreeSet<(MultiLocation, Junction)> = BTreeSet::from_iter(
				sp_std::vec![
					(SiblingBridgeHubWithBridgeHubWestendInstance::get(), GlobalConsensus(WestendNetwork::get()))
				]
			);
		}

		impl Contains<(MultiLocation, Junction)> for UniversalAliases {
			fn contains(alias: &(MultiLocation, Junction)) -> bool {
				UniversalAliases::get().contains(alias)
			}
		}

		/// Trusted reserve locations filter for `xcm_executor::Config::IsReserve`.
		/// Locations from which the runtime accepts reserved assets.
		pub type IsTrustedBridgedReserveLocationForConcreteAsset =
		matching::IsTrustedBridgedReserveLocationForConcreteAsset<
			UniversalLocation,
			(
				// allow receive BNC from AssetHubWestend
				xcm_builder::Case<BncFromBifrostKusama>,
				// and nothing else
				xcm_builder::Case<KsmFromBifrostKusama>,
				// and nothing else
				xcm_builder::Case<vMovrFromBifrostKusama>,
			),
		>;

		impl Contains<RuntimeCall> for ToWestendXcmRouter {
			fn contains(call: &RuntimeCall) -> bool {
				matches!(
					call,
					RuntimeCall::ToWestendXcmRouter(
						pallet_xcm_bridge_hub_router::Call::report_bridge_status { .. }
					)
				)
			}
		}
	}
}
