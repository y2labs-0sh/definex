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
#![cfg(test)]
use crate::*;

use codec::Decode;
use sp_core::{
    offchain::{testing, OffchainExt, TransactionPoolExt},
    testing::KeyStore,
    traits::KeystoreExt,
    H256,
};
use sp_runtime::{
    testing::{Header, TestXt},
    traits::{BlakeTwo256, Extrinsic as ExtrinsicsT, IdentityLookup},
    Perbill, RuntimeAppPublic,
};
use support::{
    assert_ok, impl_outer_origin, parameter_types,
    weights::{GetDispatchInfo, Weight},
};

impl_outer_origin! {
    pub enum Origin for Test  where system = system {}
}

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl system::Trait for Test {
    type Origin = Origin;
    type Call = ();
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = sp_core::sr25519::Public;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type ModuleToIndex = ();
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 1;
}
impl timestamp::Trait for Test {
    type Moment = u64;
    type MinimumPeriod = MinimumPeriod;
    type OnTimestampSet = ();
}

type Extrinsic = TestXt<Call<Test>, ()>;
type SubmitTransaction = system::offchain::TransactionSubmitter<crypto::Public, Test, Extrinsic>;

impl system::offchain::CreateTransaction<Test, Extrinsic> for Test {
    type Public = sp_core::sr25519::Public;
    type Signature = sp_core::sr25519::Signature;

    fn create_transaction<F: system::offchain::Signer<Self::Public, Self::Signature>>(
        call: <Extrinsic as ExtrinsicsT>::Call,
        _public: Self::Public,
        _account: <Test as system::Trait>::AccountId,
        nonce: <Test as system::Trait>::Index,
    ) -> Option<(
        <Extrinsic as ExtrinsicsT>::Call,
        <Extrinsic as ExtrinsicsT>::SignaturePayload,
    )> {
        Some((call, (nonce, ())))
    }
}

parameter_types! {
    pub const AggregateInterval: u64 = 5;
}

impl Trait for Test {
    type Event = ();
    type Call = Call<Test>;
    type SubmitSignedTransaction = SubmitTransaction;
    type SubmitUnsignedTransaction = SubmitTransaction;
    type PriceInUSDT = u64;
    type AggregateInterval = AggregateInterval;
}

type NewOracle = Module<Test>;

#[test]
fn should_make_http_call_and_parse_result() {
    let (offchain, state) = testing::TestOffchainExt::new();
    let mut t = sp_io::TestExternalities::default();
    t.register_extension(OffchainExt::new(offchain));

    price_oracle_response(&mut state.write());

    t.execute_with(|| {
        let url = b"https://min-api.cryptocompare.com/data/price?fsym=BTC&tsyms=USD".to_vec();
        let price = NewOracle::fetch_json(&url).unwrap();
        assert_eq!(
            NewOracle::parse_price(price, &vec![b"USD".to_vec()]).ok(),
            Some(1552300)
        );
    });
}

// #[test]
// fn should_submit_signed_transaction_on_chain() {
//     const PHRASE: &str =
//         "news slush supreme milk chapter athlete soap sausage put clutch what kitten";

//     let (offchain, offchain_state) = testing::TestOffchainExt::new();
//     let (pool, pool_state) = testing::TestTransactionPoolExt::new();
//     let keystore = KeyStore::new();
//     keystore
//         .write()
//         .sr25519_generate_new(
//             crate::crypto::Public::ID,
//             Some(&format!("{}/hunter1", PHRASE)),
//         )
//         .unwrap();

//     let mut t = sp_io::TestExternalities::default();
//     t.register_extension(OffchainExt::new(offchain));
//     t.register_extension(TransactionPoolExt::new(pool));
//     t.register_extension(KeystoreExt(keystore));

//     price_oracle_response(&mut offchain_state.write());

//     t.execute_with(|| {
//         // when
//         // NewOracle::fetch_price_and_send_signed().unwrap();
//         // // then
//         // let tx = pool_state.write().transactions.pop().unwrap();
//         // assert!(pool_state.read().transactions.is_empty());
//         // let tx = Extrinsic::decode(&mut &*tx).unwrap();
//         // assert_eq!(tx.signature.unwrap().0, 0);
//         // assert_eq!(tx.call, Call::submit_price(15523));
//     });
// }

// #[test]
// fn should_submit_unsigned_transaction_on_chain() {
//     let (offchain, offchain_state) = testing::TestOffchainExt::new();
//     let (pool, pool_state) = testing::TestTransactionPoolExt::new();
//     let mut t = sp_io::TestExternalities::default();
//     t.register_extension(OffchainExt::new(offchain));
//     t.register_extension(TransactionPoolExt::new(pool));

//     price_oracle_response(&mut offchain_state.write());

//     t.execute_with(|| {
//         NewOracle::fetch_price_and_submit_unsigned(1);

//         let tx = pool_state.write().transactions.pop().unwrap();
//         assert!(pool_state.read().transactions.is_empty());
//         let tx = Extrinsic::decode(&mut &*tx).unwrap();
//         assert_eq!(tx.signature, None);
//         assert_eq!(
//             tx.call,
//             Call::stack_price_unsigned(1, b"BTC".to_vec(), 1552300)
//         );
//     });
// }

// #[test]
// fn weights_work() {
// // must have a default weight.
// let default_call = <Call<Test>>::submit_price(10);
// let info = default_call.get_dispatch_info();
// // aka. `let info = <Call<Test> as GetDispatchInfo>::get_dispatch_info(&default_call);`
// assert_eq!(info.weight, 10_000);
// }

fn price_oracle_response(state: &mut testing::OffchainState) {
    state.expect_request(
        0,
        testing::PendingRequest {
            method: "GET".into(),
            uri: "https://min-api.cryptocompare.com/data/price?fsym=BTC&tsyms=USD".into(),
            response: Some(br#"{"USD": 155.23}"#.to_vec()),
            sent: true,
            ..Default::default()
        },
    );
}

#[test]
fn parse_price_works() {
    let test_data = vec![
        ("{\"USD\":6536.92}", Some(65369200)),
        ("{\"USD\":65.92}", Some(659200)),
        ("{\"USD\":6536.924565}", Some(65369246)), // round up
        ("{\"USD\":6536}", Some(65360000)),
        ("{\"USD2\":6536}", None),
        ("{\"USD\":\"6432\"}", Some(64320000)),
    ];
    let json_parse_path = vec![b"USD".to_vec()];
    for (json, expected) in test_data {
        let json_value = simple_json::parse_json(json).unwrap();
        assert_eq!(
            expected,
            NewOracle::parse_price(json_value, &json_parse_path).ok()
        );
    }
}
