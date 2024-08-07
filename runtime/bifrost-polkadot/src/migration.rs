use super::*;
use frame_support::{pallet_prelude::*, storage_alias, traits::OnRuntimeUpgrade};
use log;
use parity_scale_codec::{Decode, Encode, EncodeLike, MaxEncodedLen};
use sp_std::fmt::Debug;

#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

parameter_types! {
	pub const FellowshipReferendaData: &'static str = r#"[{"index":0,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":10,"deposit1":{"who":"dEmQ58Mi6YKd16XifjaX9jPg13C1HHV1EdeEQqQn3GwLueP","amount":0},"deposit2":{"who":"dEmQ58Mi6YKd16XifjaX9jPg13C1HHV1EdeEQqQn3GwLueP","amount":1000000000000}},{"index":4,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":21,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":20,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":16,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":11,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":14,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":6,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":19,"deposit1":{"who":"dEmQ58Mi6YKd16XifjaX9jPg13C1HHV1EdeEQqQn3GwLueP","amount":0},"deposit2":{"who":"dEmQ58Mi6YKd16XifjaX9jPg13C1HHV1EdeEQqQn3GwLueP","amount":1000000000000}},{"index":15,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":2,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":13,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":5,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":18,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":7,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":22,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":24,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":8,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":1,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":12,"deposit1":{"who":"cYPu6g4apwndq26ryA4gFjW7us5LdcpCnzgkeFyxdgu5aop","amount":0},"deposit2":{"who":"cYPu6g4apwndq26ryA4gFjW7us5LdcpCnzgkeFyxdgu5aop","amount":10000000000000}},{"index":3,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":17,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":25,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":23,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":9,"deposit1":{"who":"dEmQ58Mi6YKd16XifjaX9jPg13C1HHV1EdeEQqQn3GwLueP","amount":0},"deposit2":null}]"#;
	pub const ReferendaData: &'static str = r#"[{"index":0,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000},"deposit2":null},{"index":4,"deposit1":{"who":"gB5r2iWvPCZcUMXxXBfp9UChnRovJ9YwZMrPGDdbxayuqhZ","amount":10000000000000},"deposit2":null},{"index":6,"deposit1":{"who":"g5SajmWUz458LZG1EEX9ybhkQ35nDXr4fg1yjkq5kiVkKcD","amount":10000000000000},"deposit2":{"who":"g5SajmWUz458LZG1EEX9ybhkQ35nDXr4fg1yjkq5kiVkKcD","amount":2500000000000000}},{"index":2,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000},"deposit2":null},{"index":5,"deposit1":{"who":"gB5r2iWvPCZcUMXxXBfp9UChnRovJ9YwZMrPGDdbxayuqhZ","amount":10000000000000},"deposit2":null},{"index":7,"deposit1":{"who":"gB5r2iWvPCZcUMXxXBfp9UChnRovJ9YwZMrPGDdbxayuqhZ","amount":10000000000000},"deposit2":null},{"index":1,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000},"deposit2":null},{"index":3,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000},"deposit2":null}]"#;
}

/// Initial version of storage types.
pub mod v0 {
	use super::*;
	use pallet_referenda::{
		BalanceOf, BoundedCallOf, Config, Deposit, Pallet, PalletsOriginOf, ReferendumIndex,
		ReferendumStatus, ScheduleAddressOf, TallyOf, TrackIdOf,
	};
	// ReferendumStatus and its dependency types referenced from the latest version while staying
	// unchanged. [`super::test::referendum_status_v0()`] checks its immutability between v0 and
	// latest version.

	pub type ReferendumInfoOf<T, I> = ReferendumInfo<
		TrackIdOf<T, I>,
		PalletsOriginOf<T>,
		frame_system::pallet_prelude::BlockNumberFor<T>,
		BoundedCallOf<T, I>,
		BalanceOf<T, I>,
		TallyOf<T, I>,
		<T as frame_system::Config>::AccountId,
		ScheduleAddressOf<T, I>,
	>;

