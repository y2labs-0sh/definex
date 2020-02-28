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

use crate::mock::{
    constants::*, new_test_ext, Call, ExtBuilder, Origin, RioLoanTest, SystemTest, TestEvent,
    TestRuntime,
};

#[test]
fn unittest_works() {
    ExtBuilder::default().build().execute_with(|| {});
    dbg!("hello world");
}

#[test]
fn pause_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(RioLoanTest::pause(system::RawOrigin::Root.into()));
        assert_eq!(RioLoanTest::paused(), true);
    });
}

#[test]
fn resume_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(RioLoanTest::pause(system::RawOrigin::Root.into()));
        assert_eq!(RioLoanTest::paused(), true);
        assert_ok!(RioLoanTest::resume(system::RawOrigin::Root.into()));
        assert_eq!(RioLoanTest::paused(), false);
    });
}

#[test]
fn create_package_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(RioLoanTest::create_package(
            system::RawOrigin::Root.into(),
            10,
            100,
            1
        ));
        let active_package: LoanPackage<
            <TestRuntime as balances::Trait>::Balance,
            <TestRuntime as pallet_generic_asset::Trait>::AssetId,
        > = RioLoanTest::active_loan_packages(1);
        assert_eq!(active_package.collateral_asset_id, SBTC_ASSET_ID);
        assert_eq!(active_package.interest_rate_hourly, 100);
        assert_eq!(active_package.terms, 10);
    });
}

#[test]
fn disable_package_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(RioLoanTest::create_package(
            system::RawOrigin::Root.into(),
            10,
            100,
            1
        ));
        assert_ok!(RioLoanTest::disable_package(
            system::RawOrigin::Root.into(),
            1
        ));
        let void_package = RioLoanTest::active_loan_packages(1);
        assert_eq!(void_package.terms, 0);
    });
}

#[test]
fn apply_repay_works() {
    ExtBuilder::default().build().execute_with(|| {
        let package_id = RioLoanTest::next_loan_package_id();
        assert_ok!(RioLoanTest::create_package(
            system::RawOrigin::Root.into(),
            10,
            100,
            1
        ));
        assert_ok!(<rio_assets::Module<TestRuntime>>::mint(
            system::RawOrigin::Root.into(),
            SBTC_ASSET_ID,
            ALICE,
            1_00000000
        ));
        let loan_id = RioLoanTest::next_loan_id();
        assert_ok!(RioLoanTest::apply(
            Origin::signed(ALICE),
            1_00000000,
            4000_00000000,
            package_id
        ));
        let profit_pool = RioLoanTest::profit_pool();
        let profit = <rio_assets::Module<TestRuntime>>::free_balance(&RIO_ASSET_ID, &profit_pool);
        let user_got = <rio_assets::Module<TestRuntime>>::free_balance(&RIO_ASSET_ID, &ALICE);
        assert_eq!(profit, 96000000);
        assert_eq!(user_got + profit, 4000_00000000);

        assert_ok!(<rio_assets::Module<TestRuntime>>::mint(
            system::RawOrigin::Root.into(),
            RIO_ASSET_ID,
            ALICE,
            profit
        ));
        assert_ok!(RioLoanTest::repay(Origin::signed(ALICE), loan_id));
        assert_eq!(
            <rio_assets::Module<TestRuntime>>::free_balance(&SBTC_ASSET_ID, &ALICE),
            1_00000000
        );
        assert_eq!(
            <rio_assets::Module<TestRuntime>>::free_balance(&RIO_ASSET_ID, &ALICE),
            0
        );
    });
}

