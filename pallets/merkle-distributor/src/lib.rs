#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    pallet_prelude::*,
    sp_runtime::traits::{
        AccountIdConversion, AtLeast32BitUnsigned, Keccak256, One, Saturating, StaticLookup, Zero,
    },
    sp_std::{convert::{TryInto, TryFrom}, vec::Vec},
    transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::{Hasher, H256};

use orml_traits::MultiCurrency;

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod default_weights;
pub use default_weights::WeightInfo;

#[allow(type_alias_bounds)]
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, MaxEncodedLen, RuntimeDebug, TypeInfo)]
pub struct MerkleMetadata<Balance, CurrencyId, AccountId, BoundString> {
    /// The merkle tree root
    pub merkle_root: H256,
    /// Describe usage of the merkle root
    pub description: BoundString,
    /// The distributed currency
    pub distribute_currency: CurrencyId,
    /// The amount of distributed currency
    pub distribute_amount: Balance,
    /// The account holder distributed currency
    pub distribute_holder: AccountId,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + TypeInfo {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency ID type
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord + TypeInfo;

        type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId=Self::CurrencyId, Balance=Self::Balance>;

        /// The balance type
        type Balance: Parameter
        + Member
        + AtLeast32BitUnsigned
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + MaxEncodedLen;

        /// Identifier for the class of merkle distributor.
        type MerkleDistributorId: Member
        + Parameter
        + Default
        + Copy
        + MaxEncodedLen
        + One
        + Saturating;

        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The maximum length of a merkel description stored on-chain.
        #[pallet::constant]
        type StringLimit: Get<u32>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::storage]
    #[pallet::getter(fn get_merkle_distributor)]
    pub(super) type MerkleDistributorMetadata<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::MerkleDistributorId,
        MerkleMetadata<T::Balance, T::CurrencyId, T::AccountId, BoundedVec<u8, T::StringLimit>>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn merkle_distributor_id)]
    pub(crate) type NextMerkleDistributorId<T: Config> =
    StorageValue<_, T::MerkleDistributorId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn cliamed_bitmap)]
    pub(crate) type ClaimedBitMap<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        T::MerkleDistributorId,
        Twox64Concat,
        u32,
        u32,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub (crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// create a merkle distributor. \[merkle distributor id, merkle tree root, total reward balance]
        Create(T::MerkleDistributorId, H256, T::Balance),
        /// claim reward. \[merkle distributor id, account, balance]
        Claim(T::MerkleDistributorId, T::AccountId, u128),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Invalid metadata given.
        BadDescription,
        /// The id is not exist.
        InvalidMerkleDistributorId,
        /// The proof is invalid
        MerkleVerifyFailed,
        /// The reward is already distributed.
        Claimed,
        /// The reward is already charged.
        Charged,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// `create_merkle_distributor` will create a merkle distributor,
        ///  which allow specified users claim asset.
        ///
        /// The dispatch origin for this call must be `Signed` by root.
        ///
        /// - `merkle_root`: The root of a merkle tree.
        /// - `description`: About the purpose of this distribution.
        /// - `distribute_currency`: The id of currency about this distribution.
        /// - `distribute_amount`: The total currency amount of this distribution.
        #[pallet::weight(T::WeightInfo::create_merkle_distributor())]
        pub fn create_merkle_distributor(
            origin: OriginFor<T>,
            merkle_root: H256,
            description: Vec<u8>,
            distribute_currency: T::CurrencyId,
            distribute_amount: T::Balance,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let merkle_distributor_id = Self::next_merkle_distributor_id();
            let distribute_holder: AccountIdOf<T> =
                T::PalletId::get().into_sub_account(merkle_distributor_id);

            let description: BoundedVec<u8, T::StringLimit> = description
                .try_into()
                .map_err(|_| Error::<T>::BadDescription)?;

            MerkleDistributorMetadata::<T>::insert(
                merkle_distributor_id,
                MerkleMetadata {
                    merkle_root,
                    description,
                    distribute_currency,
                    distribute_amount,
                    distribute_holder,
                },
            );

            Self::deposit_event(Event::<T>::Create(merkle_distributor_id, merkle_root, distribute_amount));
            
            Ok(())
        }

        /// `claim` Claim rewards through user information and merkle proof.
        ///
        /// - `merkle_distributor_id`: ID of a merkle distributor.
        /// - `index`: The index of the merkle tree leaf.
        /// - `account`: The owner's account of merkle proof.
        /// - `merkle_proof`: The hashes with merkle tree leaf can get merkle tree root.
        #[pallet::weight(T::WeightInfo::claim())]
        #[transactional]
        pub fn claim(
            origin: OriginFor<T>,
            merkle_distributor_id: T::MerkleDistributorId,
            index: u32,
            account: <T::Lookup as StaticLookup>::Source,
            amount: u128,
            merkle_proof: Vec<H256>,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            ensure!(
                !Self::is_claimed(merkle_distributor_id, index),
                Error::<T>::Claimed
            );

            let owner = T::Lookup::lookup(account)?;

            let mut index_data = Vec::<u8>::from(index.to_be_bytes());
            let mut balance_data = Vec::<u8>::from(amount.to_be_bytes());

            index_data.append(&mut owner.encode());
            index_data.append(&mut balance_data);

            let node: H256 = Keccak256::hash(&index_data);

            let merkle = Self::get_merkle_distributor(merkle_distributor_id)
                .ok_or(Error::<T>::InvalidMerkleDistributorId)?;

            ensure!(
                Self::verify_merkle_proof(&merkle_proof, merkle.merkle_root, node),
                Error::<T>::MerkleVerifyFailed
            );

            T::MultiCurrency::transfer(
                merkle.distribute_currency,
                &merkle.distribute_holder,
                &owner,
                T::Balance::try_from(amount).unwrap_or_else(|_| Zero::zero()),
            )?;

            Self::set_claimed(merkle_distributor_id, index);

            Self::deposit_event(Event::<T>::Claim(merkle_distributor_id, owner, amount));
            Ok(())
        }

        /// Charge currency to the account of merkle distributor
        ///
        /// `merkle_distributor_id`: ID of a merkle distributor.
        #[pallet::weight(T::WeightInfo::charge())]
        #[transactional]
        pub fn charge(
            origin: OriginFor<T>,
            merkle_distributor_id: T::MerkleDistributorId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let merkle = Self::get_merkle_distributor(merkle_distributor_id)
                .ok_or(Error::<T>::InvalidMerkleDistributorId)?;
            ensure!(
                T::MultiCurrency::free_balance(
                    merkle.distribute_currency,
                    &merkle.distribute_holder
                ) < merkle.distribute_amount,
                Error::<T>::Charged
            );

            T::MultiCurrency::transfer(
                merkle.distribute_currency,
                &who,
                &merkle.distribute_holder,
                merkle.distribute_amount,
            )?;

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub(crate) fn next_merkle_distributor_id() -> T::MerkleDistributorId {
            let next_merkle_distributor_id = Self::merkle_distributor_id();
            NextMerkleDistributorId::<T>::mutate(|current| {
                *current = current.saturating_add(One::one())
            });
            next_merkle_distributor_id
        }

        pub(crate) fn verify_merkle_proof(
            merkle_proof: &[H256],
            merkle_root: H256,
            leaf: H256,
        ) -> bool {
            let mut computed_hash = leaf;

            for (i, _) in merkle_proof.iter().enumerate() {
                let proof_element = merkle_proof[i];
                if computed_hash <= proof_element {
                    // Hash(current computed hash + current element of the proof)
                    let mut pack = computed_hash.encode();
                    pack.append(&mut proof_element.encode());
                    computed_hash = Keccak256::hash(&pack);
                } else {
                    // Hash(current element of the proof + current computed hash)
                    let mut pack = proof_element.encode();
                    pack.append(&mut computed_hash.encode());
                    computed_hash = Keccak256::hash(&pack);
                }
            }

            computed_hash == merkle_root
        }

        pub(crate) fn set_claimed(merkle_distributor_id: T::MerkleDistributorId, index: u32) {
            let claimed_word_index: u32 = index / 32;
            let claimed_bit_index = index % 32;

            let old_value = Self::cliamed_bitmap(merkle_distributor_id, claimed_word_index);
            ClaimedBitMap::<T>::insert(
                merkle_distributor_id,
                claimed_word_index,
                old_value | (1 << claimed_bit_index),
            );
        }

        pub(crate) fn is_claimed(
            merkle_distributor_id: T::MerkleDistributorId,
            index: u32,
        ) -> bool {
            let claimed_word_index: u32 = index / 32;
            let claimed_bit_index = index % 32;

            let claimed_word = Self::cliamed_bitmap(merkle_distributor_id, claimed_word_index);
            let mask: u32 = 1 << claimed_bit_index;
            claimed_word & mask == mask
        }
    }
}
