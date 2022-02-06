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

//! # Vesting Pallet
//!
//! - [`Config`]
//! - [`Call`]
//!
//! ## Overview
//!
//! A simple pallet providing a means of placing a linear curve on an account's locked balance. This
//! pallet ensures that there is a lock in place preventing the balance to drop below the *unvested*
//! amount for any reason other than transaction fee payment.
//!
//! As the amount vested increases over time, the amount unvested reduces. However, locks remain in
//! place and explicit action is needed on behalf of the user to ensure that the amount locked is
//! equivalent to the amount remaining to be vested. This is done through a dispatchable function,
//! either `vest` (in typical case where the sender is calling on their own behalf) or `vest_other`
//! in case the sender is calling on another account's behalf.
//!
//! ## Interface
//!
//! This pallet implements the `VestingSchedule` trait.
//!
//! ### Dispatchable Functions
//!
//! - `vest` - Update the lock, reducing it in line with the amount "vested" so far.
//! - `vest_other` - Update the lock of another account, reducing it in line with the amount
//!   "vested" so far.

#![cfg_attr(not(feature = "std"), no_std)]

mod benchmarking;
pub mod weights;

use codec::{Decode, Encode};
use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{
		Currency, ExistenceRequirement, Get, LockIdentifier, LockableCurrency, VestingSchedule,
		WithdrawReasons,
	},
};
use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{
		AtLeast32BitUnsigned, Convert, MaybeSerializeDeserialize, Saturating, StaticLookup, Zero,
	},
	RuntimeDebug,
};
use sp_std::{fmt::Debug, prelude::*};
pub use weights::WeightInfo;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type MaxLocksOf<T> =
	<<T as Config>::Currency as LockableCurrency<<T as frame_system::Config>::AccountId>>::MaxLocks;

const VESTING_ID: LockIdentifier = *b"vesting ";

/// Struct to encode the vesting schedule of an individual account.
#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct VestingInfo<Balance, BlockNumber> {
	/// Locked amount at genesis.
	pub locked: Balance,
	/// Amount that gets unlocked every block after `starting_block`.
	pub per_block: Balance,
	/// Starting block for unlocking(vesting).
	/// It's relative position to the pallet vesting starting block.
	pub starting_block: BlockNumber,
}

