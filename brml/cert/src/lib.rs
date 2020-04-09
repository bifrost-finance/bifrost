#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use sp_std::{cmp, result, fmt::Debug, ops::BitOr, convert::Infallible};
use codec::{Codec, Encode, Decode};
use frame_support::{
	StorageValue, Parameter, decl_event, decl_storage, decl_module, decl_error, ensure,
	weights::SimpleDispatchInfo, traits::{
		Currency, OnUnbalanced, TryDrop, StoredMap,
		WithdrawReason, WithdrawReasons, LockIdentifier, ExistenceRequirement,
		Imbalance, SignedImbalance, Get,
		ExistenceRequirement::AllowDeath, MigrateAccount,
	}
};
use sp_runtime::{
	RuntimeDebug, DispatchResult, DispatchError,
	traits::{
		Zero, AtLeast32Bit, StaticLookup, Member, CheckedAdd, CheckedSub,
		MaybeSerializeDeserialize, Saturating, Bounded,
	},
};
use system::{ensure_signed};

pub use self::imbalances::{PositiveImbalance, NegativeImbalance};

pub trait Subtrait<I: Instance = DefaultInstance>: system::Trait {
	/// The balance of an account.
	type Balance: Parameter + Member + AtLeast32Bit + Codec + Default + Copy +
	MaybeSerializeDeserialize + Debug;

	/// The minimum amount required to keep an account open.
	type ExistentialDeposit: Get<Self::Balance>;

	/// The means of storing the balances of an account.
	type AccountStore: StoredMap<Self::AccountId, AccountData<Self::Balance>>;
}

pub trait Trait<I: Instance = DefaultInstance>: system::Trait {
	type Balance: Parameter + Member + AtLeast32Bit + Codec + Default + Copy +
	MaybeSerializeDeserialize + Debug;
	type DustRemoval: OnUnbalanced<NegativeImbalance<Self, I>>;
	type Event: From<Event<Self, I>> + Into<<Self as system::Trait>::Event>;
	type ExistentialDeposit: Get<Self::Balance>;
	type AccountStore: StoredMap<Self::AccountId, AccountData<Self::Balance>>;
}

impl<T: Trait<I>, I: Instance> Subtrait<I> for T {
	type Balance = T::Balance;
	type ExistentialDeposit = T::ExistentialDeposit;
	type AccountStore = T::AccountStore;
}

decl_event!(
	pub enum Event<T, I: Instance = DefaultInstance> where
		<T as system::Trait>::AccountId,
		<T as Trait<I>>::Balance
	{
		/// An account was created with some free balance.
		Endowed(AccountId, Balance),
		/// An account was removed whose balance was non-zero but below ExistentialDeposit,
		/// resulting in an outright loss.
		DustLost(AccountId, Balance),
		/// Transfer succeeded (from, to, value).
		Transfer(AccountId, AccountId, Balance),
		/// A balance was set by root (who, free, reserved).
		BalanceSet(AccountId, Balance, Balance),
		/// Some amount was deposited (e.g. for transaction fees).
		Deposit(AccountId, Balance),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait<I>, I: Instance> {
		/// Vesting balance too high to send value
		VestingBalance,
		/// Account liquidity restrictions prevent withdrawal
		LiquidityRestrictions,
		/// Got an overflow after adding
		Overflow,
		/// Balance too low to send value
		InsufficientBalance,
		/// Value too low to create account due to existential deposit
		ExistentialDeposit,
		/// Transfer/payment would kill account
		KeepAlive,
		/// A vesting schedule already exists for this account
		ExistingVestingSchedule,
		/// Beneficiary account must pre-exist
		DeadAccount,
	}
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug)]
pub enum Reasons {
	/// Paying system transaction fees.
	Fee = 0,
	/// Any reason other than paying system transaction fees.
	Misc = 1,
	/// Any reason at all.
	All = 2,
}

impl From<WithdrawReasons> for Reasons {
	fn from(r: WithdrawReasons) -> Reasons {
		if r == WithdrawReasons::from(WithdrawReason::TransactionPayment) {
			Reasons::Fee
		} else if r.contains(WithdrawReason::TransactionPayment) {
			Reasons::All
		} else {
			Reasons::Misc
		}
	}
}

impl BitOr for Reasons {
	type Output = Reasons;
	fn bitor(self, other: Reasons) -> Reasons {
		if self == other { return self }
		Reasons::All
	}
}