#[test]
fn draw_works() {
    ExtBuilder::default().build().execute_with(|| {
        let package_id = RioLoanTest::next_loan_package_id();
        assert_ok!(RioLoanTest::create_package(
            system::RawOrigin::Root.into(),
            10,
            100,
            1
        ));
        assert_ok!(<rio_assets::Module<TestRuntime>>::mint(
            system::RawOrigin::Root.into(),
            SBTC_ASSET_ID,
            ALICE,
            1_00000000
        ));
        let loan_id = RioLoanTest::next_loan_id();
        assert_ok!(RioLoanTest::apply(
            Origin::signed(ALICE),
            1_00000000,
            4000_00000000,
            package_id
        ));
        let profit_pool = RioLoanTest::profit_pool();
        let profit = <rio_assets::Module<TestRuntime>>::free_balance(&RIO_ASSET_ID, &profit_pool);
        let user_got = <rio_assets::Module<TestRuntime>>::free_balance(&RIO_ASSET_ID, &ALICE);
        assert_eq!(profit, 96000000);
        assert_eq!(user_got + profit, 4000_00000000);

        assert_ok!(RioLoanTest::draw(
            Origin::signed(ALICE),
            loan_id,
            1000_00000000
        ));

        let profit = <rio_assets::Module<TestRuntime>>::free_balance(&RIO_ASSET_ID, &profit_pool);
        let user_got = <rio_assets::Module<TestRuntime>>::free_balance(&RIO_ASSET_ID, &ALICE);
        assert_eq!(profit, 120000000);
        assert_eq!(user_got + profit, 5000_00000000);
        assert_ok!(<rio_assets::Module<TestRuntime>>::mint(
            system::RawOrigin::Root.into(),
            RIO_ASSET_ID,
            ALICE,
            profit
        ));
        assert_ok!(RioLoanTest::repay(Origin::signed(ALICE), loan_id));
        assert_eq!(
            <rio_assets::Module<TestRuntime>>::free_balance(&SBTC_ASSET_ID, &ALICE),
            1_00000000
        );
        assert_eq!(
            <rio_assets::Module<TestRuntime>>::free_balance(&RIO_ASSET_ID, &ALICE),
            0
        );
    });
}

#[test]
fn liquidate_loan_works() {
    ExtBuilder::default().build().execute_with(|| {
        let package_id = RioLoanTest::next_loan_package_id();
        assert_ok!(RioLoanTest::create_package(
            system::RawOrigin::Root.into(),
            10,
            100,
            1
        ));
        assert_ok!(<rio_assets::Module<TestRuntime>>::mint(
            system::RawOrigin::Root.into(),
            SBTC_ASSET_ID,
            ALICE,
            1_00000000
        ));
        let loan_id = RioLoanTest::next_loan_id();
        assert_ok!(RioLoanTest::apply(
            Origin::signed(ALICE),
            1_00000000,
            4000_00000000,
            package_id
        ));
        assert_eq!(RioLoanTest::loans_by_account(&ALICE).len(), 1);
        let loan_id = RioLoanTest::loans_by_account(&ALICE)[0];
        let loan = RioLoanTest::get_loan_by_id(&loan_id);
        assert_eq!(loan.loan_balance_total, 4000_00000000);
        assert_ok!(RioLoanTest::set_price(Origin::ROOT, 1));

        next_block();

        assert_eq!(RioLoanTest::liquidating_loans().len(), 1);
        let loan = RioLoanTest::get_loan_by_id(&loan_id);
        assert_eq!(RioLoanTest::liquidating_loans()[0], loan_id);

        assert_ok!(<rio_assets::Module<TestRuntime>>::mint(
            system::RawOrigin::Root.into(),
            RIO_ASSET_ID,
            BOB,
            5000_00000000
        ));

        let rio_alice = <rio_assets::Module<TestRuntime>>::free_balance(&RIO_ASSET_ID, &ALICE);
        assert_ok!(RioLoanTest::mark_loan_liquidated(&loan, BOB, 5000_00000000));
        assert_eq!(
            <rio_assets::Module<TestRuntime>>::free_balance(&RIO_ASSET_ID, &BOB),
            0
        );
        assert_eq!(
            <rio_assets::Module<TestRuntime>>::free_balance(&RIO_ASSET_ID, &ALICE),
            870_00000000 + rio_alice
        );
    });
}

/// TODO: try to figure out how to lower btc price to trigger liquidation
#[test]
fn add_collateral_works() {}

fn next_block() {
    SystemTest::set_block_number(SystemTest::block_number() + 1);
    RioLoanTest::on_initialize(SystemTest::block_number());
}
