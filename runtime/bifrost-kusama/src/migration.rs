use super::*;
use frame_support::{pallet_prelude::*, storage_alias, traits::OnRuntimeUpgrade};
use log;
use parity_scale_codec::{Decode, Encode, EncodeLike, MaxEncodedLen};
use sp_std::fmt::Debug;

#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

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
	#[cfg(test)]
	pub(super) use super::{ReferendumStatus, ReferendumStatusOf};

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
		BalanceOf, Config, Pallet, ReferendumIndex, ReferendumInfo, ReferendumInfoFor,
		ReferendumInfoOf,
	};

	/// The log target.
	const TARGET: &'static str = "runtime::referenda::migration::v1";

	/// Transforms a submission deposit of ReferendumInfo(Approved|Rejected|Cancelled|TimedOut) to
	/// optional value, making it refundable.
	pub struct MigrateV0ToV1<T, I = ()>(PhantomData<(T, I)>);
	impl<T: Config<I>, I: 'static> OnRuntimeUpgrade for MigrateV0ToV1<T, I> {
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
			use pallet_referenda::Deposit;
			use scale_info::prelude::string::String;
			use sp_runtime::Deserialize;
			#[derive(Debug, Deserialize, Clone)]
			struct MyItem<AccountId, Balance> {
				index: ReferendumIndex,
				item: String,
				moment: u64,
				deposit1: Option<ForeignDeposit<AccountId, Balance>>,
				deposit2: Option<ForeignDeposit<AccountId, Balance>>,
			}

			#[derive(Debug, Deserialize, Clone)]
			struct ForeignDeposit<AccountId, Balance> {
				who: AccountId,
				amount: Balance,
			}
			let json_data = r#"[{"index":0,"item":"approved","moment":4655264,"deposit1":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":0},"deposit2":null},{"index":10,"item":"approved","moment":5005706,"deposit1":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":0},"deposit2":null},{"index":4,"item":"approved","moment":4711806,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":21,"item":"approved","moment":5242084,"deposit1":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":0},"deposit2":{"who":"hJmectFjn7CCEQL1tKDxvboA1i9hcfTyUrLuW3xjDqRgxmm","amount":10000000000000}},{"index":28,"item":"approved","moment":5364637,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":20,"item":"rejected","moment":5256350,"deposit1":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":30,"item":"approved","moment":5403372,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":39,"item":"approved","moment":5627214,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":38,"item":"approved","moment":5578836,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":34,"item":"approved","moment":5483685,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":16,"item":"approved","moment":5099545,"deposit1":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":0},"deposit2":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":10000000000000}},{"index":11,"item":"approved","moment":5014894,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":14,"item":"approved","moment":5065068,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":6,"item":"approved","moment":4763470,"deposit1":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":0},"deposit2":null},{"index":19,"item":"approved","moment":5202672,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":1000000000000}},{"index":35,"item":"approved","moment":5483693,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":36,"item":"approved","moment":5522990,"deposit1":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":0},"deposit2":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":10000000000000}},{"index":31,"item":"approved","moment":5426963,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":33,"item":"approved","moment":5464554,"deposit1":{"who":"dEmQ58Mi6YKd16XifjaX9jPg13C1HHV1EdeEQqQn3GwLueP","amount":0},"deposit2":{"who":"dEmQ58Mi6YKd16XifjaX9jPg13C1HHV1EdeEQqQn3GwLueP","amount":1000000000000}},{"index":41,"item":"approved","moment":5705997,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":15,"item":"approved","moment":5077671,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":1000000000000}},{"index":40,"item":"approved","moment":5671841,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":2,"item":"rejected","moment":4726229,"deposit1":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":0},"deposit2":null},{"index":13,"item":"approved","moment":5065120,"deposit1":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":0},"deposit2":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":10000000000000}},{"index":32,"item":"approved","moment":5438975,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":27,"item":"approved","moment":5330661,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":29,"item":"approved","moment":5366509,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":42,"item":"approved","moment":5718845,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":43,"item":"approved","moment":5722849,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":5,"item":"approved","moment":4760550,"deposit1":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":0},"deposit2":null},{"index":18,"item":"approved","moment":5192305,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":7,"item":"approved","moment":4852971,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":26,"item":"approved","moment":5309065,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":22,"item":"approved","moment":5261989,"deposit1":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":24,"item":"approved","moment":5287150,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":8,"item":"approved","moment":4860211,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":1,"item":"approved","moment":4670644,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null},{"index":12,"item":"approved","moment":5022210,"deposit1":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":0},"deposit2":{"who":"ddeADYcZ13GmFSR44KjMiRixvq8Hbdh9HW9wPRX4gupWDDb","amount":10000000000000}},{"index":3,"item":"approved","moment":4704466,"deposit1":{"who":"fXznm8JzrUuyEijnyy8M2tfdFQeot2bUrERe3ZK9FwTaZZw","amount":0},"deposit2":null},{"index":17,"item":"approved","moment":5119363,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":1000000000000}},{"index":25,"item":"approved","moment":5299127,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":1000000000000}},{"index":23,"item":"approved","moment":5264569,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":37,"item":"approved","moment":5530454,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":10000000000000}},{"index":9,"item":"approved","moment":4951386,"deposit1":{"who":"fAGgdvAYwqCwpt3Wda1mzpACnNyESbfgfgvm1RLSudBsUEu","amount":0},"deposit2":null}]"#;

			let result: Vec<MyItem<T::AccountId, BalanceOf<T, I>>> =
				serde_json::from_str(json_data).expect("Failed to deserialize JSON");

			let mut weight = T::DbWeight::get().reads(1);

			v0::ReferendumInfoFor::<T, I>::iter().for_each(|(key, value)| {
				let item = result
					.iter()
					.filter(|item| item.index == key)
					.cloned()
					.collect::<Vec<MyItem<T::AccountId, BalanceOf<T, I>>>>();
				let maybe_new_value = match value {
					v0::ReferendumInfo::Ongoing(_) | v0::ReferendumInfo::Killed(_) => None,
					v0::ReferendumInfo::Approved(e, mut s, mut d) => {
						if let Some(a) = &item[0].deposit1 {
							s = Some(Deposit { amount: a.amount, who: a.who.clone() })
						}
						if let Some(a) = &item[0].deposit2 {
							d = Some(Deposit { amount: a.amount, who: a.who.clone() })
						}
						Some(ReferendumInfo::Rejected(e, s, d))
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
					v0::ReferendumInfo::Cancelled(e, s, d) =>
						Some(ReferendumInfo::Cancelled(e, s, d)),
					v0::ReferendumInfo::TimedOut(e, s, d) =>
						Some(ReferendumInfo::TimedOut(e, s, d)),
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
			log::info!(target: TARGET, "migrated all referendums.");
			let infos = ReferendumInfoFor::<T, I>::iter().collect::<Vec<_>>();
			log::info!("post_upgrade infos: {:?}", infos);
			Ok(())
		}
	}
}
