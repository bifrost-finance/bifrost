// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

pub use codec::Encode;
pub use cumulus_pallet_dmp_queue;
pub use cumulus_pallet_xcmp_queue;
pub use cumulus_primitives_core::{self, ParaId};
use frame_support::{sp_io, traits::GenesisBuild};
pub use frame_support::{traits::Get, weights::Weight};
pub use paste;
pub use polkadot_runtime_parachains::{dmp, ump};
pub use sp_io::TestExternalities;
use sp_runtime::AccountId32;
use sp_std::vec::Vec;
pub use sp_std::{cell::RefCell, marker::PhantomData};
pub use xcm::{v0::prelude::*, VersionedXcm};
pub use xcm_executor::XcmExecutor;

pub use crate::{HandleDmpMessage, HandleUmpMessage, HandleXcmpMessage};

pub trait TestExt {
	fn new_ext() -> sp_io::TestExternalities;
	fn reset_ext();
	fn execute_with<R>(execute: impl FnOnce() -> R) -> R;
}

pub enum MessageKind {
	Ump,
	Dmp,
	Xcmp,
}

pub fn encode_xcm(message: Xcm<()>, message_kind: MessageKind) -> Vec<u8> {
	match message_kind {
		MessageKind::Ump | MessageKind::Dmp => VersionedXcm::<()>::from(message).encode(),
		MessageKind::Xcmp => {
			let fmt = polkadot_parachain::primitives::XcmpMessageFormat::ConcatenatedVersionedXcm;
			let mut outbound = fmt.encode();

			let encoded = VersionedXcm::<()>::from(message).encode();
			outbound.extend_from_slice(&encoded[..]);
			outbound
		},
	}
}

#[macro_export]
macro_rules! decl_test_relay_chain {
	(
		pub struct $name:ident {
			Runtime = $runtime:path,
			XcmConfig = $xcm_config:path,
			new_ext = $new_ext:expr,
		}
	) => {
		pub struct $name;

		$crate::__impl_ext!($name, $new_ext);

		impl $crate::HandleUmpMessage for $name {
			fn handle_ump_message(from: $crate::ParaId, msg: &[u8], max_weight: $crate::Weight) {
				use ump::UmpSink;
				Self::execute_with(|| {
					let _ = ump::XcmSink::<$crate::XcmExecutor<$xcm_config>, $runtime>::process_upward_message(from, msg, max_weight);
				});
			}
		}
	};
}

#[macro_export]
macro_rules! decl_test_parachain {
	(
		pub struct $name:ident {
			Runtime = $runtime:path,
			new_ext = $new_ext:expr,
		}
	) => {
		pub struct $name;

		$crate::__impl_ext!($name, $new_ext);

		impl $crate::HandleXcmpMessage for $name {
			fn handle_xcmp_message(
				from: $crate::ParaId,
				at_relay_block: u32,
				msg: &[u8],
				max_weight: $crate::Weight,
			) {
				use cumulus_primitives_core::XcmpMessageHandler;

				$name::execute_with(|| {
					cumulus_pallet_xcmp_queue::Pallet::<$runtime>::handle_xcmp_messages(
						vec![(from, at_relay_block, msg)].into_iter(),
						max_weight,
					);
				});
			}
		}

		impl $crate::HandleDmpMessage for $name {
			fn handle_dmp_message(at_relay_block: u32, msg: Vec<u8>, max_weight: $crate::Weight) {
				use cumulus_primitives_core::DmpMessageHandler;

				$name::execute_with(|| {
					cumulus_pallet_dmp_queue::Pallet::<$runtime>::handle_dmp_messages(
						vec![(at_relay_block, msg)].into_iter(),
						max_weight,
					);
				});
			}
		}
	};
}

#[macro_export]
macro_rules! __impl_ext {
	// entry point: generate ext name
	($name:ident, $new_ext:expr) => {
		$crate::paste::paste! {
			$crate::__impl_ext!(@impl $name, $new_ext, [<EXT_ $name:upper>]);
		}
	};
	// impl
	(@impl $name:ident, $new_ext:expr, $ext_name:ident) => {
		thread_local! {
			pub static $ext_name: $crate::RefCell<TestExternalities>
				= $crate::RefCell::new($new_ext);
		}

		impl TestExt for $name {
			fn new_ext() -> TestExternalities {
				$new_ext
			}

			fn reset_ext() {
				$ext_name.with(|v| *v.borrow_mut() = $new_ext);
			}

			fn execute_with<R>(execute: impl FnOnce() -> R) -> R {
				$ext_name.with(|v| v.borrow_mut().execute_with(execute))
			}
		}
	};
}

