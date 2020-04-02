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

// This module is meant for Web3 grant. In this module, definex implemented a DeFi model which follows a 'maker-taker'.

#![cfg(test)]
#![allow(dead_code)]

use crate::*;
use support::{
    assert_noop, assert_ok,
    traits::{OnFinalize, OnInitialize},
};

#[allow(unused_imports)]
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
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
    P2PTest::on_finalize(SystemTest::block_number());
}

#[test]
fn fetch_prices_works() {
    ExtBuilder::default().build().execute_with(|| {
        let prices = P2PTest::fetch_trading_pair_prices(USDT, BTC);
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
        let options = crate::P2PBorrowOptions {
            amount: <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(
                100_00000000,
            )
            .ok()
            .unwrap(),
            terms: 10,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };
        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(
            eve,
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(100000000)
                .ok()
                .unwrap(),
            trading_pair,
            options,
        ));

        let borrow = P2PTest::borrows(borrow_id);

        dbg!(borrow);
    });
}

#[test]
fn ltv_meet_safty_works() {
    ExtBuilder::default().build().execute_with(|| {
        let prices = P2PTest::fetch_trading_pair_prices(USDT, BTC);
        let borrow_amount =
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(10000_00000000)
                .ok()
                .unwrap();
        let collateral_amount =
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(1_00000000)
                .ok()
                .unwrap();

        assert_eq!(
            P2PTest::ltv_meet_safty(&prices.unwrap(), borrow_amount, collateral_amount),
            false
        );
    });
}

#[test]
fn expected_interest_works() {
    ExtBuilder::default().build().execute_with(|| {
        let borrow_amount =
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(10000_00000000)
                .ok()
                .unwrap();
        let interest = P2PTest::calculate_expected_interest(20000, 10, borrow_amount);
        dbg!(interest);
        assert_eq!(
            interest,
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(20_00000000)
                .ok()
                .unwrap()
        );
    });
}

#[test]
fn multi_borrows_error_works() {
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
        let options = crate::P2PBorrowOptions {
            amount: <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(
                100_00000000,
            )
            .ok()
            .unwrap(),
            terms: 10,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };

        assert_ok!(P2PTest::create_borrow(
            eve.clone(),
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(100000000)
                .ok()
                .unwrap(),
            trading_pair.clone(),
            options.clone(),
        ));

        assert_noop!(
            P2PTest::create_borrow(
                eve,
                <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(100000000)
                    .ok()
                    .unwrap(),
                trading_pair,
                options,
            ),
            Error::<Test>::MultipleAliveBorrows
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
        let options = crate::P2PBorrowOptions {
            amount: <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(
                4000_00000000,
            )
            .ok()
            .unwrap(),
            terms: 10,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };

        assert_noop!(
            P2PTest::create_borrow(
                eve,
                <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(1_00000000)
                    .ok()
                    .unwrap(),
                trading_pair,
                options,
            ),
            Error::<Test>::InitialCollateralRateFail
        );
    });
}

#[test]
fn lend_works() {
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
        let options = crate::P2PBorrowOptions {
            amount: <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(
                100_00000000,
            )
            .ok()
            .unwrap(),
            terms: 10,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };
        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(
            eve,
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(100000000)
                .ok()
                .unwrap(),
            trading_pair,
            options,
        ));

        let loan_id = P2PTest::next_loan_id();
        assert_ok!(P2PTest::create_loan(dave, borrow_id));

        let loan = P2PTest::loans(loan_id);
        assert_eq!(loan.borrow_id, borrow_id);
        assert_eq!(loan.due, 864001u64);

        dbg!(loan);
    });
}

#[test]
fn repay_works() {
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
        let options = crate::P2PBorrowOptions {
            amount: <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(
                100_00000000,
            )
            .ok()
            .unwrap(),
            terms: 10,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };
        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(
            eve,
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(100000000)
                .ok()
                .unwrap(),
            trading_pair,
            options,
        ));

        let loan_id = P2PTest::next_loan_id();
        assert_ok!(P2PTest::create_loan(dave, borrow_id));
        let loan = P2PTest::loans(loan_id);
        assert_eq!(loan.borrow_id, borrow_id);
        assert_noop!(
            P2PTest::repay_loan(eve, borrow_id),
            Error::<Test>::NotEnoughBalance
        );
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &eve,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(2_00000000)
                .ok()
                .unwrap(),
        ));
        assert_ok!(P2PTest::repay_loan(eve, borrow_id));
        assert_eq!(P2PTest::alive_borrow_ids().contains(&borrow_id), false);
    });
}

#[test]
fn liquidate_works() {
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
        let options = crate::P2PBorrowOptions {
            amount: <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(
                100_00000000,
            )
            .ok()
            .unwrap(),
            terms: 1,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };
        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(
            eve,
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(100000000)
                .ok()
                .unwrap(),
            trading_pair,
            options,
        ));
        let loan_id = P2PTest::next_loan_id();
        assert_ok!(P2PTest::create_loan(dave, borrow_id));
        assert_noop!(
            P2PTest::repay_loan(eve, borrow_id),
            Error::<Test>::NotEnoughBalance
        );

        next_n_block(86403u32.into());

        let loan = P2PTest::loans(loan_id);

        assert_eq!(loan.status, P2PLoanHealth::Overdue);
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &eve,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(2_00000000)
                .ok()
                .unwrap(),
        ));

        assert_ok!(P2PTest::liquidate_loan(dave, loan_id));

        let loan = P2PTest::loans(loan_id);
        assert_eq!(loan.status, P2PLoanHealth::Liquidated);
    });
}
