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
            &1000_00000000u128,
        ));
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &1000_00000000u128,
        ));

        let trading_pair = crate::TradingPair {
            collateral: BTC,
            borrow: USDT,
        };
        let options = crate::P2PBorrowOptions {
            amount: 100_0000000u128,
            terms: 10,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };
        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(
            eve,
            100000000u128,
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
        let borrow_amount = 10000_00000000u128;
        let collateral_amount = 1_00000000u128;
        assert_eq!(
            P2PTest::ltv_meet_safty(&prices.unwrap(), borrow_amount, collateral_amount),
            false
        );
    });
}

#[test]
fn expected_interest_works() {
    ExtBuilder::default().build().execute_with(|| {
        let borrow_amount = 10000_00000000u128;
        let interest = P2PTest::calculate_expected_interest(20000, 10, borrow_amount);
        assert_eq!(interest, 20_00000000u128);
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
            &1000_00000000u128,
        ));
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &1000_00000000u128,
        ));

        let trading_pair = crate::TradingPair {
            collateral: BTC,
            borrow: USDT,
        };
        let options = crate::P2PBorrowOptions {
            amount: 100_00000000u128,
            terms: 10,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };

        assert_ok!(P2PTest::create_borrow(
            eve.clone(),
            100000000u128,
            trading_pair.clone(),
            options.clone(),
        ));

        assert_noop!(
            P2PTest::create_borrow(eve, 100000000u128, trading_pair, options),
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
            &1000_00000000u128,
        ));
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &1000_00000000u128,
        ));

        let trading_pair = crate::TradingPair {
            collateral: BTC,
            borrow: USDT,
        };
        let options = crate::P2PBorrowOptions {
            amount: 4000_00000000u128,
            terms: 10,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };

        assert_noop!(
            P2PTest::create_borrow(eve, 1_00000000u128, trading_pair, options,),
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
            &1000_00000000u128,
        ));
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &1000_00000000u128,
        ));

        let trading_pair = crate::TradingPair {
            collateral: BTC,
            borrow: USDT,
        };
        let options = crate::P2PBorrowOptions {
            amount: 100_00000000u128,
            terms: 10,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };
        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(
            eve,
            100000000u128,
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
            &1000_00000000u128,
        ));
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &1000_00000000u128,
        ));

        let trading_pair = crate::TradingPair {
            collateral: BTC,
            borrow: USDT,
        };
        let options = crate::P2PBorrowOptions {
            amount: 100_00000000u128,
            terms: 10,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };
        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(
            eve,
            100000000u128,
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
            &2_00000000u128,
        ));
        assert_ok!(P2PTest::repay_loan(eve, borrow_id));
        assert_eq!(P2PTest::alive_borrow_ids().contains(&borrow_id), false);
        assert_eq!(GenericAssetTest::free_balance(&USDT, &eve), 1_80000000u128);
        assert_eq!(
            GenericAssetTest::free_balance(&USDT, &dave),
            1000_20000000u128
        );
    });
}