	/// Info regarding a referendum, present or past.
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum ReferendumInfo<
		TrackId: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone,
		RuntimeOrigin: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone,
		Moment: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone + EncodeLike,
		Call: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone,
		Balance: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone,
		Tally: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone,
		AccountId: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone,
		ScheduleAddress: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone,
	> {
		/// Referendum has been submitted and is being voted on.
		Ongoing(
			ReferendumStatus<
				TrackId,
				RuntimeOrigin,
				Moment,
				Call,
				Balance,
				Tally,
				AccountId,
				ScheduleAddress,
			>,
		),
		/// Referendum finished with approval. Submission deposit is held.
		Approved(Moment, Option<Deposit<AccountId, Balance>>, Option<Deposit<AccountId, Balance>>),
		/// Referendum finished with rejection. Submission deposit is held.
		Rejected(Moment, Option<Deposit<AccountId, Balance>>, Option<Deposit<AccountId, Balance>>),
		/// Referendum finished with cancellation. Submission deposit is held.
		Cancelled(Moment, Option<Deposit<AccountId, Balance>>, Option<Deposit<AccountId, Balance>>),
		/// Referendum finished and was never decided. Submission deposit is held.
		TimedOut(Moment, Option<Deposit<AccountId, Balance>>, Option<Deposit<AccountId, Balance>>),
		/// Referendum finished with a kill.
		Killed(Moment),
	}

	#[storage_alias]
	pub type ReferendumInfoFor<T: Config<I>, I: 'static> =
		StorageMap<Pallet<T, I>, Blake2_128Concat, ReferendumIndex, ReferendumInfoOf<T, I>>;
}

pub mod v1 {
	use super::*;
	use pallet_referenda::{
		BalanceOf, Config, Deposit, Pallet, ReferendumIndex, ReferendumInfo, ReferendumInfoFor,
	};
	use sp_runtime::Deserialize;

	/// The log target.
	const TARGET: &'static str = "runtime::referenda::migration::v1";

	#[derive(Debug, Deserialize, Clone)]
	struct ForeignReferendumInfo<AccountId, Balance> {
		index: ReferendumIndex,
		deposit1: Option<ForeignDeposit<AccountId, Balance>>,
		deposit2: Option<ForeignDeposit<AccountId, Balance>>,
	}

	#[derive(Debug, Deserialize, Clone)]
	struct ForeignDeposit<AccountId, Balance> {
		who: AccountId,
		amount: Balance,
	}

	/// Restore ReferendumInfo(Approved|Rejected|Cancelled|TimedOut).
	pub struct RestoreReferendaV1<R: Get<&'static str>, T, I = ()>(PhantomData<(R, T, I)>);
	impl<R: Get<&'static str>, T: Config<I>, I: 'static> OnRuntimeUpgrade
		for RestoreReferendaV1<R, T, I>
	{
		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
			let referendum_count = v0::ReferendumInfoFor::<T, I>::iter().count();
			log::info!(
				target: TARGET,
				"pre-upgrade state contains '{}' referendums.",
				referendum_count
			);
			let infos = v0::ReferendumInfoFor::<T, I>::iter().collect::<Vec<_>>();
			log::info!("pre_upgrade infos: {:?}", infos);
			Ok((referendum_count as u32).encode())
		}

