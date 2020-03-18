#![cfg_attr(not(feature = "std"), no_std)]

// mod mock;
// mod tests;

#[allow(unused_imports)]
use sp_runtime::{
    traits::{
        AtLeast32Bit, Bounded, CheckedAdd, CheckedSub, Dispatchable, MaybeSerializeDeserialize,
        Member, One, Saturating, Zero,
    },
    RuntimeDebug,
};
#[allow(unused_imports)]
use sp_std::{fmt::Debug, result, vec::Vec};
#[allow(unused_imports)]
use support::{
    debug, decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::{Currency, Imbalance, ReservableCurrency},
    weights::SimpleDispatchInfo,
    Parameter,
};
#[allow(unused_imports)]
use system::{ensure_none, ensure_root, ensure_signed};

pub trait Trait: system::Trait + pallet_generic_asset::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Balance: Parameter
        + Member
        + AtLeast32Bit
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug;
    type AssetId: Parameter + Member + AtLeast32Bit + Default + Copy;
}

/// Asset creation options.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
pub struct AssetOptions<Balance: HasCompact, AccountId> {
    /// Initial issuance of this asset. All deposit to the creator of the asset.
    #[codec(compact)]
    pub initial_issuance: Balance,
    /// Which accounts are allowed to possess this asset.
    pub permissions: Permissions<AccountId>,
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
pub struct Permissions<AccountId> {
    /// Who have permission to update asset permission
    pub update: Owner<AccountId>,
    /// Who have permission to mint new asset
    pub mint: Owner<AccountId>,
    /// Who have permission to burn asset
    pub burn: Owner<AccountId>,

    pub reserve: Owner<AccountId>,
}

/// Asset permission types
pub enum PermissionType {
    /// Permission to burn asset permission
    Burn,
    /// Permission to mint new asset
    Mint,
    /// Permission to update asset
    Update,
    /// Permission to reserve asset
    Reserve,
}

impl<AccountId> Default for Permissions<AccountId> {
    fn default() -> Self {
        Permissions {
            update: Owner::None,
            mint: Owner::None,
            burn: Owner::None,
            reserve: Owner::None,
        }
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        NoNamePermission,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as AssetSymbol {
        pub Symbol get(symbol) : map hasher(blake2_256) T::AssetId => Vec<u8>;
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

        /// Next available ID for user-created asset.
        pub NextAssetId get(fn next_asset_id) config(): T::AssetId;

        /// Permission options for a given asset.
        pub Permissions get(fn get_permission):
        map hasher(blake2_256) T::AssetId => PermissionVersions<T::AccountId>;

        /// Any liquidity locks on some account balances.
        pub Locks get(fn locks):
        map hasher(blake2_256) T::AccountId => Vec<BalanceLock<T::Balance>>;

        pub ReserveDelegate get(reserve_delegate) : double_map hasher(blake2_256) T::AccountId, hasher(blake2_256) T::AccountId => bool;
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
        });
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        pub fn set_name(origin, asset_id: T::AssetId, symbol: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let update_permission = PermissionType::Update;
            let can_update = Self::check_permission(&asset_id, &who, &update_permission);
            debug::info!("{}", can_update);
            ensure!(can_update, Error::<T>::NoNamePermission);

            if <Symbol<T>>::contains_key(&asset_id) {
                <Symbol<T>>::mutate(&asset_id, |v| {
                    *v = symbol;
                });
                Self::deposit_event(RawEvent::SymbolUpdated(asset_id));
            } else {
                <Symbol<T>>::insert(&asset_id, symbol);
                Self::deposit_event(RawEvent::SymbolCreated(asset_id));
            }

            Ok(())
        }

