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
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        NoNamePermission,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as AssetSymbol {
        pub Symbol get(symbol) : map hasher(blake2_256) T::AssetId => Vec<u8>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        pub fn set_name(origin, asset_id: T::AssetId, symbol: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let update_permission = pallet_generic_asset::PermissionType::Update;
            let can_update = pallet_generic_asset::Module::<T>::check_permission(&asset_id, &who, &update_permission);
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
    }
}

decl_event! {
    pub enum Event<T> where
        AssetId = <T as pallet_generic_asset::Trait>::AssetId
    {
        SymbolCreated(AssetId),
        SymbolUpdated(AssetId),
    }
}

impl<T: Trait> Module<T> {
    pub fn is_symbol_occupied(s: Vec<u8>) -> bool {
        false
    }
}