#[macro_export]
macro_rules! decl_test_network {
	(
		pub struct $name:ident {
			relay_chain = $relay_chain:ty,
			parachains = vec![ $( ($para_id:expr, $parachain:ty), )* ],
		}
	) => {
		pub struct $name;

		impl $name {
			pub fn reset() {

				<$relay_chain>::reset_ext();
				$( <$parachain>::reset_ext(); )*
			}
		}

		/// XCM router for parachain.
		pub struct ParachainXcmRouter<T>($crate::PhantomData<T>);

		impl<T: $crate::Get<$crate::ParaId>> $crate::SendXcm for ParachainXcmRouter<T> {
			fn send_xcm(destination: $crate::MultiLocation, message: $crate::Xcm<()>) -> $crate::XcmResult {
				use $crate::{HandleUmpMessage, HandleXcmpMessage};

				match destination {
					$crate::X1($crate::Parent) => {
						let encoded = encode_xcm(message, MessageKind::Ump);
						<$relay_chain>::handle_ump_message(T::get(), &encoded[..], $crate::Weight::max_value());
						// TODO: update max weight
						Ok(())
					},
					$(
						$crate::X2($crate::Parent, $crate::Parachain(id)) if id == $para_id => {
							let encoded = encode_xcm(message, MessageKind::Xcmp);
							// TODO: update max weight; update `at_relay_block`
							<$parachain>::handle_xcmp_message(T::get(), 1, &encoded[..], $crate::Weight::max_value());
							Ok(())
						},
					)*
					_ => Err($crate::XcmError::CannotReachDestination(destination, message)),
				}
			}
		}

		/// XCM router, only sends DMP messages.
		pub struct RelayChainXcmRouter;
		impl $crate::SendXcm for RelayChainXcmRouter {
			fn send_xcm(destination: $crate::MultiLocation, message: $crate::Xcm<()>) -> $crate::XcmResult {
				use $crate::HandleDmpMessage;

				match destination {
					$(
						$crate::X1($crate::Parachain(id)) if id == $para_id => {
							let encoded = encode_xcm(message, MessageKind::Dmp);
							// TODO: update max weight; update `at_relay_block`
							<$parachain>::handle_dmp_message(1, encoded, $crate::Weight::max_value());
							Ok(())
						},
					)*
					_ => Err($crate::XcmError::SendFailed("Only sends to children parachain.")),
				}
			}
		}
	};
}

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);

decl_test_parachain! {
	pub struct ParaA {
		Runtime = para::Runtime,
		new_ext = para_ext(1),
	}
}

decl_test_parachain! {
	pub struct ParaB {
		Runtime = para::Runtime,
		new_ext = para_ext(2),
	}
}

decl_test_relay_chain! {
	pub struct Relay {
		Runtime = relay::Runtime,
		XcmConfig = relay::XcmConfig,
		new_ext = relay_ext(),
	}
}

decl_test_network! {
	pub struct MockNet {
		relay_chain = Relay,
		parachains = vec![
			(1, ParaA),
			(2, ParaB),
		],
	}
}

pub const INITIAL_BALANCE: u128 = 1_000_000_000;