        /// Create a new kind of asset.
        fn create(origin, name: Vec<u8>, options: AssetOptions<T::Balance, T::AccountId>) -> DispatchResult {
            let origin = ensure_signed(origin)?;
            Self::create_asset(None, Some(origin), name, options)
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
            new_permission: Permissions<T::AccountId>
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;

            if Self::check_permission(&asset_id, &origin, &PermissionType::Update) {
                <Permissions<T>>::insert(asset_id, &new_permissions);
                Self::deposit_event(RawEvent::PermissionUpdated(asset_id, new_permissions));
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

        pub fn authorize_reserve(origin, delegator: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            <ReserveDelegate<T>>::insert(&who, &delegator, true);
            Self::deposit_event(RawEvent::ReserveAuthorized(who, delegator));
            Ok(())
        }

        pub fn revoke_reserve(origin, delegator: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            <ReserveDelegate<T>>::remove(&who, &delegator);
            Ok(())
        }
    }
}

decl_event! {
    pub enum Event<T> where
		    <T as frame_system::Trait>::AccountId,
		<T as Trait>::Balance,
		<T as Trait>::AssetId,
		AssetOptions = AssetOptions<<T as Trait>::Balance, <T as frame_system::Trait>::AccountId>
    {
        SymbolCreated(AssetId),
        SymbolUpdated(AssetId),
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
    }
}

impl<T: Trait> Module<T> {
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
        name: Vec<u8>,
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

        <TotalIssuance<T>>::insert(asset_id, &options.initial_issuance);
        <FreeBalance<T>>::insert(&asset_id, &account_id, &options.initial_issuance);
        <Permissions<T>>::insert(&asset_id, options.permissions.clone());
        Symbol
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
        let new_balance = Self::free_balance(asset_id, from)
            .checked_sub(&amount)
            .ok_or(Error::<T>::InsufficientBalance)?;
        Self::ensure_can_withdraw(
            asset_id,
            from,
            amount,
            WithdrawReason::Transfer.into(),
            new_balance,
        )?;

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
        from: &T::AccountId,
        asset_id: &T::AssetId,
        who: &T::AccountId,
        amount: T::Balance,
    ) -> DispatchResult {
        ensure!(<ReserveDelegate<T>>::contains_key(who, from) && <ReserveDelegate<T>>::get(who, from), Error::<T>::NoReserveAuth);

        // Do we need to consider that this is an atomic transaction?
        let original_reserve_balance = Self::reserved_balance(asset_id, who);
        let original_free_balance = Self::free_balance(asset_id, who);
        if original_free_balance < amount {
            Err(Error::<T>::InsufficientBalance)?
        }
        let new_reserve_balance = original_reserve_balance + amount;
        Self::set_reserved_balance(asset_id, who, new_reserve_balance);
        let new_free_balance = original_free_balance - amount;
        Self::set_free_balance(asset_id, who, new_free_balance);
        Ok(())
    }

    /// TODO:: design a mechanism to make sure only who reserved this amount can unreserve it
    pub fn unreserve(from: &T::AccountId, asset_id: &T::AssetId, who: &T::AccountId, amount: T::Balance) -> DispatchResult {
        ensure!(<ReserveDelegate<T>>::contains_key(who, from) && <ReserveDelegate<T>>::get(who, from), Error::<T>::NoReserveAuth);

        let b = Self::reserved_balance(asset_id, who);
        // let actual = sp_std::cmp::min(b, amount);
        ensure!(amount <= b, Error::<T>::NotEnoughReserved);
        let original_free_balance = Self::free_balance(asset_id, who);
        let new_free_balance = original_free_balance + actual;
        Self::set_free_balance(asset_id, who, new_free_balance);
        Self::set_reserved_balance(asset_id, who, b - actual);

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
        let free_balance = Self::free_balance(asset_id, who);
        let free_slash = sp_std::cmp::min(free_balance, amount);
        let new_free_balance = free_balance - free_slash;
        Self::set_free_balance(asset_id, who, new_free_balance);
        if free_slash < amount {
            Self::slash_reserved(asset_id, who, amount - free_slash)
        } else {
            None
        }
    }