impl<Balance: AtLeast32BitUnsigned + Copy, BlockNumber: AtLeast32BitUnsigned + Copy>
	VestingInfo<Balance, BlockNumber>
{
	/// Amount locked at block `n`.
	pub fn locked_at<BlockNumberToBalance: Convert<BlockNumber, Balance>>(
		&self,
		n: BlockNumber,
		start_at: Option<BlockNumber>,
	) -> Balance {
		// Number of blocks that count toward vesting
		let vested_block_count = match start_at {
			Some(st) if st < n => n.saturating_sub(st),
			_ => return self.locked,
		};
		let vested_block_count = BlockNumberToBalance::convert(vested_block_count);
		// Return amount that is still locked in vesting
		let maybe_balance = vested_block_count.checked_mul(&self.per_block);
		if let Some(balance) = maybe_balance {
			self.locked.saturating_sub(balance)
		} else {
			Zero::zero()
		}
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The currency trait.
		type Currency: LockableCurrency<Self::AccountId>;

		/// Convert the block number into a balance.
		type BlockNumberToBalance: Convert<Self::BlockNumber, BalanceOf<Self>>;

		/// The minimum amount transferred to call `vested_transfer`.
		#[pallet::constant]
		type MinVestedTransfer: Get<BalanceOf<Self>>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	/// Start at
	#[pallet::storage]
	#[pallet::getter(fn vesting_start_at)]
	pub(super) type VestingStartAt<T: Config> = StorageValue<_, T::BlockNumber>;

	/// Cliff vesting
	#[pallet::storage]
	#[pallet::getter(fn cliffs)]
	pub(super) type Cliff<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::BlockNumber>;

	/// Information regarding the vesting of a given account.
	#[pallet::storage]
	#[pallet::getter(fn vesting)]
	pub type Vesting<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, VestingInfo<BalanceOf<T>, T::BlockNumber>>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub vesting: Vec<(T::AccountId, T::BlockNumber, T::BlockNumber, BalanceOf<T>)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { vesting: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			use sp_runtime::traits::Saturating;

			// Generate initial vesting configuration
			// * who - Account which we are generating vesting configuration for
			// * begin - Block when the account will start to vest
			// * length - Number of blocks from `begin` until fully vested
			// * liquid - Number of units which can be spent before vesting begins
			for &(ref who, begin, length, liquid) in self.vesting.iter() {
				let balance = T::Currency::free_balance(who);
				assert!(!balance.is_zero(), "Currencies must be init'd before vesting");
				// Total genesis `balance` minus `liquid` equals funds locked for vesting
				let locked = balance.saturating_sub(liquid);
				let length_as_balance = T::BlockNumberToBalance::convert(length);
				let per_block = locked / length_as_balance.max(sp_runtime::traits::One::one());

				Vesting::<T>::insert(who, VestingInfo { locked, per_block, starting_block: begin });
				let reasons = WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE;
				T::Currency::set_lock(VESTING_ID, who, locked, reasons);
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The amount vested has been updated. This could indicate more funds are available. The
		/// balance given is the amount which is left unvested (and thus locked).
		/// \[account, unvested\]
		VestingUpdated(T::AccountId, BalanceOf<T>),
		/// An \[account\] has become fully vested. No further vesting can happen.
		VestingCompleted(T::AccountId),
	}

	/// Error for the vesting pallet.
	#[pallet::error]
	pub enum Error<T> {
		/// The account given is not vesting.
		NotVesting,
		/// An existing vesting schedule already exists for this account that cannot be clobbered.
		ExistingVestingSchedule,
		/// Amount being transferred is too low to create a vesting schedule.
		AmountLow,
		/// change to the same per_block param
		SamePerBlock,
		/// VestingStartAt storage is not set
		VestingStartAtNotSet,
		/// Wrong amount
		WrongLockedAmount,
		/// Wrong vesting during cliff period
		WrongCliffVesting,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Unlock any vested funds of the sender account.
		///
		/// The dispatch origin for this call must be _Signed_ and the sender must have funds still
		/// locked under this pallet.
		///
		/// Emits either `VestingCompleted` or `VestingUpdated`.
		///
		/// # <weight>
		/// - `O(1)`.
		/// - DbWeight: 2 Reads, 2 Writes
		///     - Reads: Vesting Storage, Balances Locks, [Sender Account]
		///     - Writes: Vesting Storage, Balances Locks, [Sender Account]
		/// # </weight>
		#[pallet::weight(T::WeightInfo::vest_locked(MaxLocksOf::<T>::get())
		.max(T::WeightInfo::vest_unlocked(MaxLocksOf::<T>::get()))
		)]
		pub fn vest(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::check_cliff(who.clone())?;
			Self::update_lock(who)
		}

		/// Unlock any vested funds of a `target` account.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// - `target`: The account whose vested funds should be unlocked. Must have funds still
		/// locked under this pallet.
		///
		/// Emits either `VestingCompleted` or `VestingUpdated`.
		///
		/// # <weight>
		/// - `O(1)`.
		/// - DbWeight: 3 Reads, 3 Writes
		///     - Reads: Vesting Storage, Balances Locks, Target Account
		///     - Writes: Vesting Storage, Balances Locks, Target Account
		/// # </weight>
		#[pallet::weight(T::WeightInfo::vest_other_locked(MaxLocksOf::<T>::get())
		.max(T::WeightInfo::vest_other_unlocked(MaxLocksOf::<T>::get()))
		)]
		pub fn vest_other(
			origin: OriginFor<T>,
			target: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			ensure_signed(origin)?;
			let who = T::Lookup::lookup(target)?;
			Self::check_cliff(who.clone())?;
			Self::update_lock(who)
		}

		/// Create a vested transfer.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// - `target`: The account that should be transferred the vested funds.
		/// - `amount`: The amount of funds to transfer and will be vested.
		/// - `schedule`: The vesting schedule attached to the transfer.
		///
		/// Emits `VestingCreated`.
		///
		/// # <weight>
		/// - `O(1)`.
		/// - DbWeight: 3 Reads, 3 Writes
		///     - Reads: Vesting Storage, Balances Locks, Target Account, [Sender Account]
		///     - Writes: Vesting Storage, Balances Locks, Target Account, [Sender Account]
		/// # </weight>
		#[pallet::weight(T::WeightInfo::vested_transfer(MaxLocksOf::<T>::get()))]
		pub fn vested_transfer(
			origin: OriginFor<T>,
			target: <T::Lookup as StaticLookup>::Source,
			schedule: VestingInfo<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			let transactor = ensure_signed(origin)?;
			ensure!(schedule.locked >= T::MinVestedTransfer::get(), Error::<T>::AmountLow);

			let who = T::Lookup::lookup(target)?;
			ensure!(!Vesting::<T>::contains_key(&who), Error::<T>::ExistingVestingSchedule);

			T::Currency::transfer(
				&transactor,
				&who,
				schedule.locked,
				ExistenceRequirement::AllowDeath,
			)?;

			Self::add_vesting_schedule(
				&who,
				schedule.locked,
				schedule.per_block,
				schedule.starting_block,
			)
			.expect("user does not have an existing vesting schedule; q.e.d.");

			Ok(())
		}

		/// Force a vested transfer.
		///
		/// The dispatch origin for this call must be _Root_.
		///
		/// - `source`: The account whose funds should be transferred.
		/// - `target`: The account that should be transferred the vested funds.
		/// - `amount`: The amount of funds to transfer and will be vested.
		/// - `schedule`: The vesting schedule attached to the transfer.
		///
		/// Emits `VestingCreated`.
		///
		/// # <weight>
		/// - `O(1)`.
		/// - DbWeight: 4 Reads, 4 Writes
		///     - Reads: Vesting Storage, Balances Locks, Target Account, Source Account
		///     - Writes: Vesting Storage, Balances Locks, Target Account, Source Account
		/// # </weight>
		#[pallet::weight(T::WeightInfo::force_vested_transfer(MaxLocksOf::<T>::get()))]
		pub fn force_vested_transfer(
			origin: OriginFor<T>,
			source: <T::Lookup as StaticLookup>::Source,
			target: <T::Lookup as StaticLookup>::Source,
			schedule: VestingInfo<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(schedule.locked >= T::MinVestedTransfer::get(), Error::<T>::AmountLow);

			let target = T::Lookup::lookup(target)?;
			let source = T::Lookup::lookup(source)?;
			ensure!(!Vesting::<T>::contains_key(&target), Error::<T>::ExistingVestingSchedule);

			T::Currency::transfer(
				&source,
				&target,
				schedule.locked,
				ExistenceRequirement::AllowDeath,
			)?;

			Self::add_vesting_schedule(
				&target,
				schedule.locked,
				schedule.per_block,
				schedule.starting_block,
			)
			.expect("user does not have an existing vesting schedule; q.e.d.");

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn force_set_vested(
			origin: OriginFor<T>,
			source: <T::Lookup as StaticLookup>::Source,
			target: <T::Lookup as StaticLookup>::Source,
			schedule: VestingInfo<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(schedule.locked >= T::MinVestedTransfer::get(), Error::<T>::AmountLow);

			let target = T::Lookup::lookup(target)?;
			let source = T::Lookup::lookup(source)?;

			T::Currency::remove_lock(VESTING_ID, &target);

			let (from, to, value) = if schedule.locked > T::Currency::free_balance(&target) {
				(&source, &target, schedule.locked - T::Currency::free_balance(&target))
			} else {
				(&target, &source, T::Currency::free_balance(&target) - schedule.locked)
			};

			T::Currency::transfer(&from, &to, value, ExistenceRequirement::AllowDeath)?;

			Vesting::<T>::insert(target.clone(), schedule);
			let res = Self::update_lock(target.clone());
			debug_assert!(res.is_ok());

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn init_vesting_start_at(
			origin: OriginFor<T>,
			vesting_start_at: T::BlockNumber,
		) -> DispatchResult {
			ensure_root(origin)?;

			VestingStartAt::<T>::put(vesting_start_at);

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_vesting_per_block(
			origin: OriginFor<T>,
			target: <T::Lookup as StaticLookup>::Source,
			per_block: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let target = T::Lookup::lookup(target)?;

			Self::update_lock(target.clone())?;
			let vesting = Self::vesting(&target).ok_or(Error::<T>::NotVesting)?;

			ensure!(vesting.per_block != per_block, Error::<T>::SamePerBlock);

			let absolute_start =
				VestingStartAt::<T>::get().ok_or(Error::<T>::VestingStartAtNotSet)?;
			let now = <frame_system::Pallet<T>>::block_number();

			let old_start_at = absolute_start.saturating_add(vesting.starting_block);
			let remained_vesting =
				vesting.locked_at::<T::BlockNumberToBalance>(now, Some(old_start_at));

			ensure!(remained_vesting <= vesting.locked, Error::<T>::WrongLockedAmount);

			let mut new_start_offset = vesting.starting_block;
			if now > old_start_at {
				new_start_offset = now - absolute_start;
			}

			Vesting::<T>::mutate_exists(&target, |info| {
				if let Some(ref mut vesting_info) = info {
					vesting_info.locked = remained_vesting;
					vesting_info.per_block = per_block;
					vesting_info.starting_block = new_start_offset;
				}
			});

			Self::update_lock(target)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn force_set_cliff(
			origin: OriginFor<T>,
			target: <T::Lookup as StaticLookup>::Source,
			cliff_block: T::BlockNumber,
		) -> DispatchResult {
			ensure_root(origin)?;

			let target = T::Lookup::lookup(target)?;
			Cliff::<T>::insert(target.clone(), cliff_block);

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// (Re)set or remove the pallet's currency lock on `who`'s account in accordance with their
	/// current unvested amount.
	fn update_lock(who: T::AccountId) -> DispatchResult {
		let vesting = Self::vesting(&who).ok_or(Error::<T>::NotVesting)?;
		let now = <frame_system::Pallet<T>>::block_number();

		let start_at = Self::vesting_start_at().map(|st| st.saturating_add(vesting.starting_block));

		let locked_now = vesting.locked_at::<T::BlockNumberToBalance>(now, start_at);

		if locked_now.is_zero() {
			T::Currency::remove_lock(VESTING_ID, &who);
			Vesting::<T>::remove(&who);
			Self::deposit_event(Event::<T>::VestingCompleted(who));
		} else {
			let reasons = WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE;
			T::Currency::set_lock(VESTING_ID, &who, locked_now, reasons);
			Self::deposit_event(Event::<T>::VestingUpdated(who, locked_now));
		}
		Ok(())
	}

	fn check_cliff(who: T::AccountId) -> DispatchResult {
		if let Some(cliff_block) = Cliff::<T>::get(who.clone()) {
			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(cliff_block < now, Error::<T>::WrongCliffVesting);
			Cliff::<T>::remove(who);
		};

		Ok(())
	}
}

impl<T: Config> VestingSchedule<T::AccountId> for Pallet<T>
where
	BalanceOf<T>: MaybeSerializeDeserialize + Debug,
{
	type Currency = T::Currency;
	type Moment = T::BlockNumber;

	/// Get the amount that is currently being vested and cannot be transferred out of this account.
	fn vesting_balance(who: &T::AccountId) -> Option<BalanceOf<T>> {
		if let Some(v) = Self::vesting(who) {
			let now = <frame_system::Pallet<T>>::block_number();
			let start_at = Self::vesting_start_at().map(|st| st.saturating_add(v.starting_block));

			let locked_now = v.locked_at::<T::BlockNumberToBalance>(now, start_at);

			Some(T::Currency::free_balance(who).min(locked_now))
		} else {
			None
		}
	}

	/// Adds a vesting schedule to a given account.
	///
	/// If there already exists a vesting schedule for the given account, an `Err` is returned
	/// and nothing is updated.
	///
	/// On success, a linearly reducing amount of funds will be locked. In order to realise any
	/// reduction of the lock over time as it diminishes, the account owner must use `vest` or
	/// `vest_other`.
	///
	/// Is a no-op if the amount to be vested is zero.
	fn add_vesting_schedule(
		who: &T::AccountId,
		locked: BalanceOf<T>,
		per_block: BalanceOf<T>,
		starting_block: T::BlockNumber,
	) -> DispatchResult {
		if locked.is_zero() {
			return Ok(());
		}
		if Vesting::<T>::contains_key(who) {
			Err(Error::<T>::ExistingVestingSchedule)?
		}
		let vesting_schedule = VestingInfo { locked, per_block, starting_block };
		Vesting::<T>::insert(who, vesting_schedule);
		// it can't fail, but even if somehow it did, we don't really care.
		let res = Self::update_lock(who.clone());
		debug_assert!(res.is_ok());
		Ok(())
	}

	// Ensure we can call `add_vesting_schedule` without error. This should always
	// be called prior to `add_vesting_schedule`.
	fn can_add_vesting_schedule(
		_who: &T::AccountId,
		_locked: BalanceOf<T>,
		_per_block: BalanceOf<T>,
		_starting_block: T::BlockNumber,
	) -> DispatchResult {
		Ok(())
	}

	/// Remove a vesting schedule for a given account.
	fn remove_vesting_schedule(who: &T::AccountId, _schedule_index: u32) -> DispatchResult {
		Vesting::<T>::remove(who);
		// it can't fail, but even if somehow it did, we don't really care.
		Self::update_lock(who.clone())
	}
}

#[cfg(test)]
mod tests {
	use frame_support::{assert_noop, assert_ok, parameter_types};
	use frame_system::RawOrigin;
	use sp_core::H256;
	use sp_runtime::{
		testing::Header,
		traits::{BadOrigin, BlakeTwo256, Identity, IdentityLookup},
	};

	use super::*;
	use crate as pallet_vesting;

	type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
	type Block = frame_system::mocking::MockBlock<Test>;

	frame_support::construct_runtime!(
		pub enum Test where
			Block = Block,
			NodeBlock = Block,
			UncheckedExtrinsic = UncheckedExtrinsic,
		{
			System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
			Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
			Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>},
		}
	);

	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub BlockWeights: frame_system::limits::BlockWeights =
			frame_system::limits::BlockWeights::simple_max(1024);
	}
	impl frame_system::Config for Test {
		type AccountData = pallet_balances::AccountData<u64>;
		type AccountId = u64;
		type BaseCallFilter = frame_support::traits::Everything;
		type BlockHashCount = BlockHashCount;
		type BlockLength = ();
		type BlockNumber = u64;
		type BlockWeights = ();
		type Call = Call;
		type DbWeight = ();
		type Event = Event;
		type Hash = H256;
		type Hashing = BlakeTwo256;
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
		type MaxConsumers = frame_support::traits::ConstU32<16>;
	}
	parameter_types! {
		pub const MaxLocks: u32 = 10;
	}
	impl pallet_balances::Config for Test {
		type AccountStore = System;
		type Balance = u64;
		type DustRemoval = ();
		type Event = Event;
		type ExistentialDeposit = ExistentialDeposit;
		type MaxLocks = MaxLocks;
		type MaxReserves = ();
		type ReserveIdentifier = [u8; 8];
		type WeightInfo = ();
	}
	parameter_types! {
		pub const MinVestedTransfer: u64 = 256 * 2;
		pub static ExistentialDeposit: u64 = 0;
	}
	impl Config for Test {
		type BlockNumberToBalance = Identity;
		type Currency = Balances;
		type Event = Event;
		type MinVestedTransfer = MinVestedTransfer;
		type WeightInfo = ();
	}

	pub struct ExtBuilder {
		existential_deposit: u64,
	}
	impl Default for ExtBuilder {
		fn default() -> Self {
			Self { existential_deposit: 1 }
		}
	}
	impl ExtBuilder {
		pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
			self.existential_deposit = existential_deposit;
			self
		}

		pub fn build(self) -> sp_io::TestExternalities {
			EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
			let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
			pallet_balances::GenesisConfig::<Test> {
				balances: vec![
					(1, 10 * self.existential_deposit),
					(2, 20 * self.existential_deposit),
					(3, 30 * self.existential_deposit),
					(4, 40 * self.existential_deposit),
					(12, 10 * self.existential_deposit),
				],
			}
			.assimilate_storage(&mut t)
			.unwrap();
			pallet_vesting::GenesisConfig::<Test> {
				vesting: vec![
					(1, 0, 10, 5 * self.existential_deposit),
					(2, 10, 20, 0),
					(12, 10, 20, 5 * self.existential_deposit),
				],
			}
			.assimilate_storage(&mut t)
			.unwrap();
			let mut ext = sp_io::TestExternalities::new(t);
			ext.execute_with(|| System::set_block_number(1));
			ext
		}
	}

	#[test]
	fn check_vesting_status() {
		ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
			let user1_free_balance = Balances::free_balance(&1);
			let user2_free_balance = Balances::free_balance(&2);
			let user12_free_balance = Balances::free_balance(&12);
			assert_eq!(user1_free_balance, 256 * 10); // Account 1 has free balance
			assert_eq!(user2_free_balance, 256 * 20); // Account 2 has free balance
			assert_eq!(user12_free_balance, 256 * 10); // Account 12 has free balance
			let user1_vesting_schedule = VestingInfo {
				locked: 256 * 5,
				per_block: 128, // Vesting over 10 blocks
				starting_block: 0,
			};
			let user2_vesting_schedule = VestingInfo {
				locked: 256 * 20,
				per_block: 256, // Vesting over 20 blocks
				starting_block: 10,
			};
			let user12_vesting_schedule = VestingInfo {
				locked: 256 * 5,
				per_block: 64, // Vesting over 20 blocks
				starting_block: 10,
			};
			assert_eq!(Vesting::vesting(&1), Some(user1_vesting_schedule)); // Account 1 has a vesting schedule
			assert_eq!(Vesting::vesting(&2), Some(user2_vesting_schedule)); // Account 2 has a vesting schedule
			assert_eq!(Vesting::vesting(&12), Some(user12_vesting_schedule)); // Account 12 has a vesting schedule

			// Account 1 has only 128 units vested from their illiquid 256 * 5 units at block 1
			assert_eq!(Vesting::vesting_balance(&1), Some(256 * 5));
			// Account 2 has their full balance locked
			assert_eq!(Vesting::vesting_balance(&2), Some(user2_free_balance));
			// Account 12 has only their illiquid funds locked
			assert_eq!(Vesting::vesting_balance(&12), Some(user12_free_balance - 256 * 5));

			System::set_block_number(10);
			assert_eq!(System::block_number(), 10);

			assert_ok!(Vesting::init_vesting_start_at(Origin::root(), 0));

			// Account 1 has fully vested by block 10
			assert_eq!(Vesting::vesting_balance(&1), Some(0));
			// Account 2 has started vesting by block 10
			assert_eq!(Vesting::vesting_balance(&2), Some(user2_free_balance));
			// Account 12 has started vesting by block 10
			assert_eq!(Vesting::vesting_balance(&12), Some(user12_free_balance - 256 * 5));

			System::set_block_number(30);
			assert_eq!(System::block_number(), 30);

			assert_eq!(Vesting::vesting_balance(&1), Some(0)); // Account 1 is still fully vested, and not negative
			assert_eq!(Vesting::vesting_balance(&2), Some(0)); // Account 2 has fully vested by block 30
			assert_eq!(Vesting::vesting_balance(&12), Some(0)); // Account 2 has fully vested by block 30
		});
	}

	#[test]
	fn unvested_balance_should_not_transfer() {
		ExtBuilder::default().existential_deposit(10).build().execute_with(|| {
			let user1_free_balance = Balances::free_balance(&1);
			assert_eq!(user1_free_balance, 100); // Account 1 has free balance
			assert_ok!(Vesting::init_vesting_start_at(Origin::root(), 10));
			System::set_block_number(11);
			// Account 1 has only 5 units vested at block 1 (plus 50 unvested)
			assert_eq!(Vesting::vesting_balance(&1), Some(45));
			assert_noop!(
				Balances::transfer(Some(1).into(), 2, 56),
				pallet_balances::Error::<Test, _>::LiquidityRestrictions,
			); // Account 1 cannot send more than vested amount
		});
	}

	#[test]
	fn vested_balance_should_transfer() {
		ExtBuilder::default().existential_deposit(10).build().execute_with(|| {
			let user1_free_balance = Balances::free_balance(&1);
			assert_eq!(user1_free_balance, 100); // Account 1 has free balance
			assert_ok!(Vesting::init_vesting_start_at(Origin::root(), 10));
			System::set_block_number(11);
			// Account 1 has only 5 units vested at block 1 (plus 50 unvested)
			assert_eq!(Vesting::vesting_balance(&1), Some(45));
			assert_ok!(Vesting::vest(Some(1).into()));
			assert_ok!(Balances::transfer(Some(1).into(), 2, 55));
		});
	}

	#[test]
	fn vested_balance_should_transfer_using_vest_other() {
		ExtBuilder::default().existential_deposit(10).build().execute_with(|| {
			let user1_free_balance = Balances::free_balance(&1);
			assert_eq!(user1_free_balance, 100); // Account 1 has free balance
			assert_ok!(Vesting::init_vesting_start_at(Origin::root(), 10));
			System::set_block_number(11);
			// Account 1 has only 5 units vested at block 1 (plus 50 unvested)
			assert_eq!(Vesting::vesting_balance(&1), Some(45));
			assert_ok!(Vesting::vest_other(Some(2).into(), 1));
			assert_ok!(Balances::transfer(Some(1).into(), 2, 55));
		});
	}

	#[test]
	fn extra_balance_should_transfer() {
		ExtBuilder::default().existential_deposit(10).build().execute_with(|| {
			assert_ok!(Balances::transfer(Some(3).into(), 1, 100));
			assert_ok!(Balances::transfer(Some(3).into(), 2, 100));

			let user1_free_balance = Balances::free_balance(&1);
			assert_eq!(user1_free_balance, 200); // Account 1 has 100 more free balance than normal

			let user2_free_balance = Balances::free_balance(&2);
			assert_eq!(user2_free_balance, 300); // Account 2 has 100 more free balance than normal

			assert_ok!(Vesting::init_vesting_start_at(Origin::root(), 10));
			System::set_block_number(11);

			// Account 1 has only 5 units vested at block 1 (plus 150 unvested)
			assert_eq!(Vesting::vesting_balance(&1), Some(45));
			assert_ok!(Vesting::vest(Some(1).into()));
			assert_ok!(Balances::transfer(Some(1).into(), 3, 155)); // Account 1 can send extra units gained

			// Account 2 has no units vested at block 1, but gained 100
			assert_eq!(Vesting::vesting_balance(&2), Some(200));
			assert_ok!(Vesting::vest(Some(2).into()));
			assert_ok!(Balances::transfer(Some(2).into(), 3, 100)); // Account 2 can send extra units gained
		});
	}

	#[test]
	fn liquid_funds_should_transfer_with_delayed_vesting() {
		ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
			let user12_free_balance = Balances::free_balance(&12);

			assert_eq!(user12_free_balance, 2560); // Account 12 has free balance
									   // Account 12 has liquid funds
			assert_eq!(Vesting::vesting_balance(&12), Some(user12_free_balance - 256 * 5));

			// Account 12 has delayed vesting
			let user12_vesting_schedule = VestingInfo {
				locked: 256 * 5,
				per_block: 64, // Vesting over 20 blocks
				starting_block: 10,
			};
			assert_eq!(Vesting::vesting(&12), Some(user12_vesting_schedule));

			// Account 12 can still send liquid funds
			assert_ok!(Balances::transfer(Some(12).into(), 3, 256 * 5));
		});
	}

	#[test]
	fn vested_transfer_works() {
		ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
			let user3_free_balance = Balances::free_balance(&3);
			let user4_free_balance = Balances::free_balance(&4);
			assert_eq!(user3_free_balance, 256 * 30);
			assert_eq!(user4_free_balance, 256 * 40);
			// Account 4 should not have any vesting yet.
			assert_eq!(Vesting::vesting(&4), None);
			// Make the schedule for the new transfer.
			let new_vesting_schedule = VestingInfo {
				locked: 256 * 5,
				per_block: 64, // Vesting over 20 blocks
				starting_block: 10,
			};
			assert_ok!(Vesting::vested_transfer(Some(3).into(), 4, new_vesting_schedule));
			// Now account 4 should have vesting.
			assert_eq!(Vesting::vesting(&4), Some(new_vesting_schedule));
			// Ensure the transfer happened correctly.
			let user3_free_balance_updated = Balances::free_balance(&3);
			assert_eq!(user3_free_balance_updated, 256 * 25);
			let user4_free_balance_updated = Balances::free_balance(&4);
			assert_eq!(user4_free_balance_updated, 256 * 45);
			// Account 4 has 5 * 256 locked.
			assert_eq!(Vesting::vesting_balance(&4), Some(256 * 5));

			assert_ok!(Vesting::init_vesting_start_at(Origin::root(), 0));

			System::set_block_number(20);
			assert_eq!(System::block_number(), 20);

			// Account 4 has 5 * 64 units vested by block 20.
			assert_eq!(Vesting::vesting_balance(&4), Some(10 * 64));

			System::set_block_number(30);
			assert_eq!(System::block_number(), 30);

			// Account 4 has fully vested.
			assert_eq!(Vesting::vesting_balance(&4), Some(0));
		});
	}

	#[test]
	fn vested_transfer_correctly_fails() {
		ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
			let user2_free_balance = Balances::free_balance(&2);
			let user4_free_balance = Balances::free_balance(&4);
			assert_eq!(user2_free_balance, 256 * 20);
			assert_eq!(user4_free_balance, 256 * 40);
			// Account 2 should already have a vesting schedule.
			let user2_vesting_schedule = VestingInfo {
				locked: 256 * 20,
				per_block: 256, // Vesting over 20 blocks
				starting_block: 10,
			};
			assert_eq!(Vesting::vesting(&2), Some(user2_vesting_schedule));

			// The vesting schedule we will try to create, fails due to pre-existence of schedule.
			let new_vesting_schedule = VestingInfo {
				locked: 256 * 5,
				per_block: 64, // Vesting over 20 blocks
				starting_block: 10,
			};
			assert_noop!(
				Vesting::vested_transfer(Some(4).into(), 2, new_vesting_schedule),
				Error::<Test>::ExistingVestingSchedule,
			);

			// Fails due to too low transfer amount.
			let new_vesting_schedule_too_low =
				VestingInfo { locked: 256 * 1, per_block: 64, starting_block: 10 };
			assert_noop!(
				Vesting::vested_transfer(Some(3).into(), 4, new_vesting_schedule_too_low),
				Error::<Test>::AmountLow,
			);

			// Verify no currency transfer happened.
			assert_eq!(user2_free_balance, 256 * 20);
			assert_eq!(user4_free_balance, 256 * 40);
		});
	}

	#[test]
	fn force_vested_transfer_works() {
		ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
			let user3_free_balance = Balances::free_balance(&3);
			let user4_free_balance = Balances::free_balance(&4);
			assert_eq!(user3_free_balance, 256 * 30);
			assert_eq!(user4_free_balance, 256 * 40);
			// Account 4 should not have any vesting yet.
			assert_eq!(Vesting::vesting(&4), None);
			// Make the schedule for the new transfer.
			let new_vesting_schedule = VestingInfo {
				locked: 256 * 5,
				per_block: 64, // Vesting over 20 blocks
				starting_block: 10,
			};
			assert_noop!(
				Vesting::force_vested_transfer(Some(4).into(), 3, 4, new_vesting_schedule),
				BadOrigin
			);
			assert_ok!(Vesting::force_vested_transfer(
				RawOrigin::Root.into(),
				3,
				4,
				new_vesting_schedule
			));
			// Now account 4 should have vesting.
			assert_eq!(Vesting::vesting(&4), Some(new_vesting_schedule));
			// Ensure the transfer happened correctly.
			let user3_free_balance_updated = Balances::free_balance(&3);
			assert_eq!(user3_free_balance_updated, 256 * 25);
			let user4_free_balance_updated = Balances::free_balance(&4);
			assert_eq!(user4_free_balance_updated, 256 * 45);
			// Account 4 has 5 * 256 locked.
			assert_eq!(Vesting::vesting_balance(&4), Some(256 * 5));

			System::set_block_number(20);
			assert_eq!(System::block_number(), 20);

			// Account 4 has 5 * 256 locked.
			assert_eq!(Vesting::vesting_balance(&4), Some(256 * 5));

			System::set_block_number(30);
			assert_eq!(System::block_number(), 30);

			assert_ok!(Vesting::init_vesting_start_at(Origin::root(), 10));

			assert_eq!(Vesting::vesting_balance(&4), Some(64 * 10));
		});
	}

	#[test]
	fn force_vested_transfer_correctly_fails() {
		ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
			let user2_free_balance = Balances::free_balance(&2);
			let user4_free_balance = Balances::free_balance(&4);
			assert_eq!(user2_free_balance, 256 * 20);
			assert_eq!(user4_free_balance, 256 * 40);
			// Account 2 should already have a vesting schedule.
			let user2_vesting_schedule = VestingInfo {
				locked: 256 * 20,
				per_block: 256, // Vesting over 20 blocks
				starting_block: 10,
			};
			assert_eq!(Vesting::vesting(&2), Some(user2_vesting_schedule));

			// The vesting schedule we will try to create, fails due to pre-existence of schedule.
			let new_vesting_schedule = VestingInfo {
				locked: 256 * 5,
				per_block: 64, // Vesting over 20 blocks
				starting_block: 10,
			};
			assert_noop!(
				Vesting::force_vested_transfer(RawOrigin::Root.into(), 4, 2, new_vesting_schedule),
				Error::<Test>::ExistingVestingSchedule,
			);

			// Fails due to too low transfer amount.
			let new_vesting_schedule_too_low =
				VestingInfo { locked: 256 * 1, per_block: 64, starting_block: 10 };
			assert_noop!(
				Vesting::force_vested_transfer(
					RawOrigin::Root.into(),
					3,
					4,
					new_vesting_schedule_too_low
				),
				Error::<Test>::AmountLow,
			);

			// Verify no currency transfer happened.
			assert_eq!(user2_free_balance, 256 * 20);
			assert_eq!(user4_free_balance, 256 * 40);
		});
	}

	#[test]
	fn set_vesting_per_block_should_work() {
		ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
			assert_ok!(Vesting::init_vesting_start_at(Origin::root(), 1));

			let user1_free_balance = Balances::free_balance(&1);
			assert_eq!(user1_free_balance, 256 * 10); // Account 1 has free balance
			let user1_vesting_schedule = VestingInfo {
				locked: 256 * 5,
				per_block: 128, // Vesting over 10 blocks
				starting_block: 0,
			};

			assert_eq!(Vesting::vesting(&1), Some(user1_vesting_schedule)); // Account 1 has a vesting schedule

			// Account 1 has only 128 units vested from their illiquid 256 * 5 units at block 1
			assert_eq!(Vesting::vesting_balance(&1), Some(256 * 5));

			System::set_block_number(6);
			assert_eq!(System::block_number(), 6);

			// Account 1 has vested by half at the end of block 5
			assert_eq!(Vesting::vesting_balance(&1), Some(128 * 5));

			// Change the per_block of account 1 to  256
			assert_ok!(Vesting::set_vesting_per_block(Origin::root(), 1, 256));

			System::set_block_number(7);
			assert_eq!(System::block_number(), 7);

			let change1_user1_vesting_schedule = VestingInfo {
				locked: 256 * 5 - 128 * 5,
				per_block: 256, // Vesting over 10 blocks
				starting_block: 5,
			};

			assert_eq!(Vesting::vesting(&1), Some(change1_user1_vesting_schedule)); // Account 1 has a vesting schedule
			assert_eq!(Vesting::vesting_balance(&1), Some(256 * 5 - 128 * 5 - 256));

			assert_eq!(
				Vesting::set_vesting_per_block(RawOrigin::Root.into(), 1, 256),
				Err(DispatchError::Module { index: 2, error: 3, message: Some("SamePerBlock") })
			);

			assert_ok!(Vesting::set_vesting_per_block(Origin::root(), 1, 10));

			System::set_block_number(8);
			assert_eq!(System::block_number(), 8);

			let change2_user1_vesting_schedule = VestingInfo {
				locked: 256 * 5 - 128 * 5 - 256,
				per_block: 10, // Vesting over 10 blocks
				starting_block: 6,
			};

			assert_eq!(Vesting::vesting(&1), Some(change2_user1_vesting_schedule));
			assert_eq!(Vesting::vesting_balance(&1), Some(256 * 5 - 128 * 5 - 256 - 10));

			System::set_block_number(46);
			assert_eq!(System::block_number(), 46);

			assert_eq!(
				Vesting::set_vesting_per_block(Origin::root(), 1, 20),
				Err(DispatchError::Module { index: 2, error: 0, message: Some("NotVesting") })
			);
		});
	}

	#[test]
	fn set_vesting_per_block_before_and_after_original_start_block_should_work() {
		ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
			assert_ok!(Vesting::init_vesting_start_at(Origin::root(), 10));

			let user1_free_balance = Balances::free_balance(&1);
			assert_eq!(user1_free_balance, 256 * 10); // Account 1 has free balance
			let user1_vesting_schedule = VestingInfo {
				locked: 256 * 5,
				per_block: 128, // Vesting over 10 blocks
				starting_block: 0,
			};

			assert_eq!(Vesting::vesting(&1), Some(user1_vesting_schedule)); // Account 1 has a vesting schedule

			// Account 1 has only 128 units vested from their illiquid 256 * 5 units at block 1
			assert_eq!(Vesting::vesting_balance(&1), Some(256 * 5));

			System::set_block_number(6);
			assert_eq!(System::block_number(), 6);

			// Change the per_block of account 1 to  256
			assert_ok!(Vesting::set_vesting_per_block(Origin::root(), 1, 256));

			let user2_vesting_schedule = VestingInfo {
				locked: 256 * 5,
				per_block: 256, // Vesting over 10 blocks
				starting_block: 0,
			};

			assert_eq!(Vesting::vesting(&1), Some(user2_vesting_schedule)); // Account 1 has a vesting schedule

			System::set_block_number(12);
			assert_eq!(System::block_number(), 12);

			assert_ok!(Vesting::set_vesting_per_block(Origin::root(), 1, 128));

			let user3_vesting_schedule = VestingInfo {
				locked: 256 * 5 - 256 * 2,
				per_block: 128, // Vesting over 10 blocks
				starting_block: 2,
			};

			assert_eq!(Vesting::vesting(&1), Some(user3_vesting_schedule)); // Account 1 has a vesting schedule
		});
	}

	#[test]
	fn set_cliff_should_work() {
		ExtBuilder::default().existential_deposit(10).build().execute_with(|| {
			assert_ok!(Vesting::vest(Some(1).into()));
			assert_ok!(Vesting::force_set_cliff(Origin::root(), 1, 10));
			assert_noop!(Vesting::vest(Some(1).into()), Error::<Test>::WrongCliffVesting);
			assert_noop!(Vesting::vest_other(Some(2).into(), 1), Error::<Test>::WrongCliffVesting);
			System::set_block_number(10);
			assert_noop!(Vesting::vest(Some(1).into()), Error::<Test>::WrongCliffVesting);
			assert_noop!(Vesting::vest_other(Some(2).into(), 1), Error::<Test>::WrongCliffVesting);
			System::set_block_number(11);
			assert_ok!(Vesting::vest(Some(1).into()));
			assert_ok!(Vesting::vest_other(Some(2).into(), 1));
		});
	}
}
