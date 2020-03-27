#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use sp_runtime::traits::{MaybeDisplay, MaybeFromStr};
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait GenericAssetApi<AssetId> where
        AssetId: Codec + MaybeDisplay + MaybeFromStr,
    {
        fn get_symbols_list() -> Option<Vec<(AssetId, Vec<u8>)>>;
    }
}