pub fn para_ext(para_id: u32) -> sp_io::TestExternalities {
	use para::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: para_id.into() };

	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(
		&parachain_info_config,
		&mut t,
	)
	.unwrap();

	pallet_balances::GenesisConfig::<Runtime> { balances: vec![(ALICE, INITIAL_BALANCE)] }
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn relay_ext() -> sp_io::TestExternalities {
	use relay::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	pallet_balances::GenesisConfig::<Runtime> { balances: vec![(ALICE, INITIAL_BALANCE)] }
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub type RelayChainPalletXcm = pallet_xcm::Pallet<relay::Runtime>;
pub type ParachainPalletXcm = pallet_xcm::Pallet<para::Runtime>;

pub mod para {
	use frame_support::{
		construct_runtime, parameter_types,
		traits::Everything,
		weights::{constants::WEIGHT_PER_SECOND, Weight},
	};
	use frame_system::EnsureRoot;
	use pallet_xcm::XcmPassthrough;
	use polkadot_parachain::primitives::Sibling;
	use sp_core::H256;
	use sp_runtime::{testing::Header, traits::IdentityLookup, AccountId32};
	pub use xcm::v0::{
		Junction::{Parachain, Parent},
		MultiAsset,
		MultiLocation::{self, X1, X2, X3},
		NetworkId, Xcm,
	};
	pub use xcm_builder::{
		AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom,
		CurrencyAdapter as XcmCurrencyAdapter, EnsureXcmOrigin, FixedRateOfConcreteFungible,
		FixedWeightBounds, IsConcrete, LocationInverter, NativeAsset, ParentAsSuperuser,
		ParentIsDefault, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
		SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
		TakeWeightCredit,
	};
	use xcm_executor::{Config, XcmExecutor};

	pub type AccountId = AccountId32;
	pub type Balance = u128;

	parameter_types! {
		pub const BlockHashCount: u64 = 250;
	}

	impl frame_system::Config for Runtime {
		type AccountData = pallet_balances::AccountData<Balance>;
		type AccountId = AccountId;
		type BaseCallFilter = ();
		type BlockHashCount = BlockHashCount;
		type BlockLength = ();
		type BlockNumber = u64;
		type BlockWeights = ();
		type Call = Call;
		type DbWeight = ();
		type Event = Event;
		type Hash = H256;
		type Hashing = ::sp_runtime::traits::BlakeTwo256;
		type Header = Header;
		type Index = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type OnKilledAccount = ();
		type OnNewAccount = ();
		type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
		type Origin = Origin;
		type PalletInfo = PalletInfo;
		type SS58Prefix = ();
		type SystemWeightInfo = ();
		type Version = ();
	}

	parameter_types! {
		pub ExistentialDeposit: Balance = 1;
		pub const MaxLocks: u32 = 50;
		pub const MaxReserves: u32 = 50;
	}

	impl pallet_balances::Config for Runtime {
		type AccountStore = System;
		type Balance = Balance;
		type DustRemoval = ();
		type Event = Event;
		type ExistentialDeposit = ExistentialDeposit;
		type MaxLocks = MaxLocks;
		type MaxReserves = MaxReserves;
		type ReserveIdentifier = [u8; 8];
		type WeightInfo = ();
	}

	parameter_types! {
		pub const ReservedXcmpWeight: Weight = WEIGHT_PER_SECOND / 4;
		pub const ReservedDmpWeight: Weight = WEIGHT_PER_SECOND / 4;
	}

	impl cumulus_pallet_parachain_system::Config for Runtime {
		type DmpMessageHandler = DmpQueue;
		type Event = Event;
		type OnValidationData = ();
		type OutboundXcmpMessageSource = XcmpQueue;
		type ReservedDmpWeight = ReservedDmpWeight;
		type ReservedXcmpWeight = ReservedXcmpWeight;
		type SelfParaId = ParachainInfo;
		type XcmpMessageHandler = XcmpQueue;
	}

	impl parachain_info::Config for Runtime {}

	parameter_types! {
		pub const KsmLocation: MultiLocation = MultiLocation::X1(Parent);
		pub const RelayNetwork: NetworkId = NetworkId::Kusama;
		pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
		pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
	}

	pub type LocationToAccountId = (
		ParentIsDefault<AccountId>,
		SiblingParachainConvertsVia<Sibling, AccountId>,
		AccountId32Aliases<RelayNetwork, AccountId>,
	);

	pub type XcmOriginToCallOrigin = (
		SovereignSignedViaLocation<LocationToAccountId, Origin>,
		RelayChainAsNative<RelayChainOrigin, Origin>,
		SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
		SignedAccountId32AsNative<RelayNetwork, Origin>,
		XcmPassthrough<Origin>,
	);

	parameter_types! {
		pub const UnitWeightCost: Weight = 1;
		pub KsmPerSecond: (MultiLocation, u128) = (X1(Parent), 1);
	}

	pub type LocalAssetTransactor =
		XcmCurrencyAdapter<Balances, IsConcrete<KsmLocation>, LocationToAccountId, AccountId, ()>;

	pub type XcmRouter = crate::mock::ParachainXcmRouter<ParachainInfo>;
	pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

	pub struct XcmConfig;
	impl Config for XcmConfig {
		type AssetTransactor = LocalAssetTransactor;
		type Barrier = Barrier;
		type Call = Call;
		type IsReserve = NativeAsset;
		type IsTeleporter = ();
		type LocationInverter = LocationInverter<Ancestry>;
		type OriginConverter = XcmOriginToCallOrigin;
		type ResponseHandler = ();
		type Trader = FixedRateOfConcreteFungible<KsmPerSecond, ()>;
		type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
		type XcmSender = XcmRouter;
	}

	impl cumulus_pallet_xcmp_queue::Config for Runtime {
		type ChannelInfo = ParachainSystem;
		type Event = Event;
		type XcmExecutor = XcmExecutor<XcmConfig>;
	}

	impl cumulus_pallet_dmp_queue::Config for Runtime {
		type Event = Event;
		type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
		type XcmExecutor = XcmExecutor<XcmConfig>;
	}

	impl cumulus_pallet_xcm::Config for Runtime {
		type Event = Event;
		type XcmExecutor = XcmExecutor<XcmConfig>;
	}

	pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

	impl pallet_xcm::Config for Runtime {
		type Event = Event;
		type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
		type LocationInverter = LocationInverter<Ancestry>;
		type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
		type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
		type XcmExecuteFilter = Everything;
		type XcmExecutor = XcmExecutor<XcmConfig>;
		type XcmReserveTransferFilter = Everything;
		type XcmRouter = XcmRouter;
		type XcmTeleportFilter = ();
	}

	type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
	type Block = frame_system::mocking::MockBlock<Runtime>;

	construct_runtime!(
		pub enum Runtime where
			Block = Block,
			NodeBlock = Block,
			UncheckedExtrinsic = UncheckedExtrinsic,
		{
			System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
			Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},

			ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Config, Storage, Inherent, Event<T>},
			ParachainInfo: parachain_info::{Pallet, Storage, Config},
			XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>},
			DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>},
			CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin},

			PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin},
		}
	);
}

