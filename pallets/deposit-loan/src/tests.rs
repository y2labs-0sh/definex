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
    // std::time::Duration::from_secs(3 * n);
    DepositLoanTest::on_initialize(SystemTest::block_number());
    // DepositLoanTest::on_finalize(SystemTest::block_number());
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
        assert_eq!(
            GenericAssetTest::free_balance(&USDT, &DepositLoanTest::collection_account_id()),
            1000
        );
        assert_eq!(DepositLoanTest::user_dtoken(dave.clone()), 1000);

        // dave deposit 500 unit usdt
        assert_ok!(DepositLoanTest::make_redeem(
            &dave,
            &USDT,
            &DepositLoanTest::collection_account_id(),
            500
        ));

        // check status
        assert_eq!(GenericAssetTest::free_balance(&USDT, &dave), 4500);
        assert_eq!(
            GenericAssetTest::free_balance(&USDT, &DepositLoanTest::collection_account_id()),
            500
        );
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
        assert_eq!(
            GenericAssetTest::free_balance(&USDT, &DepositLoanTest::collection_account_id()),
            900
        );
        assert_eq!(DepositLoanTest::user_dtoken(eve.clone()), 400);

        assert_ok!(DepositLoanTest::make_redeem(
            &eve,
            &USDT,
            &DepositLoanTest::collection_account_id(),
            400
        ));
    });
}

#[test]
fn apply_draw_addcollateral_repay_works() {
    let root: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Root");
    let eve: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Eve");
    let dave: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Dave");

    ExtBuilder::default().build().execute_with(|| {
        // dave mint 50_0000_0000 unit usdt
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(50_0000_0000)
                .ok()
                .unwrap(),
        ));

        // dave deposit 40_0000_0000 unit USDT
        assert_ok!(DepositLoanTest::create_staking(
            dave.clone(),
            USDT.clone(),
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(40_0000_0000)
                .ok()
                .unwrap()
        ));

        // Eve mint 20_0000_0000 unit BTC
        assert_ok!(GenericAssetTest::mint_free(
            &BTC,
            &root,
            &eve,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(20_0000_0000)
                .ok()
                .unwrap(),
        ));

        // eve collateral 10_0000_0000 unit btc, borrow 25_0000_0000 unit usdt
        assert_ok!(DepositLoanTest::apply_for_loan(
            eve.clone(),
            10_0000_0000,
            25_0000_0000,
        ));

        // check status
        // saving 40_0000_0000 borrow 25_0000_0000 usdt, collection_account will left 15_0000_0000 unit usdt
        assert_eq!(
            GenericAssetTest::free_balance(&USDT, &DepositLoanTest::collection_account_id()),
            40_0000_0000 - 25_0000_0000
        );
        // eve have 300 unit usdt；180 unit btc
        assert_eq!(GenericAssetTest::free_balance(&BTC, &eve), 10_0000_0000);
        assert_eq!(GenericAssetTest::free_balance(&USDT, &eve), 25_0000_0000);
        // pawn_shop have 20 unit btc
        assert_eq!(
            GenericAssetTest::free_balance(&BTC, &DepositLoanTest::pawn_shop()),
            10_0000_0000
        );

        assert_eq!(DepositLoanTest::total_loan(), 25_0000_0000);

        // current Utilization rate 25_0000_0000/(25_0000_0000 + 40_0000_0000) = 0.38461538461538464
        // current borrow interest rate：0.08846153846153847
        assert_eq!(DepositLoanTest::current_loan_interest_rate(), 8846153);
        assert_eq!(DepositLoanTest::current_saving_interest_rate(), 5528845); // 8846153 * 25_0000_0000 / 40_0000_0000

        let eve_loan = DepositLoanTest::get_loan_by_id(0);
        assert_eq!(eve_loan.collateral_balance_original, 10_0000_0000);
        assert_eq!(eve_loan.collateral_balance_available, 9_9975_0000); // TODO: 10_0000_0000 - 25_0000_0000 / 10000
        assert_eq!(eve_loan.loan_balance_total, 25_0000_0000);

        assert_ok!(DepositLoanTest::apply_for_loan(
            eve.clone(),
            5_0000_0000,
            10_0000_0000
        ));
        assert_eq!(
            GenericAssetTest::free_balance(&USDT, &DepositLoanTest::collection_account_id()),
            40_0000_0000 - 10_0000_0000 - 25_0000_0000
        );
        assert_eq!(DepositLoanTest::total_loan(), 10_0000_0000 + 25_0000_0000);

        let eve_loan = DepositLoanTest::get_loan_by_id(0);
        assert_eq!(GenericAssetTest::free_balance(&BTC, &eve), 5_0000_0000);

        assert_ok!(DepositLoanTest::add_loan_collateral(
            &eve_loan,
            eve.clone(),
            1_0000_0000,
        ));

        assert_eq!(GenericAssetTest::free_balance(&BTC, &eve), 4_0000_0000);
        assert_eq!(GenericAssetTest::free_balance(
            &BTC,
            &DepositLoanTest::pawn_shop()),
            10_0000_0000 + 5_0000_0000 + 1_0000_0000
        );

        let eve_loan = DepositLoanTest::get_loan_by_id(0);

        assert_eq!(
            eve_loan.collateral_balance_original,
            20_0000_0000 - 10_0000_0000 + 1_0000_0000
        );
        assert_eq!(
            eve_loan.collateral_balance_available,
            9_9975_0000 + 1_0000_0000
        );

        // draw from loan
        assert_ok!(DepositLoanTest::draw_from_loan(
            eve.clone(),
            0,  // loan_id
            1_0000_0000  // amount
        ));

        let eve_loan = DepositLoanTest::get_loan_by_id(0);
        assert_eq!(eve_loan.loan_balance_total, 25_0000_0000 + 1_0000_0000);
        assert_eq!(eve_loan.collateral_balance_available, 9_9975_0000 + 1_0000_0000 - 1_0000);
    });
}

