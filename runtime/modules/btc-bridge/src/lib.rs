#![cfg_attr(not(feature = "std"), no_std)]

#[allow(unused_imports)]
use codec::{Decode, Encode, Error as codecErr, HasCompact, Input, Output};
use sp_core::H256;
use sp_std::marker::PhantomData;
use sp_std::prelude::*;
use support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
    weights::SimpleDispatchInfo,
};
use system::{ensure_root, ensure_signed};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use hkd32;

mod mock;
mod tests;

pub type TxHash = H256;

#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub enum BlackOrWhite {
    Black,
    White,
}
impl Default for BlackOrWhite {
    fn default() -> Self {
        Self::Black
    }
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub enum Auth {
    All,
    Deposit,
    Withdraw,
    Refund,
    Mark,
}
impl Default for Auth {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Deposit<AccountId, Balance> {
    /// the account who will receive "amount" of SBTC
    pub account_id: AccountId,
    /// tx_hash is the hash of the transaction that transfers BTC into TBD
    pub tx_hash: Option<TxHash>,
    /// SBTC 1:1 BTC
    pub amount: Balance,
}

pub trait Trait: system::Trait + assets::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as BTCBridge {
        /// the matrial used to derive receiving addresses for users
        pub KeyMaterial get(key_material) config() : [u8; 32];
        ///
        pub SubKeyMap get(sub_key_map) :
      double_map hasher(twox_128) T::AccountId, hasher(blake2_128) Vec<u8> => H256;
        /// the asset id for shadow BTC
        AssetId get(asset_id) config() : T::AssetId;
        /// module level switch
        Paused get(paused) : bool = false;
        /// KYC list
        List get(list) : linked_map hasher(blake2_256) T::AccountId => BlackOrWhite;
        /// deposit amount above this amount will trigger KYC
        Threshold get(threshold) config() : T::Balance;
        /// collection of accounts which are authorized to
        Admins get(admins) build(|config: &GenesisConfig<T>| {
            config.admins.clone()
        }) : linked_map hasher(blake2_256) T::AccountId => Auth;
        /// deposits grouped by account which are in pending for KYC
        PendingDepositList get(pending_deposit_list) : linked_map hasher(blake2_256) T::AccountId => Vec<Deposit<T::AccountId, T::Balance>>;
        /// keep a history of depoists in case of double spent
        DepositHistory get(deposit_history) : linked_map hasher(blake2_256) TxHash => Option<Deposit<T::AccountId, T::Balance>>;
        ///
        PendingWithdraws get(pending_withdraws) : linked_map hasher(blake2_256) T::AccountId => Vec<T::Balance>;
        ///
        PendingWithdrawVault get(pending_withdraw_vault) config() : T::AccountId;
    }