/// A single lock on a balance. There can be many of these on an account and they "overlap", so the
/// same balance is frozen by multiple locks.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct BalanceLock<Balance> {
	/// An identifier for this lock. Only one lock may be in existence for each identifier.
	pub id: LockIdentifier,
	/// The amount which the free balance may not drop below when this lock is in effect.
	pub amount: Balance,
	/// If true, then the lock remains in effect even for payment of transaction fees.
	pub reasons: Reasons,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct AccountData<Balance> {
	pub free: Balance,

	pub reserved: Balance,

	pub misc_frozen: Balance,

	pub fee_frozen: Balance,
}

impl<Balance: Saturating + Copy + Ord> AccountData<Balance> {
	fn usable(&self, reasons: Reasons) -> Balance {
		self.free.saturating_sub(self.frozen(reasons))
	}
	fn frozen(&self, reasons: Reasons) -> Balance {
		match reasons {
			Reasons::All => self.misc_frozen.max(self.fee_frozen),
			Reasons::Misc => self.misc_frozen,
			Reasons::Fee => self.fee_frozen,
		}
	}
	fn total(&self) -> Balance {
		self.free.saturating_add(self.reserved)
	}
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug)]
enum Releases {
	V1_0_0,
	V2_0_0,
}

impl Default for Releases {
	fn default() -> Self {
		Releases::V1_0_0
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance=DefaultInstance> as Balances {
		/// The total units issued in the system.
		pub TotalIssuance get(fn total_issuance) build(|config: &GenesisConfig<T, I>| {
			config.balances.iter().fold(Zero::zero(), |acc: T::Balance, &(_, n)| acc + n)
		}): T::Balance;

		pub Account: map hasher(blake2_128_concat) T::AccountId => AccountData<T::Balance>;

		pub Locks get(fn locks): map hasher(blake2_128_concat) T::AccountId => Vec<BalanceLock<T::Balance>>;

		StorageVersion build(|_: &GenesisConfig<T, I>| Releases::V2_0_0): Releases;
	}
	add_extra_genesis {
		config(balances): Vec<(T::AccountId, T::Balance)>;
		// ^^ begin, length, amount liquid at genesis
		build(|config: &GenesisConfig<T, I>| {
			assert!(
				<T as Trait<I>>::ExistentialDeposit::get() > Zero::zero(),
				"The existential deposit should be greater than zero."
			);
			for (_, balance) in &config.balances {
				assert!(
					*balance >= <T as Trait<I>>::ExistentialDeposit::get(),
					"the balance of any account should always be more than existential deposit.",
				)
			}
			for &(ref who, free) in config.balances.iter() {
				T::AccountStore::insert(who, AccountData { free, .. Default::default() });
			}
		});
	}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance = DefaultInstance> for enum Call where origin: T::Origin {
		type Error = Error<T, I>;

		const ExistentialDeposit: T::Balance = T::ExistentialDeposit::get();

		fn deposit_event() = default;

		#[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
		pub fn transfer(
			origin,
			dest: <T::Lookup as StaticLookup>::Source,
			#[compact] value: T::Balance
		) {
			let transactor = ensure_signed(origin)?;
			let dest = T::Lookup::lookup(dest)?;
			<Self as Currency<_>>::transfer(&transactor, &dest, value, ExistenceRequirement::AllowDeath)?;
		}
	}
}

impl<T: Trait<I>, I: Instance> MigrateAccount<T::AccountId> for Module<T, I> {
	fn migrate_account(account: &T::AccountId) {
		Locks::<T, I>::migrate_key_from_blake(account);
	}
}

impl<T: Trait<I>, I: Instance> Module<T, I> {

	pub fn free_balance(who: impl sp_std::borrow::Borrow<T::AccountId>) -> T::Balance {
		Self::account(who.borrow()).free
	}

	pub fn usable_balance(who: impl sp_std::borrow::Borrow<T::AccountId>) -> T::Balance {
		Self::account(who.borrow()).usable(Reasons::Misc)
	}

	pub fn usable_balance_for_fees(who: impl sp_std::borrow::Borrow<T::AccountId>) -> T::Balance {
		Self::account(who.borrow()).usable(Reasons::Fee)
	}

	pub fn reserved_balance(who: impl sp_std::borrow::Borrow<T::AccountId>) -> T::Balance {
		Self::account(who.borrow()).reserved
	}

	fn account(who: &T::AccountId) -> AccountData<T::Balance> {
		T::AccountStore::get(&who)
	}