#[test]
fn deliver_interest_works() {
    let root: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Root");
    let dave: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Dave");
    let eve: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Eve");
    let frank: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Frank");

    ExtBuilder::default().build().execute_with(|| {
        // mint for dave 200_0000_0000 unit usdt:
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &dave,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(200_0000_0000)
                .ok()
                .unwrap(),
        ));

        // mint for eve 200_0000_0000 unit usdt:
        assert_ok!(GenericAssetTest::mint_free(
            &USDT,
            &root,
            &eve,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(200_0000_0000)
                .ok()
                .unwrap(),
        ));

        // mint for frank 200_0000_0000 unit btc:
        assert_ok!(GenericAssetTest::mint_free(
            &BTC,
            &root,
            &frank,
            &<<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(200_0000_0000)
                .ok()
                .unwrap(),
        ));

        // dave deposit 100_0000_0000 unit USDT
        assert_ok!(DepositLoanTest::create_staking(
            dave.clone(),
            USDT.clone(),
            <<Test as generic_asset::Trait>::Balance as TryFrom<u64>>::try_from(100_0000_0000)
                .ok()
                .unwrap()
        ));

        assert_ok!(DepositLoanTest::apply_for_loan(
            frank.clone(),
            20_0000_0000,
            40_0000_0000,
        ));

        assert_eq!(DepositLoanTest::total_loan(), 40_0000_0000);
        assert_eq!(DepositLoanTest::user_dtoken(dave.clone()), 100_0000_0000);

        // current_total_loan = 40_0000_0000 ; current_total_saving = 100_0000_0000;
        // so current Utilization rate = 40_0000_0000 / (40_0000_0000 + 100_0000_0000) = 0.2857142857142857 ;
        // so current_total_interest_rate: 0.2857142857142857 * 0.1 + 0.05 = 0.07857142857142857
        assert_eq!(DepositLoanTest::current_loan_interest_rate(), 7857142);
        assert_eq!(DepositLoanTest::current_saving_interest_rate(), 3142856); // 7857142 * 40_0000_0000 / 100_0000_0000 = 3142856.8
        assert_eq!(DepositLoanTest::value_of_tokens(), 1_0000_0000);

        // after 1500s, interest will be: 1500 * 7857142 / 10^8 / 365 / 86400 * total_loan = 149.48900304414002

        // calculate interest generate by time
        // 1.0 * 10**8 * (1.0 + 14900.0/10000000000) = 100000149
        // TODO: add time duration in testcase

        // next_n_block(5u32.into());
        // assert_eq!(DepositLoanTest::value_of_tokens(), 100000149);
        // assert_eq!(GenericAssetTest::free_balance(&USDT, &DepositLoanTest::collection_account_id()), 6000014948);
    });
}