    add_extra_genesis {
        config(admins): Vec<(T::AccountId, Auth)>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        #[weight = SimpleDispatchInfo::MaxOperational]
        pub fn set_key_material(origin, material: [u8; 32]) -> DispatchResult {
            ensure_root(origin)?;
            KeyMaterial::put(material);
            Ok(())
        }

        pub fn get_subkey_from_path(origin, path_input: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(KeyMaterial::exists(), <Error<T>>::MissingKeyMaterial.as_str());
            ensure!(hkd32::DELIMITER.is_ascii(), <Error<T>>::UnsupporttedPathDelimiter.as_str());
            if <SubKeyMap<T>>::contains_key(&who, &path_input) {
                return Ok(());
            } else {
                let key_material = hkd32::KeyMaterial::new(KeyMaterial::get());
                let mut delimiter: [u8; 1] = [0x00];
                // ensure ascii, can't panic
                hkd32::DELIMITER.encode_utf8(&mut delimiter);
                let mut components = path_input.split(|x| -> bool { x == &delimiter[0] });
                let mut path_buf = hkd32::PathBuf::new();
                components.next();
                components.for_each(|x| {
                    path_buf.push(hkd32::Component::new(x).unwrap());
                });
                let derived_key = key_material.derive_subkey(path_buf);
                let derived_hash = H256::from_slice(derived_key.as_bytes());
                <SubKeyMap<T>>::insert(who, path_input, derived_hash);
                return Ok(());
                // match hkd32::PathBuf::from_bytes(&path_input) {
                //     Ok(path) => {
                //         let derived_key = key_material.derive_subkey(path);
                //         let derived_hash = H256::from_slice(derived_key.as_bytes());
                //         <SubKeyMap<T>>::insert(who, path_input, derived_hash);
                //         return Ok(());
                //     },
                //     Err(_) => {
                //         return Err(<Error<T>>::InvalidPathInput.into());
                //     }
                // }
            }
        }

        #[weight = SimpleDispatchInfo::MaxNormal]
        pub fn pause(origin) -> DispatchResult {
            ensure_root(origin)?;
            Paused::mutate(|v| *v = true);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::MaxNormal]
        pub fn resume(origin) -> DispatchResult {
            ensure_root(origin)?;
            Paused::mutate(|v| *v = false);
            Ok(())
        }

        /// TODO:: hash(tx_hash + account_id) as deposit identity
        /// TODO:: use offchain worker to do some verification on BTC
        #[weight = SimpleDispatchInfo::MaxOperational]
        pub fn deposit(origin, account_id: T::AccountId, amount: T::Balance, tx_hash: TxHash) -> DispatchResult {
            ensure!(!Self::is_tx_seen(&tx_hash), "repeated transaction");

            let who = ensure_signed(origin)?;
            ensure!(Self::has_auth(&who, Auth::Deposit), "no deposit auth");
            if amount >= Self::threshold() {
                let bow = Self::check_list(&account_id);
                if bow.is_none() || bow.unwrap() == BlackOrWhite::Black {
                    return Self::hold_deposit_with_event(&account_id, amount, tx_hash);
                }
            }

            Self::simple_deposit_with_event(&account_id, amount, tx_hash)?;
            Ok(())
        }

        #[weight = SimpleDispatchInfo::MaxOperational]
        pub fn refund(origin, who: T::AccountId, amount: T::Balance) -> DispatchResult {
            let author = ensure_signed(origin)?;
            let asset_id = Self::asset_id();
            ensure!(Self::has_auth(&author, Auth::Refund), "no refund auth");
            ensure!(Self::pending_withdraws(&who).contains(&amount), "pending withdraw not found");
            <assets::Module<T>>::make_transfer_with_event(&asset_id, &Self::pending_withdraw_vault(), &who, amount)?;
            Self::remove_from_pending_withdraws(who.clone(), amount);
            Self::deposit_event(RawEvent::Refund(who, amount));
            Ok(())
        }

        #[weight = SimpleDispatchInfo::MaxOperational]
        pub fn withdraw_finish(origin, who: T::AccountId, amount: T::Balance) -> DispatchResult {
            let author = ensure_signed(origin)?;
            let asset_id = Self::asset_id();
            ensure!(Self::has_auth(&author, Auth::Withdraw), "no withdraw auth");
            ensure!(Self::pending_withdraws(&who).contains(&amount), "pending withdraw not found");
            Self::remove_from_pending_withdraws(who.clone(), amount);
            Self::deposit_event(RawEvent::Withdraw(who, amount));
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(1000_000)]
        pub fn withdraw(origin, amount: T::Balance) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let asset_id = Self::asset_id();
            <assets::Module<T>>::make_transfer_with_event(&asset_id, &who, &Self::pending_withdraw_vault(), amount)?;
            if <PendingWithdraws<T>>::contains_key(&who) {
                <PendingWithdraws<T>>::mutate(&who, |v| {
                    v.push(amount);
                });
            } else {
                <PendingWithdraws<T>>::insert(&who, vec![amount]);
            }
            Self::deposit_event(RawEvent::PendingWithdraw(who, amount));
            Ok(())
        }


