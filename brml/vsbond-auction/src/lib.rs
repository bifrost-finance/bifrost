use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use orml_traits::{
	MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency, MultiReservableCurrency,
};

mod mock;
mod tests;

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct OrderInfo<T: Config> {
	owner: AccountIdOf<T>,
	currency_sold: CurrencyIdOf<T>,
	amount_sold: BalanceOf<T>,
	currency_expected: CurrencyIdOf<T>,
	amount_expected: BalanceOf<T>,
	order_id: OrderId,
	order_state: OrderState,
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq)]
pub enum OrderState {
	InTrade,
	Revoked,
	Clinchd,
}

pub type OrderId = u64;

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Assets: MultiCurrency<AccountIdOf<Self>>
			+ MultiCurrencyExtended<AccountIdOf<Self>>
			+ MultiLockableCurrency<AccountIdOf<Self>>
			+ MultiReservableCurrency<AccountIdOf<Self>>;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughCurrency,
		NotFindOrderInfo,
		ForbidRevokeOrderNotInTrade,
		ForbidRevokeOrderWithoutOwnership,
		ForbidClinchOrderNotInTrade,
		ForbidClinchOrderWithinOwnership,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The order has been created.
		///
		/// [order_id, order_owner, currency_sold, amount_sold, currency_expected, amount_expected]
		OrderCreated(
			OrderId,
			AccountIdOf<T>,
			CurrencyIdOf<T>,
			BalanceOf<T>,
			CurrencyIdOf<T>,
			BalanceOf<T>,
		),
		/// The order has been revoked.
		///
		/// [order_id_revoked, order_owner]
		OrderRevoked(OrderId, AccountIdOf<T>),
		/// The order has been clinched.
		///
		/// [order_id_clinched, order_owner, order_buyer]
		OrderClinchd(OrderId, AccountIdOf<T>, AccountIdOf<T>),
	}

	#[pallet::storage]
	pub type NextOrderId<T: Config> = StorageValue<_, OrderId, ValueQuery>;

	#[pallet::storage]
	pub type SellerOrders<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		AccountIdOf<T>,
		Twox64Concat,
		CurrencyIdOf<T>,
		Vec<OrderId>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type TotalOrders<T: Config> = StorageMap<_, Twox64Concat, OrderId, OrderInfo<T>>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn create_order(
			origin: OriginFor<T>,
			currency_sold: CurrencyIdOf<T>,
			amount_sold: BalanceOf<T>,
			currency_expected: CurrencyIdOf<T>,
			amount_expected: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			// Check origin
			let owner = ensure_signed(origin)?;

			// Check assets
			let free_balance_currency_sold = T::Assets::free_balance(currency_sold, &owner);
			ensure!(
				free_balance_currency_sold >= amount_sold,
				Error::<T>::NotEnoughCurrency
			);

			// TODO: Lock assets

			// Create order
			let order_id = Self::next_order_id();
			let order_info = OrderInfo::<T> {
				owner: owner.clone(),
				currency_sold,
				amount_sold,
				currency_expected,
				amount_expected,
				order_id,
				order_state: OrderState::InTrade,
			};

			TotalOrders::<T>::insert(order_id, order_info);
			SellerOrders::<T>::mutate(owner.clone(), currency_sold, |orders| orders.push(order_id));

			Self::deposit_event(Event::OrderCreated(
				order_id,
				owner,
				currency_sold,
				amount_sold,
				currency_expected,
				amount_expected,
			));

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn revoke_order(origin: OriginFor<T>, order_id: OrderId) -> DispatchResultWithPostInfo {
			// Check order
			ensure!(
				TotalOrders::<T>::contains_key(order_id),
				Error::<T>::NotFindOrderInfo
			);

			let order_info = TotalOrders::<T>::get(order_id).unwrap();

			// Check order state
			ensure!(
				order_info.order_state == OrderState::InTrade,
				Error::<T>::ForbidRevokeOrderNotInTrade
			);

			// Check origin
			let from = ensure_signed(origin)?;
			ensure!(
				order_info.owner == from,
				Error::<T>::ForbidRevokeOrderWithoutOwnership
			);

			// TODO: Unlock assets

			// Revoke order
			TotalOrders::<T>::mutate(order_id, |oi| match oi {
				Some(oi) => {
					oi.order_state = OrderState::Revoked;
				}
				_ => {}
			});

			Self::deposit_event(Event::OrderRevoked(order_id, from));

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn clinch_order(origin: OriginFor<T>, order_id: OrderId) -> DispatchResultWithPostInfo {
			// Check order
			ensure!(
				TotalOrders::<T>::contains_key(order_id),
				Error::<T>::NotFindOrderInfo
			);

			let order_info = TotalOrders::<T>::get(order_id).unwrap();

			// Check order state
			ensure!(
				order_info.order_state == OrderState::InTrade,
				Error::<T>::ForbidClinchOrderNotInTrade
			);

			// Check origin
			let from = ensure_signed(origin)?;
			ensure!(
				order_info.owner != from,
				Error::<T>::ForbidClinchOrderWithinOwnership
			);

			// TODO: Exchange assets

			// Clinch order
			TotalOrders::<T>::mutate(order_id, |oi| match oi {
				Some(oi) => {
					oi.order_state = OrderState::Clinchd;
				}
				_ => {}
			});

			Self::deposit_event(Event::<T>::OrderClinchd(order_id, order_info.owner, from));

			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub(crate) fn next_order_id() -> OrderId {
		let next_order_id = NextOrderId::<T>::get();
		NextOrderId::<T>::mutate(|current| *current + 1);
		next_order_id
	}
}

#[allow(type_alias_bounds)]
type AccountIdOf<T: Config> = <T as frame_system::Config>::AccountId;
#[allow(type_alias_bounds)]
type BalanceOf<T: Config> = <<T as Config>::Assets as MultiCurrency<AccountIdOf<T>>>::Balance;
#[allow(type_alias_bounds)]
type CurrencyIdOf<T: Config> = <<T as Config>::Assets as MultiCurrency<AccountIdOf<T>>>::CurrencyId;
