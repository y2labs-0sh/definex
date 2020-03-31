#![cfg(test)]
#![allow(dead_code)]

use crate::*;
use support::{assert_noop, assert_ok};

#[allow(unused_imports)]
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, OnFinalize, OnInitialize},
    Perbill,
};

use crate::mock::{constants::*, *};

#[test]
fn unittest_works() {
    ExtBuilder::default().build().execute_with(|| {});
    dbg!("hello world");
}

fn next_n_block(n: <Test as system::Trait>::BlockNumber) {
    SystemTest::set_block_number(SystemTest::block_number() + n);
    DepositLoanTest::on_finalize(SystemTest::block_number());
}

// #[test]
// fn deposit_works() {
//     let root: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Root");
//     let eve: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Eve");
//     let dave: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Dave");

//     ExtBuilder::default().build().execute_with(|| {
//         assert_ok!(GenericAssetTest::mint_free(
//             &USDT,
//             &root,
//             &dave,
//             &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(1000_00000000)
//                 .ok()
//                 .unwrap(),
//         ));

//         assert_ok!(DepositLoanTest::staking(DAVE, USDT.clone(), 1000,));
//     });
// }