		fn on_runtime_upgrade() -> Weight {
			let result: Vec<ForeignReferendumInfo<T::AccountId, BalanceOf<T, I>>> =
				serde_json::from_str(R::get()).expect("Failed to deserialize JSON");

			let mut weight = T::DbWeight::get().reads(1);

			v0::ReferendumInfoFor::<T, I>::iter().for_each(|(key, value)| {
				let item = result
					.iter()
					.filter(|item| item.index == key)
					.cloned()
					.collect::<Vec<ForeignReferendumInfo<T::AccountId, BalanceOf<T, I>>>>();
				if item.len() != 1 {
					weight.saturating_accrue(T::DbWeight::get().reads(1));
					return;
				}
				let maybe_new_value = match value {
					v0::ReferendumInfo::Ongoing(_) | v0::ReferendumInfo::Killed(_) => None,
					v0::ReferendumInfo::Approved(e, mut s, mut d) => {
						if let Some(a) = &item[0].deposit1 {
							s = Some(Deposit { amount: a.amount, who: a.who.clone() })
						}
						if let Some(a) = &item[0].deposit2 {
							d = Some(Deposit { amount: a.amount, who: a.who.clone() })
						}
						Some(ReferendumInfo::Approved(e, s, d))
					},
					v0::ReferendumInfo::Rejected(e, mut s, mut d) => {
						if let Some(a) = &item[0].deposit1 {
							s = Some(Deposit { amount: a.amount, who: a.who.clone() })
						}
						if let Some(a) = &item[0].deposit2 {
							d = Some(Deposit { amount: a.amount, who: a.who.clone() })
						}
						Some(ReferendumInfo::Rejected(e, s, d))
					},
					v0::ReferendumInfo::Cancelled(e, mut s, mut d) => {
						if let Some(a) = &item[0].deposit1 {
							s = Some(Deposit { amount: a.amount, who: a.who.clone() })
						}
						if let Some(a) = &item[0].deposit2 {
							d = Some(Deposit { amount: a.amount, who: a.who.clone() })
						}
						Some(ReferendumInfo::Cancelled(e, s, d))
					},
					v0::ReferendumInfo::TimedOut(e, mut s, mut d) => {
						if let Some(a) = &item[0].deposit1 {
							s = Some(Deposit { amount: a.amount, who: a.who.clone() })
						}
						if let Some(a) = &item[0].deposit2 {
							d = Some(Deposit { amount: a.amount, who: a.who.clone() })
						}
						Some(ReferendumInfo::TimedOut(e, s, d))
					},
				};
				if let Some(new_value) = maybe_new_value {
					weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
					log::info!(target: TARGET, "migrating referendum #{:?}", &key);
					ReferendumInfoFor::<T, I>::insert(key, new_value);
				} else {
					weight.saturating_accrue(T::DbWeight::get().reads(1));
				}
			});
			StorageVersion::new(1).put::<Pallet<T, I>>();
			weight.saturating_accrue(T::DbWeight::get().writes(1));
			weight
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
			let pre_referendum_count: u32 = Decode::decode(&mut &state[..])
				.expect("failed to decode the state from pre-upgrade.");
			let post_referendum_count = ReferendumInfoFor::<T, I>::iter().count() as u32;
			ensure!(post_referendum_count == pre_referendum_count, "must migrate all referendums.");

			let result: Vec<ForeignReferendumInfo<T::AccountId, BalanceOf<T, I>>> =
				serde_json::from_str(R::get()).expect("Failed to deserialize JSON");
			for item in result {
				let referendum_info = ReferendumInfoFor::<T, I>::get(item.index)
					.expect("failed to decode the state from pre-upgrade.");

				match referendum_info {
					ReferendumInfo::Ongoing(_) | ReferendumInfo::Killed(_) => (),
					ReferendumInfo::Approved(_e, s, d) |
					ReferendumInfo::Rejected(_e, s, d) |
					ReferendumInfo::Cancelled(_e, s, d) |
					ReferendumInfo::TimedOut(_e, s, d) => {
						match (s, item.deposit1) {
							(Some(s), Some(a)) => {
								ensure!(s.amount == a.amount, "amount not equal");
								ensure!(s.who == a.who, "who not equal");
							},
							(None, None) => (),
							_ => return Err(TryRuntimeError::Other("Referenda Data mismatch")),
						}
						match (d, item.deposit2) {
							(Some(d), Some(a)) => {
								ensure!(d.amount == a.amount, "amount not equal");
								ensure!(d.who == a.who, "who not equal");
							},
							(None, None) => (),
							_ => return Err(TryRuntimeError::Other("Referenda Data mismatch")),
						}
						()
					},
				};
			}

			log::info!(target: TARGET, "migrated all referendums.");
			Ok(())
		}
	}
}

pub mod slpx_migrates_whitelist {
	use super::*;
	use bifrost_slpx::types::SupportChain;
	use sp_core::crypto::Ss58Codec;