        #[weight = SimpleDispatchInfo::MaxOperational]
        pub fn mark_black(origin, account_id: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::has_auth(&who, Auth::Mark), "no mark auth");
            let pendings = <PendingDepositList<T>>::take(&account_id);
            if pendings.len() > 0 {
                pendings.iter().for_each(|v| {
                    <DepositHistory<T>>::remove(v.tx_hash.unwrap());
                });
            }
            Self::mark_with_event(account_id, BlackOrWhite::Black);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::MaxOperational]
        pub fn mark_white(origin, account_id: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::has_auth(&who, Auth::Mark), "no mark auth");
            let mut pendings = &<PendingDepositList<T>>::take(&account_id)[..];
            while pendings.len() > 0 {
                pendings[0].tx_hash.unwrap();
                Self::simple_deposit_with_event(&pendings[0].account_id,
                                                pendings[0].amount, pendings[0].tx_hash.unwrap()).or_else(|err| -> DispatchResult {
                        <PendingDepositList<T>>::insert(&account_id, pendings);
                        Err(err)
                    })?;
                pendings = &pendings[1..];
            }
            Self::mark_with_event(account_id, BlackOrWhite::White);
            Ok(())
        }
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        MissingKeyMaterial,
        InvalidPathInput,
        UnsupporttedPathDelimiter,
    }
}

decl_event!(
    #[rustfmt::skip]
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = <T as generic_asset::Trait>::Balance,
    {
        AccountMarked(AccountId, BlackOrWhite),
        Deposit(AccountId, Balance, TxHash),
        Pending(AccountId, Balance, TxHash),
        PendingWithdraw(AccountId, Balance),
        Refund(AccountId, Balance),
        Withdraw(AccountId, Balance),
    }
);

impl<T: Trait> Module<T> {
    fn is_tx_seen(tx_hash: &TxHash) -> bool {
        <DepositHistory<T>>::contains_key(tx_hash)
    }

    fn simple_deposit_with_event(
        account_id: &T::AccountId,
        amount: T::Balance,
        tx_hash: TxHash,
    ) -> DispatchResult {
        let dep: Deposit<T::AccountId, T::Balance> = Deposit {
            tx_hash: None,
            amount: amount,
            account_id: account_id.clone(),
        };
        <DepositHistory<T>>::insert(&tx_hash, dep);
        <assets::Module<T>>::mint(
            system::RawOrigin::Root.into(),
            Self::asset_id(),
            account_id.clone(),
            amount,
        )?;
        Self::deposit_event(RawEvent::Deposit(account_id.clone(), amount, tx_hash));
        Ok(())
    }

    fn hold_deposit_with_event(
        account_id: &T::AccountId,
        amount: T::Balance,
        tx_hash: TxHash,
    ) -> DispatchResult {
        let mut dep: Deposit<T::AccountId, T::Balance> = Deposit {
            tx_hash: None,
            amount: amount,
            account_id: account_id.clone(),
        };
        <DepositHistory<T>>::insert(&tx_hash, dep.clone());
        dep.tx_hash = Some(tx_hash);
        <PendingDepositList<T>>::mutate(&account_id, |v| {
            v.push(dep);
        });
        Self::deposit_event(RawEvent::Pending(account_id.clone(), amount, tx_hash));
        Ok(())
    }

    fn has_auth(account_id: &T::AccountId, auth: Auth) -> bool {
        if !<Admins<T>>::contains_key(account_id) {
            return false;
        } else {
            let account_auth = Self::admins(account_id);
            account_auth == Auth::All || account_auth == auth
        }
    }

    fn remove_from_pending_withdraws(who: T::AccountId, amount: T::Balance) {
        let pendings = <PendingWithdraws<T>>::take(&who);
        if pendings.len() > 1 {
            let mut x = Vec::with_capacity(pendings.len() - 1);
            let mut found = false;
            for amt in pendings {
                if found || amt != amount {
                    x.push(amt);
                } else {
                    found = true;
                }
            }
            <PendingWithdraws<T>>::insert(&who, x);
        }
    }

    pub fn mark(account_id: T::AccountId, bow: BlackOrWhite) {
        <List<T>>::insert(account_id, bow);
    }

    pub fn mark_with_event(account_id: T::AccountId, bow: BlackOrWhite) {
        Self::mark(account_id.clone(), bow);
        Self::deposit_event(RawEvent::AccountMarked(account_id, bow));
    }

    pub fn check_list(account_id: &T::AccountId) -> Option<BlackOrWhite> {
        if <List<T>>::contains_key(account_id) {
            Some(<List<T>>::get(account_id))
        } else {
            None
        }
    }
}
