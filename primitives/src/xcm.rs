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

use crate::AccountId;
use frame_support::{
	parameter_types,
	traits::{ContainsPair, Get},
};
use sp_runtime::traits::Convert;
use sp_std::marker::PhantomData;
use xcm::{
	latest::Asset,
	prelude::{AccountId32, Ethereum, Fungible, GlobalConsensus, Parachain},
	v4::{AssetId, InteriorLocation, Location, NetworkId, Parent},
};

// Parachain Id
parameter_types! {
	pub const AssetHubChainId: u32 = 1000;
	pub const AstarChainId: u32 = 2006;
	pub const BifrostKusamaChainId: u32 = 2001;
	pub const BifrostPolkadotChainId: u32 = 2030;
	pub const BridgeHubChainId: u32 = 1002;
	pub const HydrationChainId: u32 = 2034;
	pub const InterlayChainId: u32 = 2032;
	pub const MantaChainId: u32 = 2104;
	pub const MoonbeamChainId: u32 = 2004;
	pub const MoonriverChainId: u32 = 2023;
	pub const PhalaChainId: u32 = 2035;
	pub const KaruraChainId: u32 = 2000;
	pub const EthereumChainId: u64 = 1;
}

// Token Location
parameter_types! {
	pub MoonbeamTokenLocation: Location = Location::here();
}

// Location
parameter_types! {
	pub SelfLocation: Location = Location::here();
	pub AssetHubLocation: Location = Location::new(1, Parachain(AssetHubChainId::get()));
	pub EthereumLocation: Location = Location::new(2, [GlobalConsensus(Ethereum { chain_id: EthereumChainId::get() })]);

	pub const KusamaNetwork: NetworkId = NetworkId::Kusama;
	pub const PolkadotNetwork: NetworkId = NetworkId::Polkadot;

	pub KusamaUniversalLocation: InteriorLocation = [GlobalConsensus(KusamaNetwork::get()), Parachain(BifrostKusamaChainId::get())].into();
	pub PolkadotUniversalLocation: InteriorLocation = [GlobalConsensus(PolkadotNetwork::get()), Parachain(BifrostPolkadotChainId::get())].into();
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

/// Convert AccountId to Location
pub struct AccountIdToLocation;
impl Convert<AccountId, Location> for AccountIdToLocation {
	fn convert(account: AccountId) -> Location {
		Location::new(0, [AccountId32 { network: None, id: account.into() }])
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use xcm::v4::Junctions;

	#[test]
	fn parachain_location() {
		let assethub_location: Location = (Parent, Parachain(1000)).into();
		assert_eq!(assethub_location, Location::new(1, Parachain(1000)));
	}

	#[test]
	fn bifrost_account_to_location() {
		let account: AccountId = AccountId::new([0u8; 32]);
		let location: Location = AccountIdToLocation::convert(account);
		assert_eq!(location, Location::new(0, [AccountId32 { network: None, id: [0u8; 32] }]));
	}

	#[test]
	fn universal_location() {
		assert_eq!(
			KusamaUniversalLocation::get(),
			Junctions::X2([GlobalConsensus(NetworkId::Kusama), Parachain(2001)].into())
		);
		assert_eq!(
			PolkadotUniversalLocation::get(),
			Junctions::X2([GlobalConsensus(NetworkId::Polkadot), Parachain(2030)].into())
		);
	}
}