	fn post_mutation(
		who: &T::AccountId,
		new: AccountData<T::Balance>,
	) -> Option<AccountData<T::Balance>> {
		let total = new.total();
		if total < T::ExistentialDeposit::get() {
			if !total.is_zero() {
				T::DustRemoval::on_unbalanced(NegativeImbalance::new(total));
				Self::deposit_event(RawEvent::DustLost(who.clone(), total));
			}
			None
		} else {
			Some(new)
		}
	}

	fn mutate_account<R>(
		who: &T::AccountId,
		f: impl FnOnce(&mut AccountData<T::Balance>) -> R
	) -> R {
		Self::try_mutate_account(who, |a| -> Result<R, Infallible> { Ok(f(a)) })
			.expect("Error is infallible; qed")
	}
	fn try_mutate_account<R, E>(
		who: &T::AccountId,
		f: impl FnOnce(&mut AccountData<T::Balance>) -> Result<R, E>
	) -> Result<R, E> {
		T::AccountStore::try_mutate_exists(who, |maybe_account| {
			let mut account = maybe_account.take().unwrap_or_default();
			let was_zero = account.total().is_zero();
			f(&mut account).map(move |result| {
				let maybe_endowed = if was_zero { Some(account.free) } else { None };
				*maybe_account = Self::post_mutation(who, account);
				(maybe_endowed, result)
			})
		}).map(|(maybe_endowed, result)| {
			if let Some(endowed) = maybe_endowed {
				Self::deposit_event(RawEvent::Endowed(who.clone(), endowed));
			}
			result
		})
	}
}
mod imbalances {
	use super::{
		result, Subtrait, DefaultInstance, Imbalance, Trait, Zero, Instance, Saturating, TryDrop
	};
	use sp_std::mem;

	#[must_use]
	pub struct PositiveImbalance<T: Subtrait<I>, I: Instance=DefaultInstance>(T::Balance);

	impl<T: Subtrait<I>, I: Instance> PositiveImbalance<T, I> {
		pub fn new(amount: T::Balance) -> Self {
			PositiveImbalance(amount)
		}
	}

	#[must_use]
	pub struct NegativeImbalance<T: Subtrait<I>, I: Instance=DefaultInstance>(T::Balance);
	impl<T: Subtrait<I>, I: Instance> NegativeImbalance<T, I> {
		pub fn new(amount: T::Balance) -> Self {
			NegativeImbalance(amount)
		}
	}

	impl<T: Trait<I>, I: Instance> TryDrop for PositiveImbalance<T, I> {
		fn try_drop(self) -> result::Result<(), Self> {
			self.drop_zero()
		}
	}

	impl<T: Trait<I>, I: Instance> Imbalance<T::Balance> for PositiveImbalance<T, I> {
		type Opposite = NegativeImbalance<T, I>;

		fn zero() -> Self {
			Self(Zero::zero())
		}
		fn drop_zero(self) -> result::Result<(), Self> {
			if self.0.is_zero() {
				Ok(())
			} else {
				Err(self)
			}
		}
		fn split(self, amount: T::Balance) -> (Self, Self) {
			let first = self.0.min(amount);
			let second = self.0 - first;

			mem::forget(self);
			(Self(first), Self(second))
		}
		fn merge(mut self, other: Self) -> Self {
			self.0 = self.0.saturating_add(other.0);
			mem::forget(other);

			self
		}
		fn subsume(&mut self, other: Self) {
			self.0 = self.0.saturating_add(other.0);
			mem::forget(other);
		}
		fn offset(self, other: Self::Opposite) -> result::Result<Self, Self::Opposite> {
			let (a, b) = (self.0, other.0);
			mem::forget((self, other));

			if a >= b {
				Ok(Self(a - b))
			} else {
				Err(NegativeImbalance::new(b - a))
			}
		}
		fn peek(&self) -> T::Balance {
			self.0.clone()
		}
	}

	impl<T: Trait<I>, I: Instance> TryDrop for NegativeImbalance<T, I> {
		fn try_drop(self) -> result::Result<(), Self> {
			self.drop_zero()
		}
	}

	impl<T: Trait<I>, I: Instance> Imbalance<T::Balance> for NegativeImbalance<T, I> {
		type Opposite = PositiveImbalance<T, I>;

