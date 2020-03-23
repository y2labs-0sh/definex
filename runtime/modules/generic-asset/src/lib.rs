// Copyright (C) 2020 by definex.io

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, Error as CodecError, HasCompact, Input, Output};

use sp_runtime::traits::{
    AtLeast32Bit, Bounded, CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member, One,
    Saturating, Zero,
};
use sp_runtime::{DispatchError, DispatchResult, RuntimeDebug};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    traits::{
        BalanceStatus, Currency, ExistenceRequirement, Imbalance, LockIdentifier, LockableCurrency,
        ReservableCurrency, SignedImbalance, TryDrop, WithdrawReason, WithdrawReasons,
    },
    Parameter, StorageMap,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_std::prelude::*;
use sp_std::{cmp, fmt::Debug, result};

mod mock;
mod tests;

pub trait Trait: frame_system::Trait + sudo::Trait {
    type Balance: Parameter
        + Member
        + AtLeast32Bit
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug;
    type AssetId: Parameter + Member + AtLeast32Bit + Default + Copy;
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

pub trait Subtrait: frame_system::Trait {
    type Balance: Parameter
        + Member
        + AtLeast32Bit
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug;
    type AssetId: Parameter + Member + AtLeast32Bit + Default + Copy;
}

impl<T: Trait> Subtrait for T {
    type Balance = T::Balance;
    type AssetId = T::AssetId;
}

/// Asset creation options.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
pub struct AssetOptions<Balance: HasCompact, AccountId> {
    /// Initial issuance of this asset. All deposit to the creator of the asset.
    #[codec(compact)]
    pub initial_issuance: Balance,
    /// Which accounts are allowed to possess this asset.
    pub permissions: PermissionLatest<AccountId>,
}

/// Owner of an asset.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
pub enum Owner<AccountId> {
    /// No owner.
    None,
    /// Owned by an AccountId
    Address(AccountId),
}

impl<AccountId> Default for Owner<AccountId> {
    fn default() -> Self {
        Owner::None
    }
}

/// Asset permissions
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
pub struct PermissionsV1<AccountId> {
    /// Who have permission to update asset permission
    pub update: Owner<AccountId>,
    /// Who have permission to mint new asset
    pub mint: Owner<AccountId>,
    /// Who have permission to burn asset
    pub burn: Owner<AccountId>,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
#[repr(u8)]
enum PermissionVersionNumber {
    V1 = 0,
}

/// Versioned asset permission
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub enum PermissionVersions<AccountId> {
    V1(PermissionsV1<AccountId>),
}

/// Asset permission types
pub enum PermissionType {
    /// Permission to burn asset permission
    Burn,
    /// Permission to mint new asset
    Mint,
    /// Permission to update asset
    Update,
}

/// Alias to latest asset permissions
pub type PermissionLatest<AccountId> = PermissionsV1<AccountId>;

impl<AccountId> Default for PermissionVersions<AccountId> {
    fn default() -> Self {
        PermissionVersions::V1(Default::default())
    }
}

impl<AccountId: Encode> Encode for PermissionVersions<AccountId> {
    fn encode_to<T: Output>(&self, dest: &mut T) {
        match self {
            PermissionVersions::V1(payload) => {
                dest.push(&PermissionVersionNumber::V1);
                dest.push(payload);
            }
        }
    }
}

impl<AccountId: Encode> codec::EncodeLike for PermissionVersions<AccountId> {}

impl<AccountId: Decode> Decode for PermissionVersions<AccountId> {
    fn decode<I: Input>(input: &mut I) -> core::result::Result<Self, CodecError> {
        let version = PermissionVersionNumber::decode(input)?;
        Ok(match version {
            PermissionVersionNumber::V1 => PermissionVersions::V1(Decode::decode(input)?),
        })
    }
}

impl<AccountId> Default for PermissionsV1<AccountId> {
    fn default() -> Self {
        PermissionsV1 {
            update: Owner::None,
            mint: Owner::None,
            burn: Owner::None,
        }
    }
}

impl<AccountId> Into<PermissionLatest<AccountId>> for PermissionVersions<AccountId> {
    fn into(self) -> PermissionLatest<AccountId> {
        match self {
            PermissionVersions::V1(v1) => v1,
        }
    }
}

/// Converts the latest permission to other version.
impl<AccountId> Into<PermissionVersions<AccountId>> for PermissionLatest<AccountId> {
    fn into(self) -> PermissionVersions<AccountId> {
        PermissionVersions::V1(self)
    }
}

decl_error! {
    /// Error for the generic-asset module.
    pub enum Error for Module<T: Trait> {
        /// No new assets id available.
        NoIdAvailable,
        /// Cannot transfer zero amount.
        ZeroAmount,
        /// The origin does not have enough permission to update permissions.
        NoUpdatePermission,
        /// The origin does not have permission to mint an asset.
        NoMintPermission,
        /// The origin does not have permission to burn an asset.
        NoBurnPermission,
        /// Total issuance got overflowed after minting.
        TotalMintingOverflow,
        /// Free balance got overflowed after minting.
        FreeMintingOverflow,
        /// Total issuance got underflowed after burning.
        TotalBurningUnderflow,
        /// Free balance got underflowed after burning.
        FreeBurningUnderflow,
        /// Asset id is already taken.
        IdAlreadyTaken,
        /// Asset id not available.
        IdUnavailable,
        /// The balance is too low to send amount.
        InsufficientBalance,
        /// The account liquidity restrictions prevent withdrawal.
        LiquidityRestrictions,
        /// The reserved balance is too low
        InsufficientReservedBalance,
        /// The exact lock of the balance amount is not found
        NoSuchLock,
        /// Lock Id is unknown
        MissingLock,
        /// No lock meets the required amount
        NoLockMeetRequirement,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// Create a new kind of asset.
        fn create(origin, initial_issuance: T::Balance, symbol: Vec<u8>) -> DispatchResult {
            ensure_root(origin)?;
            let root_account_id = <sudo::Module<T>>::key();

            let options = AssetOptions {
                initial_issuance:initial_issuance,
                permissions: PermissionLatest {
                    update: Owner::Address(root_account_id.clone()),
                    mint: Owner::Address(root_account_id.clone()),
                    burn: Owner::Address(root_account_id.clone()),
                },
            };

            let asset_id = Self::next_asset_id();
            Self::create_asset(None, Some(root_account_id), options)?;
            <Symbols<T>>::insert(asset_id, symbol.clone());
            Ok(())
        }

        /// Transfer some liquid free balance to another account.
        pub fn transfer(origin, #[compact] asset_id: T::AssetId, to: T::AccountId, #[compact] amount: T::Balance) {
            let origin = ensure_signed(origin)?;
            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);
            Self::make_transfer_with_event(&asset_id, &origin, &to, amount)?;
        }

        /// Updates permission for a given `asset_id` and an account.
        ///
        /// The `origin` must have `update` permission.
        fn update_permission(
            origin,
            #[compact] asset_id: T::AssetId,
            new_permission: PermissionLatest<T::AccountId>
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;

            let permissions: PermissionVersions<T::AccountId> = new_permission.into();

            if Self::check_permission(&asset_id, &origin, &PermissionType::Update) {
                <Permissions<T>>::insert(asset_id, &permissions);

                Self::deposit_event(RawEvent::PermissionUpdated(asset_id, permissions.into()));

                Ok(())
            } else {
                Err(Error::<T>::NoUpdatePermission)?
            }
        }

        /// Mints an asset, increases its total issuance.
        /// The origin must have `mint` permissions.
        fn mint(origin, #[compact] asset_id: T::AssetId, to: T::AccountId, amount: T::Balance) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::mint_free(&asset_id, &who, &to, &amount)?;
            Self::deposit_event(RawEvent::Minted(asset_id, to, amount));
            Ok(())
        }

        /// Burns an asset, decreases its total issuance.
        /// The `origin` must have `burn` permissions.
        fn burn(origin, #[compact] asset_id: T::AssetId, to: T::AccountId, amount: T::Balance) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::burn_free(&asset_id, &who, &to, &amount)?;
            Self::deposit_event(RawEvent::Burned(asset_id, to, amount));
            Ok(())
        }

        /// Can be used to create reserved tokens.
        /// Requires Root call.
        fn create_reserved(
            origin,
            asset_id: T::AssetId,
            options: AssetOptions<T::Balance, T::AccountId>
        ) -> DispatchResult {
            ensure_root(origin)?;
            Self::create_asset(Some(asset_id), None, options)
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct BalanceLock<Balance, AssetId> {
    pub id: u128,
    pub asset_id: AssetId,
    pub amount: Balance,
    pub reasons: WithdrawReasons,
}

decl_storage! {
    trait Store for Module<T: Trait> as GenericAsset {
        /// Total issuance of a given asset.
        pub TotalIssuance get(fn total_issuance) build(|config: &GenesisConfig<T>| {
            let issuance = config.initial_balance * (config.endowed_accounts.len() as u32).into();
            config.assets.iter().map(|id| (id.clone(), issuance)).collect::<Vec<_>>()
        }): map hasher(blake2_256) T::AssetId => T::Balance;

        /// The free balance of a given asset under an account.
        pub FreeBalance:
            double_map hasher(blake2_256) T::AssetId, hasher(twox_128) T::AccountId => T::Balance;

        /// The reserved balance of a given asset under an account.
        pub ReservedBalance:
            double_map hasher(blake2_256) T::AssetId, hasher(twox_128) T::AccountId => T::Balance;

        /// "Symbols" can only keep Vec<u8>, and utf8 safty is totally on the client side
        pub Symbols get(symbols) config() : map hasher(blake2_256) T::AssetId => Vec<u8>;

        /// Next available ID for user-created asset.
        pub NextAssetId get(fn next_asset_id) config(): T::AssetId;

        /// Permission options for a given asset.
        pub Permissions get(fn get_permission):
            map hasher(blake2_256) T::AssetId => PermissionVersions<T::AccountId>;

        /// Any liquidity locks on some account balances.
        pub NextLockId get(fn next_lock_id) : u128 = 1;
        pub Locks get(fn locks):
            double_map hasher(blake2_256) LockIdentifier, hasher(blake2_256) T::AccountId => Vec<BalanceLock<T::Balance, T::AssetId>>;
    }

    add_extra_genesis {
        config(assets): Vec<T::AssetId>;
        config(initial_balance): T::Balance;
        config(endowed_accounts): Vec<T::AccountId>;

        build(|config: &GenesisConfig<T>| {
            config.assets.iter().for_each(|asset_id| {
                config.endowed_accounts.iter().for_each(|account_id| {
                    <FreeBalance<T>>::insert(asset_id, account_id, &config.initial_balance);
                });
            });

            let root_id = <sudo::Module<T>>::key();
            let options = AssetOptions {
                initial_issuance: T::Balance::from(0),
                permissions: PermissionLatest {
                    update: Owner::Address(root_id.clone()),
                    mint: Owner::Address(root_id.clone()),
                    burn: Owner::Address(root_id.clone()),
                },
            };
            for i in &config.symbols {
                <Module<T>>::create_asset(Some(i.0), Some(root_id.clone()), options.clone()).unwrap();
            }
        });
    }
}

decl_event!(
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		<T as Trait>::Balance,
		<T as Trait>::AssetId,
		AssetOptions = AssetOptions<<T as Trait>::Balance, <T as frame_system::Trait>::AccountId>
	{
		  /// Asset created (asset_id, creator, asset_options).
		  Created(AssetId, AccountId, AssetOptions),
		  /// Asset transfer succeeded (asset_id, from, to, amount).
		  Transferred(AssetId, AccountId, AccountId, Balance),
		  /// Asset permission updated (asset_id, new_permissions).
		  PermissionUpdated(AssetId, PermissionLatest<AccountId>),
		  /// New asset minted (asset_id, account, amount).
		  Minted(AssetId, AccountId, Balance),
		  /// Asset burned (asset_id, account, amount).
		  Burned(AssetId, AccountId, Balance),
      ///
      Reserved(AssetId, AccountId, Balance, u128),
      ///
      Unreserved(AssetId, AccountId, Balance, u128),
	}
);

impl<T: Trait> Module<T> {
    // PUBLIC IMMUTABLES

    /// Get an account's total balance of an asset kind.
    pub fn total_balance(asset_id: &T::AssetId, who: &T::AccountId) -> T::Balance {
        Self::free_balance(asset_id, who) + Self::reserved_balance(asset_id, who)
    }

    /// Get an account's free balance of an asset kind.
    pub fn free_balance(asset_id: &T::AssetId, who: &T::AccountId) -> T::Balance {
        <FreeBalance<T>>::get(asset_id, who)
    }

    /// Get an account's reserved balance of an asset kind.
    pub fn reserved_balance(asset_id: &T::AssetId, who: &T::AccountId) -> T::Balance {
        <ReservedBalance<T>>::get(asset_id, who)
    }

    /// Mint to an account's free balance, without event
    pub fn mint_free(
        asset_id: &T::AssetId,
        who: &T::AccountId,
        to: &T::AccountId,
        amount: &T::Balance,
    ) -> DispatchResult {
        if Self::check_permission(asset_id, who, &PermissionType::Mint) {
            let original_free_balance = Self::free_balance(&asset_id, &to);
            let current_total_issuance = <TotalIssuance<T>>::get(asset_id);
            let new_total_issuance = current_total_issuance
                .checked_add(&amount)
                .ok_or(Error::<T>::TotalMintingOverflow)?;
            let value = original_free_balance
                .checked_add(&amount)
                .ok_or(Error::<T>::FreeMintingOverflow)?;

            <TotalIssuance<T>>::insert(asset_id, new_total_issuance);
            Self::set_free_balance(&asset_id, &to, value);
            Ok(())
        } else {
            Err(Error::<T>::NoMintPermission)?
        }
    }

    /// Burn an account's free balance, without event
    pub fn burn_free(
        asset_id: &T::AssetId,
        who: &T::AccountId,
        to: &T::AccountId,
        amount: &T::Balance,
    ) -> DispatchResult {
        if Self::check_permission(asset_id, who, &PermissionType::Burn) {
            let original_free_balance = Self::free_balance(asset_id, to);

            let current_total_issuance = <TotalIssuance<T>>::get(asset_id);
            let new_total_issuance = current_total_issuance
                .checked_sub(amount)
                .ok_or(Error::<T>::TotalBurningUnderflow)?;
            let value = original_free_balance
                .checked_sub(amount)
                .ok_or(Error::<T>::FreeBurningUnderflow)?;

            <TotalIssuance<T>>::insert(asset_id, new_total_issuance);
            Self::set_free_balance(asset_id, to, value);
            Ok(())
        } else {
            Err(Error::<T>::NoBurnPermission)?
        }
    }

    /// Creates an asset.
    ///
    /// # Arguments
    /// * `asset_id`: An ID of a reserved asset.
    /// If not provided, a user-generated asset will be created with the next available ID.
    /// * `from_account`: The initiator account of this call
    /// * `asset_options`: Asset creation options.
    ///
    pub fn create_asset(
        asset_id: Option<T::AssetId>,
        from_account: Option<T::AccountId>,
        options: AssetOptions<T::Balance, T::AccountId>,
    ) -> DispatchResult {
        let asset_id = if let Some(asset_id) = asset_id {
            ensure!(
                !<TotalIssuance<T>>::contains_key(&asset_id),
                Error::<T>::IdAlreadyTaken
            );
            ensure!(asset_id < Self::next_asset_id(), Error::<T>::IdUnavailable);
            asset_id
        } else {
            let asset_id = Self::next_asset_id();
            let next_id = asset_id
                .checked_add(&One::one())
                .ok_or(Error::<T>::NoIdAvailable)?;
            <NextAssetId<T>>::put(next_id);
            asset_id
        };

        let account_id = from_account.unwrap_or_default();
        let permissions: PermissionVersions<T::AccountId> = options.permissions.clone().into();

        <TotalIssuance<T>>::insert(asset_id, &options.initial_issuance);
        <FreeBalance<T>>::insert(&asset_id, &account_id, &options.initial_issuance);
        <Permissions<T>>::insert(&asset_id, permissions);

        Self::deposit_event(RawEvent::Created(asset_id, account_id, options));

        Ok(())
    }

    /// Transfer some liquid free balance from one account to another.
    /// This will not emit the `Transferred` event.
    pub fn make_transfer(
        asset_id: &T::AssetId,
        from: &T::AccountId,
        to: &T::AccountId,
        amount: T::Balance,
    ) -> DispatchResult {
        ensure!(
            Self::free_balance(asset_id, from) >= amount,
            Error::<T>::InsufficientBalance
        );

        if from != to {
            <FreeBalance<T>>::mutate(asset_id, from, |balance| *balance -= amount);
            <FreeBalance<T>>::mutate(asset_id, to, |balance| *balance += amount);
        }

        Ok(())
    }

    /// Transfer some liquid free balance from one account to another.
    /// This will emit the `Transferred` event.
    pub fn make_transfer_with_event(
        asset_id: &T::AssetId,
        from: &T::AccountId,
        to: &T::AccountId,
        amount: T::Balance,
    ) -> DispatchResult {
        Self::make_transfer(asset_id, from, to, amount)?;

        if from != to {
            Self::deposit_event(RawEvent::Transferred(
                *asset_id,
                from.clone(),
                to.clone(),
                amount,
            ));
        }

        Ok(())
    }

    /// Move `amount` from free balance to reserved balance.
    ///
    /// If the free balance is lower than `amount`, then no funds will be moved and an `Err` will
    /// be returned. This is different behavior than `unreserve`.
    pub fn reserve(
        asset_id: &T::AssetId,
        who: &T::AccountId,
        amount: T::Balance,
    ) -> Result<u128, &'static str> {
        // Do we need to consider that this is an atomic transaction?
        let original_reserve_balance = Self::reserved_balance(asset_id, who);
        let original_free_balance = Self::free_balance(asset_id, who);
        // if original_free_balance < amount {
        //     Err(Error::<T>::InsufficientBalance)?
        // }
        ensure!(
            original_free_balance >= amount,
            Error::<T>::InsufficientBalance
        );

        let new_reserve_balance = original_reserve_balance + amount;
        Self::set_reserved_balance(asset_id, who, new_reserve_balance);
        let new_free_balance = original_free_balance - amount;
        Self::set_free_balance(asset_id, who, new_free_balance);

        let lock_id = Self::set_lock(asset_id, who, amount, WithdrawReason::Reserve.into());

        Self::deposit_event(RawEvent::Reserved(
            asset_id.clone(),
            who.clone(),
            amount,
            lock_id,
        ));

        Ok(lock_id)
    }

    /// Moves up to `amount` from reserved balance to free balance. This function cannot fail.
    ///
    /// As many assets up to `amount` will be moved as possible. If the reserve balance of `who`
    /// is less than `amount`, then the remaining amount will be returned.
    /// NOTE: This is different behavior than `reserve`.
    pub fn unreserve(
        asset_id: &T::AssetId,
        who: &T::AccountId,
        amount: T::Balance,
        lock_id: Option<u128>,
    ) -> Result<(), &'static str> {
        let reserved_balance = Self::reserved_balance(asset_id, who);
        ensure!(
            reserved_balance >= amount,
            Error::<T>::InsufficientReservedBalance
        );

        let mut valid_lock_id = None;
        let idf = Self::generic_asset_lock_identifier(asset_id);
        match lock_id {
            Some(lock_id) => {
                if !Self::lock_id_exists(idf, who, lock_id) {
                    return Err(Error::<T>::MissingLock.into());
                } else {
                    valid_lock_id = Some(lock_id);
                }
            }
            None => {
                if let Some(lock_id) = Self::find_lock_id_by_amount(idf, who, amount) {
                    valid_lock_id = Some(lock_id);
                } else {
                    return Err(Error::<T>::NoLockMeetRequirement.into());
                }
            }
        }

        Self::remove_lock(idf, who, valid_lock_id.unwrap());
        let original_free_balance = Self::free_balance(asset_id, who);
        let new_free_balance = original_free_balance + amount;
        Self::set_free_balance(asset_id, who, new_free_balance);
        Self::set_reserved_balance(asset_id, who, reserved_balance - amount);

        Self::deposit_event(RawEvent::Unreserved(
            asset_id.clone(),
            who.clone(),
            amount,
            valid_lock_id.unwrap(),
        ));

        Ok(())
    }

    /// Deduct up to `amount` from the combined balance of `who`, preferring to deduct from the
    /// free balance. This function cannot fail.
    ///
    /// As much funds up to `amount` will be deducted as possible. If this is less than `amount`
    /// then `Some(remaining)` will be returned. Full completion is given by `None`.
    /// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is expected that
    /// the caller will do this.
    pub fn slash(
        asset_id: &T::AssetId,
        who: &T::AccountId,
        amount: T::Balance,
    ) -> Option<T::Balance> {
        // let free_balance = Self::free_balance(asset_id, who);
        // let free_slash = sp_std::cmp::min(free_balance, amount);
        // let new_free_balance = free_balance - free_slash;
        // Self::set_free_balance(asset_id, who, new_free_balance);
        // if free_slash < amount {
        //     Self::slash_reserved(asset_id, who, amount - free_slash)
        // } else {
        //     None
        // }
        None
    }

    /// Deducts up to `amount` from reserved balance of `who`. This function cannot fail.
    ///
    /// As much funds up to `amount` will be deducted as possible. If the reserve balance of `who`
    /// is less than `amount`, then a non-zero second item will be returned.
    /// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is expected that
    /// the caller will do this.
    // pub fn slash_reserved(
    //     asset_id: &T::AssetId,
    //     who: &T::AccountId,
    //     amount: T::Balance,
    // ) -> Option<T::Balance> {
    //     let original_reserve_balance = Self::reserved_balance(asset_id, who);
    //     let slash = sp_std::cmp::min(original_reserve_balance, amount);
    //     let new_reserve_balance = original_reserve_balance - slash;
    //     Self::set_reserved_balance(asset_id, who, new_reserve_balance);
    //     if amount == slash {
    //         None
    //     } else {
    //         Some(amount - slash)
    //     }
    // }

    /// Move up to `amount` from reserved balance of account `who` to balance of account
    /// `beneficiary`, either free or reserved depending on `status`.
    ///
    /// As much funds up to `amount` will be moved as possible. If this is less than `amount`, then
    /// the `remaining` would be returned, else `Zero::zero()`.
    /// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is expected that
    /// the caller will do this.
    pub fn repatriate_reserved(
        asset_id: &T::AssetId,
        who: &T::AccountId,
        beneficiary: &T::AccountId,
        amount: T::Balance,
        status: BalanceStatus,
    ) -> T::Balance {
        // let b = Self::reserved_balance(asset_id, who);
        // let slash = sp_std::cmp::min(b, amount);

        // match status {
        //     BalanceStatus::Free => {
        //         let original_free_balance = Self::free_balance(asset_id, beneficiary);
        //         let new_free_balance = original_free_balance + slash;
        //         Self::set_free_balance(asset_id, beneficiary, new_free_balance);
        //     }
        //     BalanceStatus::Reserved => {
        //         let original_reserved_balance = Self::reserved_balance(asset_id, beneficiary);
        //         let new_reserved_balance = original_reserved_balance + slash;
        //         Self::set_reserved_balance(asset_id, beneficiary, new_reserved_balance);
        //     }
        // }

        // let new_reserve_balance = b - slash;
        // Self::set_reserved_balance(asset_id, who, new_reserve_balance);
        // amount - slash
        0.into()
    }

    /// Check permission to perform burn, mint or update.
    ///
    /// # Arguments
    /// * `asset_id`:  A `T::AssetId` type that contains the `asset_id`, which has the permission embedded.
    /// * `who`: A `T::AccountId` type that contains the `account_id` for which to check permissions.
    /// * `what`: The permission to check.
    ///
    pub fn check_permission(
        asset_id: &T::AssetId,
        who: &T::AccountId,
        what: &PermissionType,
    ) -> bool {
        let permission_versions: PermissionVersions<T::AccountId> = Self::get_permission(asset_id);
        let permission = permission_versions.into();

        match (what, permission) {
            (
                PermissionType::Burn,
                PermissionLatest {
                    burn: Owner::Address(account),
                    ..
                },
            ) => account == *who,
            (
                PermissionType::Mint,
                PermissionLatest {
                    mint: Owner::Address(account),
                    ..
                },
            ) => account == *who,
            (
                PermissionType::Update,
                PermissionLatest {
                    update: Owner::Address(account),
                    ..
                },
            ) => account == *who,
            _ => false,
        }
    }

    /// Return `Ok` iff the account is able to make a withdrawal of the given amount
    /// for the given reason.
    ///
    /// `Err(...)` with the reason why not otherwise.
    pub fn ensure_can_withdraw(
        _asset_id: &T::AssetId,
        _who: &T::AccountId,
        _amount: T::Balance,
        _reasons: WithdrawReasons,
        _new_balance: T::Balance,
    ) -> DispatchResult {
        // if asset_id != &Self::staking_asset_id() {
        //     return Ok(());
        // }

        // let locks = Self::locks(who);
        // if locks.is_empty() {
        //     return Ok(());
        // }
        // if Self::locks(who)
        //     .into_iter()
        //     .all(|l| new_balance >= l.amount || !l.reasons.intersects(reasons))
        // {
        //     Ok(())
        // } else {
        //     Err(Error::<T>::LiquidityRestrictions)?
        // }
        Ok(())
    }

    fn take_lock_id() -> u128 {
        let id = Self::next_lock_id();
        NextLockId::mutate(|v| {
            *v += 1;
        });
        id
    }

    pub fn generic_asset_lock_identifier(asset_id: &T::AssetId) -> LockIdentifier {
        use sp_io::hashing::twox_64;
        twox_64(&asset_id.encode())
    }

    // PRIVATE MUTABLES

    /// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is expected that
    /// the caller will do this.
    fn set_reserved_balance(asset_id: &T::AssetId, who: &T::AccountId, balance: T::Balance) {
        <ReservedBalance<T>>::insert(asset_id, who, &balance);
    }

    /// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is expected that
    /// the caller will do this.
    fn set_free_balance(asset_id: &T::AssetId, who: &T::AccountId, balance: T::Balance) {
        <FreeBalance<T>>::insert(asset_id, who, &balance);
    }

    fn set_lock(
        asset_id: &T::AssetId,
        who: &T::AccountId,
        amount: T::Balance,
        reasons: WithdrawReasons,
    ) -> u128 {
        let lock_id = Self::take_lock_id();
        let new_lock = BalanceLock {
            id: lock_id,
            asset_id: asset_id.clone(),
            amount,
            reasons,
        };
        let identifier = Self::generic_asset_lock_identifier(asset_id);
        <Locks<T>>::append_or_insert(&identifier, &who, vec![new_lock]);
        lock_id
    }

    #[allow(dead_code)]
    fn extend_lock(
        _id: LockIdentifier,
        _who: &T::AccountId,
        _amount: T::Balance,
        _reasons: WithdrawReasons,
    ) {
        // let mut new_lock = Some(BalanceLock {
        //     id,
        //     amount,
        //     reasons,
        // });
        // let mut locks = <Module<T>>::locks(who)
        //     .into_iter()
        //     .filter_map(|l| {
        //         if l.id == id {
        //             new_lock.take().map(|nl| BalanceLock {
        //                 id: l.id,
        //                 amount: l.amount.max(nl.amount),
        //                 reasons: l.reasons | nl.reasons,
        //             })
        //         } else {
        //             Some(l)
        //         }
        //     })
        //     .collect::<Vec<_>>();
        // if let Some(lock) = new_lock {
        //     locks.push(lock)
        // }
        // <Locks<T>>::insert(who, locks);
    }

    fn remove_lock(identifier: LockIdentifier, who: &T::AccountId, lock_id: u128) {
        let mut first = true;
        let locks = <Locks<T>>::get(identifier, who);
        let new_locks = locks
            .iter()
            .filter(|v| {
                if first && v.id == lock_id {
                    first = false;
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();

        if locks.len() - new_locks.len() == 1 {
            <Locks<T>>::insert(identifier, who, new_locks);
        }
    }

    pub fn lock_id_exists(identifier: LockIdentifier, who: &T::AccountId, lock_id: u128) -> bool {
        let locks = <Locks<T>>::get(identifier, who);
        for l in locks {
            if l.id == lock_id {
                return true;
            }
        }
        false
    }

    pub fn find_lock_id_by_amount(
        identifier: LockIdentifier,
        who: &T::AccountId,
        amount: T::Balance,
    ) -> Option<u128> {
        let locks = <Locks<T>>::get(identifier, who);
        for l in locks {
            if l.amount == amount {
                return Some(l.id);
            }
        }
        None
    }

    pub fn locked_balance(
        asset_id: &T::AssetId,
        who: &T::AccountId,
        lock_id: u128,
    ) -> Option<T::Balance> {
        let locks = <Locks<T>>::get(Self::generic_asset_lock_identifier(asset_id), who);
        for l in locks {
            if l.id == lock_id {
                return Some(l.amount);
            }
        }
        None
    }

    pub fn asset_id_exists(asset_id: T::AssetId) -> bool {
        <Symbols<T>>::contains_key(asset_id)
    }
}