pub mod relay {
	use cumulus_primitives_core::ParaId;
	use frame_support::{
		construct_runtime, parameter_types,
		traits::{Everything, OnUnbalanced},
		weights::{Weight, WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial},
	};
	use pallet_balances::NegativeImbalance;
	use polkadot_primitives;
	use polkadot_runtime_parachains::{configuration, origin, shared, ump};
	use smallvec::smallvec;
	use sp_core::H256;
	use sp_runtime::{testing::Header, traits::IdentityLookup, AccountId32, Perbill};
	use xcm::v0::{MultiLocation, NetworkId};
	use xcm_builder::{
		AccountId32Aliases, AllowTopLevelPaidExecutionFrom, ChildParachainAsNative,
		ChildParachainConvertsVia, ChildSystemParachainAsSuperuser,
		CurrencyAdapter as XcmCurrencyAdapter, FixedWeightBounds, IsConcrete, LocationInverter,
		SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
		TakeWeightCredit, UsingComponents,
	};
	use xcm_executor::{Config, XcmExecutor};

	pub type AccountId = AccountId32;
	pub type Balance = u128;

	parameter_types! {
		pub const BlockHashCount: u64 = 250;
	}

	/// Logic for the author to get a portion of fees.
	pub struct ToAuthor<R>(sp_std::marker::PhantomData<R>);
	impl<R> OnUnbalanced<NegativeImbalance<R>> for ToAuthor<R>
	where
		R: pallet_balances::Config,
		<R as frame_system::Config>::AccountId: From<polkadot_primitives::v1::AccountId>,
		<R as frame_system::Config>::AccountId: Into<polkadot_primitives::v1::AccountId>,
		<R as frame_system::Config>::Event: From<pallet_balances::Event<R>>,
	{
		fn on_nonzero_unbalanced(_amount: NegativeImbalance<R>) {}
	}