	pub struct UpdateWhitelist;
	impl OnRuntimeUpgrade for UpdateWhitelist {
		fn on_runtime_upgrade() -> Weight {
			pallet_xcm::migrations::migrate_to_v1::<Runtime, pallet_xcm::Pallet<Runtime>>();
			let new_whitelist: BoundedVec<AccountId, ConstU32<10>> = vec![
				AccountId::from_ss58check("gWEvf2EDMzxR7JHyrEHXf3nqxKLGvHaFbk7HUkJnNPUxDts")
					.unwrap(),
				AccountId::from_ss58check("bpRBE4rWcBJqqN2ESts5LefuvLonBeZ4r2YBpfuuxvMxoea")
					.unwrap(),
			]
			.try_into()
			.unwrap();
			bifrost_slpx::WhitelistAccountId::<Runtime>::insert(
				SupportChain::Moonbeam,
				new_whitelist,
			);

			let new_whitelist: BoundedVec<AccountId, ConstU32<10>> = vec![
				AccountId::from_ss58check("g96o4GVpsAop1MJiArnmUYtXUjEisfkbfcpsuqmXrS28MEr")
					.unwrap(),
				AccountId::from_ss58check("dZWAvaUWbPN1oKbWMkeDWHBceX8RqP4CMwmi1trq9uf5pBn")
					.unwrap(),
			]
			.try_into()
			.unwrap();
			bifrost_slpx::WhitelistAccountId::<Runtime>::insert(SupportChain::Astar, new_whitelist);

			Weight::from(<Runtime as frame_system::Config>::DbWeight::get().writes(2u64))
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
			let whitelist =
				bifrost_slpx::WhitelistAccountId::<Runtime>::get(SupportChain::Moonbeam);
			let new_whitelist: BoundedVec<AccountId, ConstU32<10>> = vec![
				AccountId::from_ss58check("gWEvf2EDMzxR7JHyrEHXf3nqxKLGvHaFbk7HUkJnNPUxDts")
					.unwrap(),
				AccountId::from_ss58check("bpRBE4rWcBJqqN2ESts5LefuvLonBeZ4r2YBpfuuxvMxoea")
					.unwrap(),
			]
			.try_into()
			.unwrap();
			assert_eq!(whitelist, new_whitelist);

			let whitelist = bifrost_slpx::WhitelistAccountId::<Runtime>::get(SupportChain::Astar);
			let new_whitelist: BoundedVec<AccountId, ConstU32<10>> = vec![
				AccountId::from_ss58check("g96o4GVpsAop1MJiArnmUYtXUjEisfkbfcpsuqmXrS28MEr")
					.unwrap(),
				AccountId::from_ss58check("dZWAvaUWbPN1oKbWMkeDWHBceX8RqP4CMwmi1trq9uf5pBn")
					.unwrap(),
			]
			.try_into()
			.unwrap();
			assert_eq!(whitelist, new_whitelist);

			Ok(())
		}
	}
}

pub mod opengov {
	use super::*;
	use pallet_ranked_collective::{Config, IdToIndex, IndexToId};
	use sp_core::crypto::Ss58Codec;