		fn zero() -> Self {
			Self(Zero::zero())
		}
		fn drop_zero(self) -> result::Result<(), Self> {
			if self.0.is_zero() {
				Ok(())
			} else {
				Err(self)
			}
		}
		fn split(self, amount: T::Balance) -> (Self, Self) {
			let first = self.0.min(amount);
			let second = self.0 - first;

			mem::forget(self);
			(Self(first), Self(second))
		}
		fn merge(mut self, other: Self) -> Self {
			self.0 = self.0.saturating_add(other.0);
			mem::forget(other);

			self
		}
		fn subsume(&mut self, other: Self) {
			self.0 = self.0.saturating_add(other.0);
			mem::forget(other);
		}
		fn offset(self, other: Self::Opposite) -> result::Result<Self, Self::Opposite> {
			let (a, b) = (self.0, other.0);
			mem::forget((self, other));

			if a >= b {
				Ok(Self(a - b))
			} else {
				Err(PositiveImbalance::new(b - a))
			}
		}
		fn peek(&self) -> T::Balance {
			self.0.clone()
		}
	}
}

impl<T: Trait<I>, I: Instance> Currency<T::AccountId> for Module<T, I> where
	T::Balance: MaybeSerializeDeserialize + Debug
{
	type Balance = T::Balance;
	type PositiveImbalance = PositiveImbalance<T, I>;
	type NegativeImbalance = NegativeImbalance<T, I>;

	fn total_balance(who: &T::AccountId) -> Self::Balance {
		Self::account(who).total()
	}

	// Check if `value` amount of free balance can be slashed from `who`.
	fn can_slash(who: &T::AccountId, value: Self::Balance) -> bool {
		if value.is_zero() { return true }
		Self::free_balance(who) >= value
	}

	fn total_issuance() -> Self::Balance {
		<TotalIssuance<T, I>>::get()
	}

	fn minimum_balance() -> Self::Balance {
		T::ExistentialDeposit::get()
	}

	// Burn funds from the total issuance, returning a positive imbalance for the amount burned.
	// Is a no-op if amount to be burned is zero.
	fn burn(mut amount: Self::Balance) -> Self::PositiveImbalance {
		if amount.is_zero() { return PositiveImbalance::zero() }
		<TotalIssuance<T, I>>::mutate(|issued| {
			*issued = issued.checked_sub(&amount).unwrap_or_else(|| {
				amount = *issued;
				Zero::zero()
			});
		});
		PositiveImbalance::new(amount)
	}

	// Create new funds into the total issuance, returning a negative imbalance
	// for the amount issued.
	// Is a no-op if amount to be issued it zero.
	fn issue(mut amount: Self::Balance) -> Self::NegativeImbalance {
		if amount.is_zero() { return NegativeImbalance::zero() }
		<TotalIssuance<T, I>>::mutate(|issued|
			*issued = issued.checked_add(&amount).unwrap_or_else(|| {
				amount = Self::Balance::max_value() - *issued;
				Self::Balance::max_value()
			})
		);
		NegativeImbalance::new(amount)
	}

	fn free_balance(who: &T::AccountId) -> Self::Balance {
		Self::account(who).free
	}

	fn ensure_can_withdraw(
		who: &T::AccountId,
		amount: T::Balance,
		reasons: WithdrawReasons,
		new_balance: T::Balance,
	) -> DispatchResult {
		if amount.is_zero() { return Ok(()) }
		let min_balance = Self::account(who).frozen(reasons.into());
		ensure!(new_balance >= min_balance, Error::<T, I>::LiquidityRestrictions);
		Ok(())
	}

	fn transfer(
		transactor: &T::AccountId,
		dest: &T::AccountId,
		value: Self::Balance,
		existence_requirement: ExistenceRequirement,
	) -> DispatchResult {
		if value.is_zero() || transactor == dest { return Ok(()) }

		Self::try_mutate_account(dest, |to_account| -> DispatchResult {
			Self::try_mutate_account(transactor, |from_account| -> DispatchResult {
				from_account.free = from_account.free.checked_sub(&value)
					.ok_or(Error::<T, I>::InsufficientBalance)?;

				// NOTE: total stake being stored in the same type means that this could never overflow
				// but better to be safe than sorry.
				to_account.free = to_account.free.checked_add(&value).ok_or(Error::<T, I>::Overflow)?;

				let ed = T::ExistentialDeposit::get();
				ensure!(to_account.total() >= ed, Error::<T, I>::ExistentialDeposit);

				Self::ensure_can_withdraw(
					transactor,
					value,
					WithdrawReason::Transfer.into(),
					from_account.free,
				)?;

				let allow_death = existence_requirement == ExistenceRequirement::AllowDeath;
				let allow_death = allow_death && system::Module::<T>::allow_death(transactor);
				ensure!(allow_death || from_account.free >= ed, Error::<T, I>::KeepAlive);

				Ok(())
			})
		})?;

		// Emit transfer event.
		Self::deposit_event(RawEvent::Transfer(transactor.clone(), dest.clone(), value));

		Ok(())
	}

	fn slash(
		who: &T::AccountId,
		value: Self::Balance
	) -> (Self::NegativeImbalance, Self::Balance) {
		if value.is_zero() { return (NegativeImbalance::zero(), Zero::zero()) }

		Self::mutate_account(who, |account| {
			let free_slash = cmp::min(account.free, value);
			account.free -= free_slash;

			let remaining_slash = value - free_slash;
			if !remaining_slash.is_zero() {
				let reserved_slash = cmp::min(account.reserved, remaining_slash);
				account.reserved -= reserved_slash;
				(NegativeImbalance::new(free_slash + reserved_slash), remaining_slash - reserved_slash)
			} else {
				(NegativeImbalance::new(value), Zero::zero())
			}
		})
	}

	fn deposit_into_existing(
		who: &T::AccountId,
		value: Self::Balance
	) -> Result<Self::PositiveImbalance, DispatchError> {
		if value.is_zero() { return Ok(PositiveImbalance::zero()) }

		Self::try_mutate_account(who, |account| -> Result<Self::PositiveImbalance, DispatchError> {
			ensure!(!account.total().is_zero(), Error::<T, I>::DeadAccount);
			account.free = account.free.checked_add(&value).ok_or(Error::<T, I>::Overflow)?;
			Ok(PositiveImbalance::new(value))
		})
	}

	/// Deposit some `value` into the free balance of `who`, possibly creating a new account.
	///
	/// This function is a no-op if:
	/// - the `value` to be deposited is zero; or
	/// - if the `value` to be deposited is less than the ED and the account does not yet exist; or
	/// - `value` is so large it would cause the balance of `who` to overflow.
	fn deposit_creating(
		who: &T::AccountId,
		value: Self::Balance,
	) -> Self::PositiveImbalance {
		if value.is_zero() { return Self::PositiveImbalance::zero() }

		Self::try_mutate_account(who, |account| -> Result<Self::PositiveImbalance, Self::PositiveImbalance> {
			// bail if not yet created and this operation wouldn't be enough to create it.
			let ed = T::ExistentialDeposit::get();
			ensure!(value >= ed || !account.total().is_zero(), Self::PositiveImbalance::zero());

			// defensive only: overflow should never happen, however in case it does, then this
			// operation is a no-op.
			account.free = account.free.checked_add(&value).ok_or(Self::PositiveImbalance::zero())?;

			Ok(PositiveImbalance::new(value))
		}).unwrap_or_else(|x| x)
	}

	fn withdraw(
		who: &T::AccountId,
		value: Self::Balance,
		reasons: WithdrawReasons,
		liveness: ExistenceRequirement,
	) -> result::Result<Self::NegativeImbalance, DispatchError> {
		if value.is_zero() { return Ok(NegativeImbalance::zero()); }

		Self::try_mutate_account(who, |account|
									   -> Result<Self::NegativeImbalance, DispatchError>
			{
				let new_free_account = account.free.checked_sub(&value)
					.ok_or(Error::<T, I>::InsufficientBalance)?;

				// bail if we need to keep the account alive and this would kill it.
				let ed = T::ExistentialDeposit::get();
				let would_be_dead = new_free_account + account.reserved < ed;
				let would_kill = would_be_dead && account.free + account.reserved >= ed;
				ensure!(liveness == AllowDeath || !would_kill, Error::<T, I>::KeepAlive);

				Self::ensure_can_withdraw(who, value, reasons, new_free_account)?;

				account.free = new_free_account;

				Ok(NegativeImbalance::new(value))
			})
	}

	fn make_free_balance_be(who: &T::AccountId, value: Self::Balance)
							-> SignedImbalance<Self::Balance, Self::PositiveImbalance>
	{
		Self::try_mutate_account(who, |account|
									   -> Result<SignedImbalance<Self::Balance, Self::PositiveImbalance>, ()>
			{
				let ed = T::ExistentialDeposit::get();
				ensure!(value + account.reserved >= ed || !account.total().is_zero(), ());

				let imbalance = if account.free <= value {
					SignedImbalance::Positive(PositiveImbalance::new(value - account.free))
				} else {
					SignedImbalance::Negative(NegativeImbalance::new(account.free - value))
				};
				account.free = value;
				Ok(imbalance)
			}).unwrap_or(SignedImbalance::Positive(Self::PositiveImbalance::zero()))
	}
}




