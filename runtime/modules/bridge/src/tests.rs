#![cfg(test)]
#![allow(dead_code)]

use super::*;
use support::{assert_noop, assert_ok};

#[allow(unused_imports)]
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, OnFinalize, OnInitialize},
    Perbill,
};

use crate::mock::{
    constants::*, new_test_ext, BalancesTest, BridgeTest, Call, ExtBuilder, Origin, SystemTest,
    TestEvent, TestRuntime,
};

#[test]
fn unittest_works() {
    ExtBuilder::default().build().execute_with(|| {});
    dbg!("hello world");
}

#[test]
fn pause_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(BridgeTest::pause(system::RawOrigin::Root.into()));
        assert_eq!(BridgeTest::paused(), true);
    });
}

#[test]
fn resume_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(BridgeTest::pause(system::RawOrigin::Root.into()));
        assert_eq!(BridgeTest::paused(), true);
        assert_ok!(BridgeTest::resume(system::RawOrigin::Root.into()));
        assert_eq!(BridgeTest::paused(), false);
    });
}

#[test]
fn deposit_auth_works() {
    ExtBuilder::default().build().execute_with(|| {
        let tx_hash = TxHash::default();
        let who = DAVE;
        let amount = 1_00000000;
        let orig = Origin::signed(ALICE);
        assert_noop!(
            BridgeTest::deposit(orig, who, amount, tx_hash),
            "no deposit auth"
        );
    });
}

#[test]
fn deposit_works() {
    ExtBuilder::default().build().execute_with(|| {
        let tx_hash = TxHash::default();
        let who = DAVE;
        let amount = 1_00000000;
        let orig = Origin::signed(ROOT);
        assert_ok!(BridgeTest::deposit(orig, who, amount, tx_hash));
        assert_eq!(
            <assets::Module<TestRuntime>>::free_balance(&SBTC_ASSET_ID, &DAVE),
            1_00000000
        );
    });
}

#[test]
fn deposit_repeated_tx_hash_works() {
    ExtBuilder::default().build().execute_with(|| {
        let tx_hash = TxHash::default();
        let who = DAVE;
        let amount = 1_00000000;
        let orig = Origin::signed(ROOT);
        assert_ok!(BridgeTest::deposit(orig.clone(), who, amount, tx_hash));
        let amount = 2_00000000;
        assert_noop!(
            BridgeTest::deposit(orig, who, amount, tx_hash),
            "repeated transaction"
        );
    });
}

#[test]
fn deposit_over_threshold_works() {
    ExtBuilder::default().build().execute_with(|| {
        let tx_hash = TxHash::default();
        let who = DAVE;
        let amount = 30_00000000;
        let orig = Origin::signed(ROOT);
        assert_ok!(BridgeTest::deposit(orig, who, amount, tx_hash));
        let pending_list = BridgeTest::pending_deposit_list(who);
        assert_eq!(pending_list.len(), 1);
        assert_eq!(pending_list[0].tx_hash.unwrap(), tx_hash);
    });
}

#[test]
fn mark_black_works() {
    ExtBuilder::default().build().execute_with(|| {
        let tx_hash = TxHash::default();
        let who = DAVE;
        let amount = 30_00000000;
        let orig = Origin::signed(ROOT);
        assert_ok!(BridgeTest::deposit(orig.clone(), who, amount, tx_hash));
        assert_ok!(BridgeTest::mark_black(orig, who));
        let l = BridgeTest::list(who);
        assert_eq!(l, BlackOrWhite::Black);
        let pending = BridgeTest::pending_deposit_list(who);
        assert_eq!(pending.len(), 0);
        assert_eq!(
            <assets::Module<TestRuntime>>::free_balance(&SBTC_ASSET_ID, &who),
            0
        );
    });
}

#[test]
fn mark_white_works() {
    ExtBuilder::default().build().execute_with(|| {
        let tx_hash = TxHash::default();
        let who = DAVE;
        let amount = 30_00000000;
        let orig = Origin::signed(ROOT);
        assert_ok!(BridgeTest::deposit(orig.clone(), who, amount, tx_hash));
        assert_ok!(BridgeTest::mark_white(orig, who));
        let l = BridgeTest::list(who);
        assert_eq!(l, BlackOrWhite::White);
        let pending = BridgeTest::pending_deposit_list(who);
        assert_eq!(pending.len(), 0);
        assert_eq!(
            <assets::Module<TestRuntime>>::free_balance(&SBTC_ASSET_ID, &who),
            30_00000000
        );
    });
}

#[test]
fn black_blocks_works() {
    ExtBuilder::default().build().execute_with(|| {
        let orig = Origin::signed(ROOT);
        let who = DAVE;
        let amount = 30_00000000;
        let tx_hash = TxHash::default();
        assert_ok!(BridgeTest::mark_black(orig.clone(), who));
        assert_ok!(BridgeTest::deposit(orig.clone(), who, amount, tx_hash));
        let pending_list = BridgeTest::pending_deposit_list(who);
        assert_eq!(pending_list.len(), 1);
        assert_eq!(pending_list[0].tx_hash.unwrap(), tx_hash);
    });
}

