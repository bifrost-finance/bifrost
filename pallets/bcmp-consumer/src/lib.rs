#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

extern crate alloc;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use sp_std::vec::Vec;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*, PalletId, transactional};
	use frame_support::dispatch::DispatchErrorWithPostInfo;
	use frame_support::sp_runtime::{SaturatedConversion, traits::AccountIdConversion};
	use sp_core::{H256, U256};
	use frame_support::traits::{Currency, ExistenceRequirement, LockableCurrency};
	use frame_system::pallet_prelude::*;
	use pallet_bcmp::Message;

	pub type BalanceOf<T> =
	<<T as pallet_bcmp::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	const RESOURCE_ACCOUNT: PalletId = PalletId(*b"ResrcAcc");
	const MAX_ACCOUNT_LENGTH: usize = 32;
	const AMOUNT_LENGTH: usize = 32;

	#[derive(RuntimeDebug, Clone, Eq, PartialEq, Encode, Decode, TypeInfo)]
	pub struct Payload<T: Config> {
		pub amount: BalanceOf<T>,
		pub receiver: T::AccountId,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_bcmp::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
		/// Address represent this pallet, ie 'keccak256(&b"PALLET_CONSUMER"))'
		#[pallet::constant]
		type AnchorAddress: Get<H256>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::event]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {
		BalanceConvertFailed,
		AccountConvertFailed,
		InvalidPayloadLength,
		UnsupportedReceiverLength,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Generate cross tx, will call 'Bcmp::send_message' to emit 'MessageSent' event.
		#[pallet::weight({0})]
		#[pallet::call_index(0)]
		#[transactional]
		pub fn send_message(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			fee: BalanceOf<T>,
			dst_chain: u32,
			receiver: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			ensure!(receiver.len() <= MAX_ACCOUNT_LENGTH, Error::<T>::UnsupportedReceiverLength);
			<T as pallet_bcmp::Config>::Currency::transfer(
				&sender,
				&Self::resource_account(),
				amount,
				ExistenceRequirement::AllowDeath,
			)?;
			let payload = Self::eth_api_encode(amount, &receiver);
			let src_anchor = T::AnchorAddress::get();
			pallet_bcmp::Pallet::<T>::send_message(
				sender,
				fee,
				src_anchor,
				dst_chain,
				payload,
			)
		}
	}

	impl<T: Config> Pallet<T> {
		/// Resource account to lock balance.
		pub(crate) fn resource_account() -> T::AccountId {
			RESOURCE_ACCOUNT.into_account_truncating()
		}

		/// Example for parsing payload from Evm payload, contains 'amount' and 'receiver'.
		pub(crate) fn parse_payload(raw: &[u8]) -> Result<Payload<T>, DispatchErrorWithPostInfo> {
			return if raw.len() == MAX_ACCOUNT_LENGTH + AMOUNT_LENGTH {
				let amount: u128 = U256::from_big_endian(&raw[..32])
					.try_into()
					.map_err(|_| Error::<T>::BalanceConvertFailed)?;
				// account id decode may different, ie. 'AccountId20', 'AccountId32', ..
				let account_len = T::AccountId::max_encoded_len();
				if account_len >= raw.len() {
					return Err(Error::<T>::AccountConvertFailed.into())
				}
				let receiver = T::AccountId::decode(&mut raw[raw.len() - account_len..].as_ref())
					.map_err(|_| Error::<T>::AccountConvertFailed)?;
				Ok(
					Payload {
						amount: SaturatedConversion::saturated_from(amount),
						receiver,
					}
				)
			} else {
				Err(Error::<T>::InvalidPayloadLength.into())
			}
		}

		/// Example for encoding to Evm payload.
		pub(crate) fn eth_api_encode(amount: BalanceOf<T>, receiver: &[u8]) -> Vec<u8> {
			let mut fixed_amount= [0u8; 32];
			U256::from(amount.saturated_into::<u128>()).to_big_endian(&mut fixed_amount);
			let mut payload = fixed_amount.to_vec();
			let mut fixed_address = Self::extend_to_bytes32(receiver, 32);
			payload.append(&mut fixed_address);
			payload
		}

		/// Extend bytes to target length.
		pub(crate) fn extend_to_bytes32(data: &[u8], size: usize) -> Vec<u8> {
			let mut append = Vec::new();
			let mut len = data.len();
			while len < size {
				append.push(0);
				len += 1;
			}
			append.append(&mut data.to_vec());
			append
		}
	}

	impl<T: Config> pallet_bcmp::ConsumerLayer<T> for Pallet<T> {
		/// Called by 'Bcmp::receive_message', has already verified committee's signature.
		fn receive_op(message: &Message) -> DispatchResultWithPostInfo {
			let payload = Self::parse_payload(&message.payload)?;
			<T as pallet_bcmp::Config>::Currency::transfer(
				&Self::resource_account(),
				&payload.receiver,
				payload.amount,
				ExistenceRequirement::AllowDeath,
			)?;
			Ok(().into())
		}

		fn anchor_addr() -> H256 {
			T::AnchorAddress::get()
		}
	}
}