	pub struct WeightToFee;
	impl WeightToFeePolynomial for WeightToFee {
		type Balance = Balance;
		fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
			let p = 1_000_000_00;
			let q = 1_000_000;
			smallvec![WeightToFeeCoefficient {
				degree: 1,
				negative: false,
				coeff_frac: Perbill::from_rational(p % q, q),
				coeff_integer: p / q,
			}]
		}
	}

	impl frame_system::Config for Runtime {
		type AccountData = pallet_balances::AccountData<Balance>;
		type AccountId = AccountId;
		type BaseCallFilter = ();
		type BlockHashCount = BlockHashCount;
		type BlockLength = ();
		type BlockNumber = u64;
		type BlockWeights = ();
		type Call = Call;
		type DbWeight = ();
		type Event = Event;
		type Hash = H256;
		type Hashing = ::sp_runtime::traits::BlakeTwo256;
		type Header = Header;
		type Index = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type OnKilledAccount = ();
		type OnNewAccount = ();
		type OnSetCode = ();
		type Origin = Origin;
		type PalletInfo = PalletInfo;
		type SS58Prefix = ();
		type SystemWeightInfo = ();
		type Version = ();
	}

	parameter_types! {
		pub ExistentialDeposit: Balance = 1;
		pub const MaxLocks: u32 = 50;
		pub const MaxReserves: u32 = 50;
	}

	impl pallet_balances::Config for Runtime {
		type AccountStore = System;
		type Balance = Balance;
		type DustRemoval = ();
		type Event = Event;
		type ExistentialDeposit = ExistentialDeposit;
		type MaxLocks = MaxLocks;
		type MaxReserves = MaxReserves;
		type ReserveIdentifier = [u8; 8];
		type WeightInfo = ();
	}

	impl shared::Config for Runtime {}

	impl configuration::Config for Runtime {}

	parameter_types! {
		pub const KsmLocation: MultiLocation = MultiLocation::Null;
		pub const KusamaNetwork: NetworkId = NetworkId::Kusama;
		pub const AnyNetwork: NetworkId = NetworkId::Any;
		pub Ancestry: MultiLocation = MultiLocation::Null;
		pub UnitWeightCost: Weight = 1_000;
	}

	pub type SovereignAccountOf = (
		ChildParachainConvertsVia<ParaId, AccountId>,
		AccountId32Aliases<KusamaNetwork, AccountId>,
	);

	pub type LocalAssetTransactor =
		XcmCurrencyAdapter<Balances, IsConcrete<KsmLocation>, SovereignAccountOf, AccountId, ()>;

	type LocalOriginConverter = (
		SovereignSignedViaLocation<SovereignAccountOf, Origin>,
		ChildParachainAsNative<origin::Origin, Origin>,
		SignedAccountId32AsNative<KusamaNetwork, Origin>,
		ChildSystemParachainAsSuperuser<ParaId, Origin>,
	);

	parameter_types! {
		pub const BaseXcmWeight: Weight = 1_000;
		pub KsmPerSecond: (MultiLocation, u128) = (KsmLocation::get(), 1);
	}

	pub type XcmRouter = crate::mock::RelayChainXcmRouter;
	pub type Barrier = (TakeWeightCredit, AllowTopLevelPaidExecutionFrom<Everything>);

	pub struct XcmConfig;
	impl Config for XcmConfig {
		type AssetTransactor = LocalAssetTransactor;
		type Barrier = Barrier;
		type Call = Call;
		type IsReserve = ();
		type IsTeleporter = ();
		type LocationInverter = LocationInverter<Ancestry>;
		type OriginConverter = LocalOriginConverter;
		type ResponseHandler = ();
		type Trader =
			UsingComponents<WeightToFee, KsmLocation, AccountId, Balances, ToAuthor<Runtime>>;
		type Weigher = FixedWeightBounds<BaseXcmWeight, Call>;
		type XcmSender = XcmRouter;
	}

	pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, KusamaNetwork>;

	impl pallet_xcm::Config for Runtime {
		type Event = Event;
		// Anyone can execute XCM messages locally...
		type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<Origin, LocalOriginToLocation>;
		type LocationInverter = LocationInverter<Ancestry>;
		type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<Origin, LocalOriginToLocation>;
		type Weigher = FixedWeightBounds<BaseXcmWeight, Call>;
		type XcmExecuteFilter = ();
		type XcmExecutor = XcmExecutor<XcmConfig>;
		type XcmReserveTransferFilter = Everything;
		type XcmRouter = XcmRouter;
		type XcmTeleportFilter = Everything;
	}

	parameter_types! {
		pub const FirstMessageFactorPercent: u64 = 100;
	}

	impl ump::Config for Runtime {
		type Event = Event;
		type FirstMessageFactorPercent = FirstMessageFactorPercent;
		type UmpSink = ump::XcmSink<XcmExecutor<XcmConfig>, Runtime>;
	}

	impl origin::Config for Runtime {}

	type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
	type Block = frame_system::mocking::MockBlock<Runtime>;

	construct_runtime!(
		pub enum Runtime where
			Block = Block,
			NodeBlock = Block,
			UncheckedExtrinsic = UncheckedExtrinsic,
		{
			System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
			Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
			ParasOrigin: origin::{Pallet, Origin},
			ParasUmp: ump::{Pallet, Call, Storage, Event},
			XcmPallet: pallet_xcm::{Pallet, Call, Storage, Event<T>},
		}
	);
}