	pub struct RankedCollectiveV1<T, I = ()>(PhantomData<(T, I)>);
	impl<T: Config<I>, I: 'static> OnRuntimeUpgrade for RankedCollectiveV1<T, I>
	where
		AccountId:
			Ss58Codec + parity_scale_codec::EncodeLike<<T as frame_system::Config>::AccountId>,
	{
		fn on_runtime_upgrade() -> Weight {
			let remove_member =
				AccountId::from_ss58check("cYPu6g4apwndq26ryA4gFjW7us5LdcpCnzgkeFyxdgu5aop")
					.unwrap();
			let demote_member1 =
				AccountId::from_ss58check("fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw")
					.unwrap();
			let demote_member2 =
				AccountId::from_ss58check("fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu")
					.unwrap();
			IdToIndex::<T, I>::remove(5, demote_member1.clone());
			IdToIndex::<T, I>::remove(5, demote_member2.clone());
			IdToIndex::<T, I>::remove(6, demote_member1.clone());
			IdToIndex::<T, I>::remove(6, demote_member2.clone());
			IdToIndex::<T, I>::remove(0, remove_member.clone());
			IdToIndex::<T, I>::remove(1, remove_member.clone());
			IdToIndex::<T, I>::remove(2, remove_member.clone());
			IdToIndex::<T, I>::remove(3, remove_member.clone());
			IdToIndex::<T, I>::remove(4, remove_member.clone());
			IdToIndex::<T, I>::remove(5, remove_member.clone());
			IdToIndex::<T, I>::remove(6, remove_member.clone());

			IndexToId::<T, I>::remove(2, 14);
			IndexToId::<T, I>::remove(3, 10);
			IndexToId::<T, I>::remove(4, 7);

			IndexToId::<T, I>::remove(5, 2);
			IndexToId::<T, I>::remove(5, 3);
			IndexToId::<T, I>::remove(5, 4);
			IndexToId::<T, I>::remove(6, 2);
			IndexToId::<T, I>::remove(6, 3);
			IndexToId::<T, I>::remove(6, 4);
			Weight::from(<Runtime as frame_system::Config>::DbWeight::get().writes(20u64))
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
			let rank6_member =
				AccountId::from_ss58check("hJmectFjn7CCEQL1tKDxvboA1i9hcfTyUrLuW3xjDqRgxmm")
					.unwrap();
			let remove_member =
				AccountId::from_ss58check("cYPu6g4apwndq26ryA4gFjW7us5LdcpCnzgkeFyxdgu5aop")
					.unwrap();
			let demote_member1 =
				AccountId::from_ss58check("fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw")
					.unwrap();
			let demote_member2 =
				AccountId::from_ss58check("fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu")
					.unwrap();
			assert_eq!(IdToIndex::<T, I>::get(5, rank6_member.clone()), Some(0));
			assert_eq!(IdToIndex::<T, I>::get(6, rank6_member.clone()), Some(0));
			assert_eq!(IndexToId::<T, I>::get(5, 2), None);
			assert_eq!(IndexToId::<T, I>::get(5, 3), None);
			assert_eq!(IndexToId::<T, I>::get(5, 4), None);
			assert_eq!(IndexToId::<T, I>::get(6, 2), None);
			assert_eq!(IndexToId::<T, I>::get(6, 3), None);
			assert_eq!(IndexToId::<T, I>::get(6, 4), None);

			assert_eq!(IndexToId::<T, I>::get(4, 7), None);
			assert_eq!(IndexToId::<T, I>::get(4, 8), None);
			assert_eq!(IndexToId::<T, I>::get(3, 10), None);
			assert_eq!(IndexToId::<T, I>::get(3, 11), None);
			assert_eq!(IndexToId::<T, I>::get(2, 14), None);
			assert_eq!(IndexToId::<T, I>::get(1, 19), None);
			assert_eq!(IndexToId::<T, I>::get(0, 19), None);

			let deserialize_to_account_id32 = |account_id: T::AccountId| -> AccountId {
				let encoded = account_id.encode();
				AccountId::decode(&mut &encoded[..]).expect("Failed to decode AccountId")
			};
			assert_eq!(
				IndexToId::<T, I>::get(5, 0).map(deserialize_to_account_id32),
				Some(rank6_member)
			);
			assert_eq!(IdToIndex::<T, I>::get(5, demote_member1.clone()), None);
			assert_eq!(IdToIndex::<T, I>::get(6, demote_member1.clone()), None);
			assert_eq!(IdToIndex::<T, I>::get(5, demote_member2.clone()), None);
			assert_eq!(IdToIndex::<T, I>::get(6, demote_member2.clone()), None);

			assert_eq!(IdToIndex::<T, I>::get(0, remove_member.clone()), None);
			assert_eq!(IdToIndex::<T, I>::get(1, remove_member.clone()), None);
			assert_eq!(IdToIndex::<T, I>::get(2, remove_member.clone()), None);
			assert_eq!(IdToIndex::<T, I>::get(3, remove_member.clone()), None);
			assert_eq!(IdToIndex::<T, I>::get(4, remove_member.clone()), None);
			assert_eq!(IdToIndex::<T, I>::get(5, remove_member.clone()), None);
			assert_eq!(IdToIndex::<T, I>::get(6, remove_member.clone()), None);

			Ok(())
		}
	}
}

pub mod genesis_evm_storage {
	use crate::{Runtime, Weight};
	use frame_support::traits::OnRuntimeUpgrade;
	use pallet_dynamic_fee::MinGasPrice;
	use pallet_evm_chain_id::ChainId;
	use primitive_types::U256;

	pub struct GenesisEVMStorage;

	impl OnRuntimeUpgrade for GenesisEVMStorage {
		fn on_runtime_upgrade() -> Weight {
			let evm_id: u64 = 996u64;
			let min_gas_fee: U256 = U256::from(560174200u64);
			ChainId::<Runtime>::put(evm_id);
			MinGasPrice::<Runtime>::put(min_gas_fee);
			<Runtime as frame_system::Config>::DbWeight::get().reads_writes(0, 2)
		}
	}
}
