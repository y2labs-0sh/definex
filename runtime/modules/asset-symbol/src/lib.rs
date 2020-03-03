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

type AssetId<T> = <<T as Trait>::Asset as pallet_generic_asset::Trait>::AssetId;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Asset: pallet_generic_asset::Trait;
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        NoNamePermission,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as AssetSymbol {
        pub Symbol get(symbol) : map hasher(blake2_256) AssetId<T> => Vec<u8>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        pub fn set_name(origin, asset_id: AssetId<T>, symbol: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let update_permission = pallet_generic_asset::PermissionType::Update;
            // let can_update = T::Asset::check_permission(&asset_id, &who, &update_permission);
            let can_update = pallet_generic_asset::Module::<T::Asset>::check_permission(&asset_id, &who, &update_permission);
            debug::info!("{}", can_update);
            Ok(())
        }
    }
}

decl_event! {
    pub enum Event<T> where
        AssetId = AssetId<T>
    {
        SymbolCreated(AssetId),
    }
}

impl<T: Trait> Module<T> {
    pub fn is_symbol_occupied(s: Vec<u8>) -> bool {
        false
    }
}