#[test]
fn liquidate_works() {
    let root: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Root");
    let eve: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Eve");
    let dave: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Dave");
    let liquidator: <Test as system::Trait>::AccountId =
        get_from_seed::<sr25519::Public>("liquidator");
    let platform: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Platform");

    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(GenericAssetTest::mint_free(
            &BTC,
            &root,
            &eve,
            &1000_00000000u128,
        ));
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &10000_00000000u128,
        ));
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &liquidator,
            &10000_00000000u128,
        ));

        let trading_pair = crate::TradingPair {
            collateral: BTC,
            borrow: USDT,
        };
        let options = crate::P2PBorrowOptions {
            amount: 100_00000000u128,
            terms: 1,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };
        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(
            eve,
            100000000u128,
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
        assert_eq!(
            SystemTest::events()
                .into_iter()
                .map(|r| r.event)
                .filter_map(|e| {
                    if let MetaEvent::p2p(inner) = e {
                        match inner {
                            RawEvent::LoanOverdue(_) => Some(inner),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .last()
                .unwrap(),
            RawEvent::LoanOverdue(loan_id)
        );

        // add extra 2 period checks, you do the math ^+^
        next_n_block(5u32.into());
        next_n_block(5u32.into());
        assert_eq!(
            SystemTest::events()
                .into_iter()
                .map(|r| r.event)
                .filter_map(|ele| {
                    if let MetaEvent::p2p(inner) = ele {
                        match inner {
                            RawEvent::CheckingAliveLoans => Some(inner),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .count(),
            3
        );
        assert_eq!(
            SystemTest::events()
                .into_iter()
                .map(|r| r.event)
                .filter_map(|ele| {
                    if let MetaEvent::p2p(inner) = ele {
                        match inner {
                            RawEvent::CheckingAliveLoansDone => Some(inner),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .count(),
            3
        );
        // make sure no repeated overdue events fired
        assert_eq!(
            SystemTest::events()
                .into_iter()
                .map(|r| r.event)
                .filter_map(|ele| {
                    if let MetaEvent::p2p(inner) = ele {
                        match inner {
                            RawEvent::LoanOverdue(_) => Some(inner),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .count(),
            1
        );

        let loan = P2PTest::loans(loan_id);
        assert_eq!(loan.status, P2PLoanHealth::Overdue);
        assert_ok!(P2PTest::liquidate_loan(liquidator, loan_id));
        let loan = P2PTest::loans(loan_id);
        assert_eq!(loan.status, P2PLoanHealth::Liquidated);

        // now comes to money...
        assert_eq!(
            GenericAssetTest::free_balance(&BTC, &liquidator),
            100000000u128
        );
        assert_eq!(
            (P2PTest::liquidator_discount() as u128)
                * loan.collateral_balance
                * <new_oracle::Module<Test>>::current_price(b"BTC".to_vec()) as u128
                / 100u128
                / new_oracle::PRICE_SCALE as u128,
            9000_00000000u128
        );
        assert_eq!(
            GenericAssetTest::free_balance(&USDT, &liquidator),
            1000_00000000u128
        );
        assert_eq!(
            GenericAssetTest::free_balance(&USDT, &dave),
            10000_02000000u128
        );
        assert_eq!(GenericAssetTest::free_balance(&BTC, &eve), 999_00000000u128);
        assert_eq!(
            GenericAssetTest::free_balance(&USDT, &eve),
            4549_99000000u128,
        );
        assert_eq!(
            GenericAssetTest::free_balance(&USDT, &platform),
            4449_99000000u128,
        );
    });
}

#[test]
fn add_works() {
    let root: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Root");
    let eve: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Eve");
    let dave: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Dave");

    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(GenericAssetTest::mint_free(
            &BTC,
            &root,
            &eve,
            &1000_00000000u128
        ));
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &100000_00000000u128
        ));

        let trading_pair = crate::TradingPair {
            collateral: BTC,
            borrow: USDT,
        };
        let options = crate::P2PBorrowOptions {
            amount: 100_0000000u128,
            terms: 10,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };
        let one_btc = 100000000u128;
        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(eve, one_btc, trading_pair, options));
        assert_ok!(P2PTest::add_collateral(eve, borrow_id, one_btc));
        let borrow = P2PTest::borrows(borrow_id);
        assert_eq!(borrow.collateral_balance, one_btc * 2u128);
        assert_ok!(P2PTest::create_loan(dave, borrow_id));
        assert_ok!(P2PTest::add_collateral(eve, borrow_id, one_btc));
        let borrow = P2PTest::borrows(borrow_id);
        assert_eq!(borrow.collateral_balance, one_btc * 3u128);
        let loan = P2PTest::loans(borrow.loan_id.unwrap());
        assert_eq!(loan.collateral_balance, one_btc * 3u128);
    });
}

#[test]
fn cancel_works() {
    let root: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Root");
    let eve: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Eve");
    let dave: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Dave");

    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(GenericAssetTest::mint_free(
            &BTC,
            &root,
            &eve,
            &1000_00000000u128,
        ));
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &100000_00000000u128,
        ));

        let trading_pair = crate::TradingPair {
            collateral: BTC,
            borrow: USDT,
        };
        let options = crate::P2PBorrowOptions {
            amount: 100_00000000u128,
            terms: 10,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };
        let one_btc = 100000000u128;
        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(
            eve,
            one_btc,
            trading_pair.clone(),
            options.clone(),
        ));

        assert_ok!(P2PTest::remove_borrow(eve, borrow_id));

        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(
            eve,
            one_btc,
            trading_pair.clone(),
            options.clone()
        ));
        assert_ok!(P2PTest::add_collateral(eve, borrow_id, one_btc));
        assert_ok!(P2PTest::remove_borrow(eve, borrow_id));

        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(eve, one_btc, trading_pair, options));
        assert_ok!(P2PTest::add_collateral(eve, borrow_id, one_btc));
        assert_ok!(P2PTest::create_loan(dave, borrow_id));
        assert_noop!(
            P2PTest::remove_borrow(eve, borrow_id),
            Error::<Test>::CanNotCancelBorrow
        );
    });
}

#[test]
fn ensure_borrow_available_for_loan_works() {
    let root: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Root");
    let eve: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Eve");
    let dave: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Dave");

    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(GenericAssetTest::mint_free(
            &BTC,
            &root,
            &eve,
            &1000_00000000u128,
        ));
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &100000_00000000u128,
        ));
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &eve,
            &100000_00000000u128,
        ));

        let trading_pair = crate::TradingPair {
            collateral: BTC,
            borrow: USDT,
        };
        let options = crate::P2PBorrowOptions {
            amount: 100_00000000u128,
            terms: 1,
            interest_rate: 20000,
            warranty: Some(<Test as system::Trait>::BlockNumber::from(30u32)),
        };
        let one_btc = 100000000u128;
        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(
            eve,
            one_btc,
            trading_pair.clone(),
            options.clone(),
        ));
        assert!(P2PTest::ensure_borrow_available_for_loan(borrow_id).is_ok());
        next_n_block(10000000);
        assert!(P2PTest::ensure_borrow_available_for_loan(borrow_id).is_err());

        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(
            eve,
            one_btc,
            trading_pair.clone(),
            options.clone(),
        ));
        assert_ok!(P2PTest::create_loan(dave, borrow_id));
        assert!(P2PTest::ensure_borrow_available_for_loan(borrow_id).is_err());
        assert_ok!(P2PTest::repay_loan(eve, borrow_id));
        assert!(P2PTest::ensure_borrow_available_for_loan(borrow_id).is_err());

        let borrow_id = P2PTest::next_borrow_id();
        assert_ok!(P2PTest::create_borrow(
            eve,
            one_btc,
            trading_pair.clone(),
            options.clone(),
        ));
        assert_ok!(P2PTest::remove_borrow(eve, borrow_id));
        assert!(P2PTest::ensure_borrow_available_for_loan(borrow_id).is_err());
    });
}
