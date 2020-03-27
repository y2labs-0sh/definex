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

#[allow(unused_imports)]
use codec::{Codec, Decode, Encode, Error as codecErr, HasCompact, Input, Output};
use sp_core::crypto::KeyTypeId;
#[allow(unused_imports)]
use sp_core::H256;
#[allow(unused_imports)]
use sp_runtime::{
    offchain::{http, storage::StorageValueRef, Duration},
    traits::{
        AtLeast32Bit, Bounded, CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member,
        Saturating, Zero,
    },
    transaction_validity::{
        InvalidTransaction, TransactionLongevity, TransactionValidity, ValidTransaction,
    },
};
use sp_std::{
    convert::{TryFrom, TryInto},
    fmt::Debug,
    prelude::*,
};
use support::{
    debug, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
    storage::IterableStorageMap, traits::Get, weights::SimpleDispatchInfo, Parameter, StorageValue,
};

#[cfg(not(feature = "std"))]
use num_traits::float::FloatCore;

#[allow(unused_imports)]
use system::{ensure_none, ensure_root, ensure_signed, offchain};

use simple_json::{self, json::JsonValue};

mod mock;
mod tests;

pub type StrBytes = Vec<u8>;
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"orcl");
pub const PRICE_SCALE: u64 = 10000;

pub mod crypto {
    use super::KEY_TYPE;
    use sp_runtime::app_crypto::{app_crypto, sr25519};
    app_crypto!(sr25519, KEY_TYPE);
}

pub trait Trait: timestamp::Trait + system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Call: From<Call<Self>>;

    type SubmitSignedTransaction: offchain::SubmitSignedTransaction<Self, <Self as Trait>::Call>;
    type SubmitUnsignedTransaction: offchain::SubmitUnsignedTransaction<Self, <Self as Trait>::Call>;

    type PriceInUSDT: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug;

    type AggregateInterval: Get<Self::BlockNumber>;
}

