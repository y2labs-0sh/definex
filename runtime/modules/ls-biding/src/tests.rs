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

fn next_block() {
    SystemTest::set_block_number(SystemTest::block_number() + 1);
    LSBidingTest::on_finalize(SystemTest::block_number());
}

#[test]
fn fetch_prices_works() {
    ExtBuilder::default().build().execute_with(|| {
        let prices = LSBidingTest::fetch_trading_pair_prices(USDT, BTC);
        assert_eq!(prices.is_some(), true);
        let prices = prices.unwrap();
        assert_eq!(prices.borrow_asset_price, 10000u32.into());
        assert_eq!(prices.collateral_asset_price, 100000000u32.into());
    });
}

#[test]
fn borrow_works() {
    let root: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Root");
    let eve: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Eve");
    let dave: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Dave");

    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(GenericAssetTest::mint_free(
            &BTC,
            &root,
            &eve,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(1000_00000000)
                .ok()
                .unwrap(),
        ));
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(1000_00000000)
                .ok()
                .unwrap(),
        ));

        let trading_pair = crate::TradingPair {
            collateral: BTC,
            borrow: USDT,
        };
        let options = crate::BorrowOptions {
            amount: <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(
                100_00000000,
            )
            .ok()
            .unwrap(),
            terms: 10,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };
        let borrow_id = LSBidingTest::next_borrow_id();
        assert_ok!(LSBidingTest::create_borrow(
            eve,
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(100000000)
                .ok()
                .unwrap(),
            trading_pair,
            options,
        ));

        let borrow = LSBidingTest::borrows(borrow_id);

        dbg!(borrow);
    });
}

#[test]
fn ltv_meet_safty_works() {
    ExtBuilder::default().build().execute_with(|| {
        let trading_pair = crate::TradingPair {
            collateral: BTC,
            borrow: USDT,
        };
        let prices = LSBidingTest::fetch_trading_pair_prices(USDT, BTC);
        let borrow_amount =
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(10000_00000000)
                .ok()
                .unwrap();
        let collateral_amount =
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(1_00000000)
                .ok()
                .unwrap();

        assert_eq!(
            LSBidingTest::ltv_meet_safty(&prices.unwrap(), borrow_amount, collateral_amount),
            false
        );
    });
}

#[test]
fn invalid_borrow_works() {
    let root: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Root");
    let eve: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Eve");
    let dave: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Dave");

    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(GenericAssetTest::mint_free(
            &BTC,
            &root,
            &eve,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(1000_00000000)
                .ok()
                .unwrap(),
        ));
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(1000_00000000)
                .ok()
                .unwrap(),
        ));

        let trading_pair = crate::TradingPair {
            collateral: BTC,
            borrow: USDT,
        };
        let options = crate::BorrowOptions {
            amount: <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(
                3000_00000000,
            )
            .ok()
            .unwrap(),
            terms: 10,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };

        assert_noop!(
            LSBidingTest::create_borrow(
                eve,
                <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(100000000)
                    .ok()
                    .unwrap(),
                trading_pair,
                options,
            ),
            Error::<Test>::InitialCollateralRateFail
        );
    });
}