    /// Deducts up to `amount` from reserved balance of `who`. This function cannot fail.
    ///
    /// As much funds up to `amount` will be deducted as possible. If the reserve balance of `who`
    /// is less than `amount`, then a non-zero second item will be returned.
    /// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is expected that
    /// the caller will do this.
    pub fn slash_reserved(
        asset_id: &T::AssetId,
        who: &T::AccountId,
        amount: T::Balance,
    ) -> Option<T::Balance> {
        let original_reserve_balance = Self::reserved_balance(asset_id, who);
        let slash = sp_std::cmp::min(original_reserve_balance, amount);
        let new_reserve_balance = original_reserve_balance - slash;
        Self::set_reserved_balance(asset_id, who, new_reserve_balance);
        if amount == slash {
            None
        } else {
            Some(amount - slash)
        }
    }

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
        let b = Self::reserved_balance(asset_id, who);
        let slash = sp_std::cmp::min(b, amount);

        match status {
            BalanceStatus::Free => {
                let original_free_balance = Self::free_balance(asset_id, beneficiary);
                let new_free_balance = original_free_balance + slash;
                Self::set_free_balance(asset_id, beneficiary, new_free_balance);
            }
            BalanceStatus::Reserved => {
                let original_reserved_balance = Self::reserved_balance(asset_id, beneficiary);
                let new_reserved_balance = original_reserved_balance + slash;
                Self::set_reserved_balance(asset_id, beneficiary, new_reserved_balance);
            }
        }

        let new_reserve_balance = b - slash;
        Self::set_reserved_balance(asset_id, who, new_reserve_balance);
        amount - slash
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
        asset_id: &T::AssetId,
        who: &T::AccountId,
        _amount: T::Balance,
        reasons: WithdrawReasons,
        new_balance: T::Balance,
    ) -> DispatchResult {
        if asset_id != &Self::staking_asset_id() {
            return Ok(());
        }

        let locks = Self::locks(who);
        if locks.is_empty() {
            return Ok(());
        }
        if Self::locks(who)
            .into_iter()
            .all(|l| new_balance >= l.amount || !l.reasons.intersects(reasons))
        {
            Ok(())
        } else {
            Err(Error::<T>::LiquidityRestrictions)?
        }
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
        id: LockIdentifier,
        who: &T::AccountId,
        amount: T::Balance,
        reasons: WithdrawReasons,
    ) {
        let mut new_lock = Some(BalanceLock {
            id,
            amount,
            reasons,
        });
        let mut locks = <Module<T>>::locks(who)
            .into_iter()
            .filter_map(|l| if l.id == id { new_lock.take() } else { Some(l) })
            .collect::<Vec<_>>();
        if let Some(lock) = new_lock {
            locks.push(lock)
        }
        <Locks<T>>::insert(who, locks);
    }

    fn extend_lock(
        id: LockIdentifier,
        who: &T::AccountId,
        amount: T::Balance,
        reasons: WithdrawReasons,
    ) {
        let mut new_lock = Some(BalanceLock {
            id,
            amount,
            reasons,
        });
        let mut locks = <Module<T>>::locks(who)
            .into_iter()
            .filter_map(|l| {
                if l.id == id {
                    new_lock.take().map(|nl| BalanceLock {
                        id: l.id,
                        amount: l.amount.max(nl.amount),
                        reasons: l.reasons | nl.reasons,
                    })
                } else {
                    Some(l)
                }
            })
            .collect::<Vec<_>>();
        if let Some(lock) = new_lock {
            locks.push(lock)
        }
        <Locks<T>>::insert(who, locks);
    }

    fn remove_lock(id: LockIdentifier, who: &T::AccountId) {
        let mut locks = <Module<T>>::locks(who);
        locks.retain(|l| l.id != id);
        <Locks<T>>::insert(who, locks);
    }
}