decl_storage! {
    trait Store for Module<T: Trait> as NewOracle {
        pub CryptoPriceSources get(crypto_price_sources) config() : map hasher(opaque_blake2_256) StrBytes => Vec<(StrBytes, StrBytes, Vec<StrBytes>)>;
        pub PriceCandidates get(price_candidates) : map hasher(twox_64_concat) StrBytes => Vec<T::PriceInUSDT>;
        pub CurrentPrice get(current_price) config() : map hasher(twox_64_concat) StrBytes => T::PriceInUSDT;
        pub NextAggregateAt get(next_aggregate_at) : T::BlockNumber;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        fn on_finalize(bn: T::BlockNumber) {
            Self::_on_finalize_(bn)
        }

        pub fn add_source(origin, token: StrBytes, source_name: StrBytes, source_url: StrBytes, json_path: Vec<StrBytes>) -> DispatchResult {
            ensure_root(origin)?;
            if CryptoPriceSources::contains_key(&token) {
                CryptoPriceSources::mutate(&token, |v| {
                    v.push((source_name, source_url, json_path));
                })
            } else{
                CryptoPriceSources::insert(&token, vec![(source_name, source_url, json_path)]);
            }

            Ok(())
        }

        pub fn stack_price_unsigned(origin, block_number: T::BlockNumber, token: StrBytes, price: T::PriceInUSDT) -> DispatchResult {
            ensure_none(origin)?;

            Self::stack_price(block_number, token, price)?;

            Ok(())
        }

        pub fn stack_price_signed(origin, block_number: T::BlockNumber, token: StrBytes, price: T::PriceInUSDT) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Self::stack_price(block_number, token, price);

            Ok(())
        }

        fn offchain_worker(block_number: T::BlockNumber) {
            use system::offchain::SubmitUnsignedTransaction;
            debug::native::info!("Hello World from offchain workers!");

            let rand_s = sp_io::offchain::random_seed();
            let r = u64::from_ne_bytes(sp_io::hashing::twox_64(&rand_s));
            debug::info!("{}", r);

            let token = b"BTC".to_vec();
            let sources = CryptoPriceSources::get(&token);
            let source = &sources[r as usize % sources.len()];

            match Self::fetch_json(&source.1) {
                Err(e) => {
                    debug::error!("Fail to fetch price: {}", e);
                }
                Ok(json_data) => {
                    let price_r = Self::parse_price(json_data, &source.2);
                    match price_r {
                        Ok(price) => {
                            let call = Call::stack_price_unsigned(block_number, token, price);
                            match T::SubmitUnsignedTransaction::submit_unsigned(call) {
                                Err(e) => {
                                    debug::error!("Fail to submit unsigned transaction for price: {:?}", e);
                                }
                                Ok(_) => {
                                    return ();
                                }
                            }
                        }
                        Err(e) => {
                            debug::error!("Fail to parse price: {}", e);
                        }
                    }
                }
            }
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn is_token_known(token: &StrBytes) -> bool {
        <CurrentPrice<T>>::contains_key(token)
    }

    fn stack_price(
        _block_number: T::BlockNumber,
        token: StrBytes,
        price: T::PriceInUSDT,
    ) -> Result<(), &'static str> {
        // check price

        <PriceCandidates<T>>::mutate(&token, |v| {
            v.push(price);
        });

        Ok(())
    }

    fn fetch_json(url: &StrBytes) -> Result<JsonValue, &'static str> {
        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
        let remote_url = core::str::from_utf8(&url).map_err(|_| "Invalid Remote URL")?;
        let request = http::Request::get(remote_url);
        let pending = request.deadline(deadline).send().map_err(|_| "")?;
        let response = pending
            .try_wait(deadline)
            .map_err(|_| "deadline")?
            .map_err(|_| "")?;

        if response.code != 200 {
            debug::warn!("Unexpected status code: {}", response.code);
            return Err("unexpected status code");
        }

        let body = response.body().collect::<Vec<u8>>();
        Ok(simple_json::parse_json(
            &core::str::from_utf8(&body).map_err(|_| "invalid UTF8 response")?,
        )
        .map_err(|_| "invalid JSON response")?)
    }

    fn parse_price(
        json_data: JsonValue,
        json_path: &[StrBytes],
    ) -> Result<T::PriceInUSDT, &'static str> {
        if json_path.len() == 0 {
            // if let Some(p_f64) = json_data.get_number_f64() {
            //     return Ok(((p_f64 * 10000.).round() as u64).try_into().ok.unwrap());
            // } else if let Some(price_u8) = json_data.get_bytes() {
            //     let val_f64: f64 = core::str::from_utf8(&price_u8)
            //         .map_err(|_| "fetch_price: val_f64 convert to string error")?
            //         .parse::<f64>()
            //         .map_err(|_| "fetch_price: val_u8 parsing to f64 error")?;
            //     return Ok((val_f64 * 10000.).round() as u64);
            // }
            return Ok(Self::parse_field(&json_data)?.try_into().ok().unwrap());
        } else {
            let mut data_cur = &json_data;

            for f in json_path {
                if let Some(obj_vec) = data_cur.get_object() {
                    let (_, v) = obj_vec
                        .iter()
                        .filter(|(k, _)| f.to_vec() == Self::vecchars_to_vecbytes(k))
                        .nth(0)
                        .ok_or("fetch_price: JSON does not conform to expectation")?;
                    data_cur = v;
                } else {
                    return Err("JSON does not confirm to expectation");
                }
            }

            return Ok(Self::parse_field(data_cur)?.try_into().ok().unwrap());
        }
    }

    /// parse_field can only parse number & &[u8]
    fn parse_field(json_data: &JsonValue) -> Result<u64, &'static str> {
        if let Some(p_f64) = json_data.get_number_f64() {
            return Ok(((p_f64 * PRICE_SCALE as f64).round() as u64)
                .try_into()
                .ok()
                .unwrap());
        } else if let Some(price_u8) = json_data.get_bytes() {
            let val_f64: f64 = core::str::from_utf8(&price_u8)
                .map_err(|_| "parse_field: val_f64 convert to string error")?
                .parse::<f64>()
                .map_err(|_| "parse_field: val_u8 parsing to f64 error")?;
            return Ok((val_f64 * PRICE_SCALE as f64).round() as u64);
        }
        Err("unknown data")
    }

    fn vecchars_to_vecbytes<I: IntoIterator<Item = char> + Clone>(it: &I) -> Vec<u8> {
        it.clone().into_iter().map(|c| c as u8).collect::<_>()
    }

    fn _on_finalize_(bn: T::BlockNumber) {
        if (bn % T::AggregateInterval::get()).is_zero() {
            for (k, _) in <PriceCandidates<T>>::iter() {
                let pc = <PriceCandidates<T>>::take(&k);
                let l = pc.len();
                let t: T::PriceInUSDT = pc.into_iter().fold(0.into(), |acc, x| acc + x);
                let mean = t / <T::PriceInUSDT as TryFrom<usize>>::try_from(l)
                    .ok()
                    .unwrap();
                if <CurrentPrice<T>>::contains_key(&k) {
                    <CurrentPrice<T>>::mutate(&k, |v| {
                        *v = mean;
                    });
                } else {
                    <CurrentPrice<T>>::insert(&k, mean);
                }
            }
            <NextAggregateAt<T>>::put(bn + T::AggregateInterval::get());
        }
    }

    // fn parse_price(
    //     json_data: json::Value,
    //     json_path: &[StrBytes],
    // ) -> Result<T::PriceInUSDT, &'static str> {
    //     let mut jval = &json_data;
    //     for f in json_path {
    //         let f_str = core::str::from_utf8(f).map_err(|_| "invalid json path")?;
    //         match json_data.get(f_str) {
    //             None => return Err("incompatible json URI"),
    //             Some(obj) => jval = &obj,
    //         }
    //     }

    //     if jval.is_string() {
    //         let price_str = jval.as_str().unwrap();
    //         let price_f = price_str
    //             .parse::<f64>()
    //             .map_err(|_| "fail to parse json into f64")?;
    //         return Ok(((price_f * 10000.).round() as u64).try_into().ok().unwrap());
    //     } else if jval.is_u64() {
    //         return Ok((jval.as_u64().unwrap() * 10000).try_into().ok().unwrap());
    //     } else if jval.is_f64() {
    //         return Ok(((jval.as_f64().unwrap() * 10000.).round() as u64)
    //             .try_into()
    //             .ok()
    //             .unwrap());
    //     } else {
    //         return Err("incompatible json URI");
    //     }
    // }
}

decl_event! {
    pub enum Event<T> where
        BlockNumber = <T as system::Trait>::BlockNumber
    {
        FetchedPrice(BlockNumber, StrBytes, u64),
    }
}

#[allow(deprecated)]
impl<T: Trait> support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    fn validate_unsigned(call: &Self::Call) -> TransactionValidity {
        match call {
            Call::stack_price_unsigned(block, token, price) => Ok(ValidTransaction {
                priority: 0,
                requires: vec![],
                provides: vec![(block, token, price).encode()],
                longevity: 3,
                propagate: true,
            }),
            // Call::record_agg_pp(block, sym, price) => Ok(ValidTransaction {
            //     priority: 0,
            //     requires: vec![],
            //     provides: vec![(block, sym, price).encode()],
            //     longevity: TransactionLongevity::max_value(),
            //     propagate: true,
            // }),
            _ => InvalidTransaction::Call.into(),
        }
    }
}
