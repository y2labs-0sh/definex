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
    DepositLoanTest::on_finalize(SystemTest::block_number());
}

#[test]
fn staking_redeem_works() {
    let root: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Root");
    let eve: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Eve");
    let dave: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Dave");

    ExtBuilder::default().build().execute_with(|| {

        // mint 5000 unit usdt for dave;
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(5000)
                .ok()
                .unwrap(),
        ));

        // dave will save 1000 unit usdt；
        assert_ok!(DepositLoanTest::create_staking(
            dave.clone(),
            USDT.clone(),
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(1000)
                .ok()
                .unwrap()
        ));

        // check status
        assert_eq!(GenericAssetTest::free_balance(&USDT, &dave), 4000);
        assert_eq!(GenericAssetTest::free_balance(&USDT, &DepositLoanTest::collection_account_id()), 1000);
        assert_eq!(DepositLoanTest::market_dtoken(), 1000);
        assert_eq!(DepositLoanTest::total_dtoken(), 1000);
        assert_eq!(DepositLoanTest::user_dtoken(dave.clone()), 1000);

        // dave deposit 500 unit usdt
        assert_ok!(DepositLoanTest::make_redeem(&dave, &USDT, &DepositLoanTest::collection_account_id(), 500));

        // check status
        assert_eq!(GenericAssetTest::free_balance(&USDT, &dave), 4500);
        assert_eq!(GenericAssetTest::free_balance(&USDT, &DepositLoanTest::collection_account_id()), 500);
        assert_eq!(DepositLoanTest::market_dtoken(), 500);
        assert_eq!(DepositLoanTest::total_dtoken(), 500);
        assert_eq!(DepositLoanTest::user_dtoken(dave.clone()), 500);

        // mint 5000 unit usdt to eve
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &eve,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(5000)
                .ok()
                .unwrap(),
        ));
        // eve deposit 400 unit usdt
        assert_ok!(DepositLoanTest::create_staking(
            eve.clone(),
            USDT.clone(),
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(400)
                .ok()
                .unwrap()
        ));

        // check status
        assert_eq!(GenericAssetTest::free_balance(&USDT, &eve), 4600);
        assert_eq!(GenericAssetTest::free_balance(&USDT, &DepositLoanTest::collection_account_id()), 900);
        assert_eq!(DepositLoanTest::market_dtoken(), 900);
        assert_eq!(DepositLoanTest::total_dtoken(), 900);
        assert_eq!(DepositLoanTest::user_dtoken(eve.clone()), 400);

        assert_ok!(DepositLoanTest::make_redeem(&eve, &USDT, &DepositLoanTest::collection_account_id(), 400));
    });
}

#[test]
fn apply_draw_addcollateral_repay_works() {
    let root: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Root");
    let eve: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Eve");
    let dave: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Dave");

    ExtBuilder::default().build().execute_with(|| {
        // dave mint 50000 unit usdt
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(50000)
                .ok()
                .unwrap(),
        ));

        // dave deposit 24000 unit USDT
        assert_ok!(DepositLoanTest::create_staking(
            dave.clone(),
            USDT.clone(),
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(24000)
                .ok()
                .unwrap()
        ));

        // Eve mint 200 unit BTC
        assert_ok!(GenericAssetTest::mint_free(
            &BTC,
            &root,
            &eve,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(200)
                .ok()
                .unwrap(),
        ));

        // eve collateral 20 unit btc, borrow 300 unit usdt
        assert_ok!(DepositLoanTest::apply_for_loan(
            eve.clone(),
            20,
            300,
        ));

        // check status
        // saving 2400 borrow 300 usdt, collection_account will left 2100 unit usdt
        assert_eq!(GenericAssetTest::free_balance(&USDT, &DepositLoanTest::collection_account_id()), 24000 - 300);
        // eve have 300 unit usdt；180 unit btc
        assert_eq!(GenericAssetTest::free_balance(&BTC, &eve), 180);
        assert_eq!(GenericAssetTest::free_balance(&USDT, &eve), 300);
        // pawn_sho have 20 unit btc
        assert_eq!(GenericAssetTest::free_balance(&BTC, &DepositLoanTest::pawn_shop()), 20);

        assert_eq!(DepositLoanTest::total_loan(), 300);
        
        // current Utilization rate 300.0/24300 = 0.012345679012345678
        // current borrow interest rate：0.05123456790123457
        assert_eq!(DepositLoanTest::current_loan_interest_rate(), 5123456);
        assert_eq!(DepositLoanTest::current_saving_interest_rate(), 64043); // 5123456 * 300 / 24000

        assert_ok!(DepositLoanTest::apply_for_loan(
            eve.clone(),
            20,
            22000,
        ));
        assert_eq!(GenericAssetTest::free_balance(&USDT,  &DepositLoanTest::collection_account_id()), 24000 - 300 - 22000);
        assert_eq!(DepositLoanTest::total_loan(), 300 + 22000);

        // current Utilization rate：`48164146` = (22000 + 300) / (24000 + 22000 + 300)
        // loan interest rate：0.2*48164146+0.01*10**8 = 10632829
        assert_eq!(DepositLoanTest::current_loan_interest_rate(), 10632829);


        assert_ok!(DepositLoanTest::draw_from_loan(
            eve.clone(),
            0,
            10
        ));

        let eve_loan = DepositLoanTest::get_loan_by_id(0);
        assert_ok!(DepositLoanTest::add_loan_collateral(
            &eve_loan,
            eve.clone(),
            10,
        ));

    });
}