#[test]
fn white_passes_works() {
    ExtBuilder::default().build().execute_with(|| {
        let orig = Origin::signed(ROOT);
        let who = DAVE;
        let amount = 30_00000000;
        let tx_hash = TxHash::default();
        assert_ok!(BridgeTest::mark_white(orig.clone(), who));
        assert_ok!(BridgeTest::deposit(orig.clone(), who, amount, tx_hash));
        let pending_list = BridgeTest::pending_deposit_list(who);
        assert_eq!(pending_list.len(), 0);
        assert_eq!(
            <assets::Module<TestRuntime>>::free_balance(&SBTC_ASSET_ID, &who),
            30_00000000
        );
    });
}

#[test]
fn withdraw_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(<assets::Module<TestRuntime>>::mint(
            Origin::ROOT,
            SBTC_ASSET_ID,
            CHRIS,
            100000000
        ));
        assert_ok!(BridgeTest::withdraw(Origin::signed(CHRIS), 100000000));
        assert_eq!(
            <assets::Module<TestRuntime>>::free_balance(&SBTC_ASSET_ID, &CHRIS),
            0
        );
        let withdraws = BridgeTest::pending_withdraws(CHRIS);
        assert_eq!(withdraws.len(), 1);
        assert_eq!(withdraws[0], 100000000);
    });
}

#[test]
fn refund_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(<assets::Module<TestRuntime>>::mint(
            Origin::ROOT,
            SBTC_ASSET_ID,
            CHRIS,
            100000000
        ));
        assert_ok!(BridgeTest::withdraw(Origin::signed(CHRIS), 100000000));
        assert_ok!(BridgeTest::refund(Origin::signed(ROOT), CHRIS, 100000000));
        let withdraws = BridgeTest::pending_withdraws(CHRIS);
        assert_eq!(withdraws.len(), 0);
        assert_eq!(
            <assets::Module<TestRuntime>>::free_balance(&SBTC_ASSET_ID, &CHRIS),
            100000000
        );
    });
}

#[test]
fn withdraw_finish_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(<assets::Module<TestRuntime>>::mint(
            Origin::ROOT,
            SBTC_ASSET_ID,
            CHRIS,
            100000000
        ));
        assert_ok!(BridgeTest::withdraw(Origin::signed(CHRIS), 100000000));
        assert_ok!(BridgeTest::withdraw_finish(
            Origin::signed(ROOT),
            CHRIS,
            100000000
        ));
        let withdraws = BridgeTest::pending_withdraws(CHRIS);
        assert_eq!(withdraws.len(), 0);
        assert_eq!(
            <assets::Module<TestRuntime>>::free_balance(&SBTC_ASSET_ID, &CHRIS),
            0
        );
    });
}

#[test]
fn two_withdraw_of_same_amount_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(<assets::Module<TestRuntime>>::mint(
            Origin::ROOT,
            SBTC_ASSET_ID,
            CHRIS,
            200000000
        ));
        assert_ok!(BridgeTest::withdraw(Origin::signed(CHRIS), 100000000));
        assert_ok!(BridgeTest::withdraw(Origin::signed(CHRIS), 100000000));
        assert_eq!(
            <assets::Module<TestRuntime>>::free_balance(&SBTC_ASSET_ID, &CHRIS),
            0
        );
        let withdraws = BridgeTest::pending_withdraws(CHRIS);
        assert_eq!(withdraws.len(), 2);

        assert_ok!(BridgeTest::withdraw_finish(
            Origin::signed(ROOT),
            CHRIS,
            100000000
        ));
        let withdraws = BridgeTest::pending_withdraws(CHRIS);
        assert_eq!(withdraws.len(), 1);
        assert_eq!(
            <assets::Module<TestRuntime>>::free_balance(&SBTC_ASSET_ID, &CHRIS),
            0
        );
    });
}

#[test]
fn two_withdraw_of_same_amount_refund_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(<assets::Module<TestRuntime>>::mint(
            Origin::ROOT,
            SBTC_ASSET_ID,
            CHRIS,
            200000000
        ));
        assert_ok!(BridgeTest::withdraw(Origin::signed(CHRIS), 100000000));
        assert_ok!(BridgeTest::withdraw(Origin::signed(CHRIS), 100000000));
        assert_eq!(
            <assets::Module<TestRuntime>>::free_balance(&SBTC_ASSET_ID, &CHRIS),
            0
        );
        let withdraws = BridgeTest::pending_withdraws(CHRIS);
        assert_eq!(withdraws.len(), 2);

        assert_ok!(BridgeTest::refund(Origin::signed(ROOT), CHRIS, 100000000));
        let withdraws = BridgeTest::pending_withdraws(CHRIS);
        assert_eq!(withdraws.len(), 1);
        assert_eq!(
            <assets::Module<TestRuntime>>::free_balance(&SBTC_ASSET_ID, &CHRIS),
            100000000
        );
    });
}
