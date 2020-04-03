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

use super::*;
use crate::{GenesisConfig, Module, Trait};
use balances;
use sp_core::H256;
pub use sp_core::{sr25519, Pair, Public};
use std::cell::RefCell;
use support::{
    impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types, weights::Weight,
};

#[allow(unused_imports)]
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, ConvertInto, IdentityLookup, OnFinalize, OnInitialize},
    Perbill,
};

thread_local! {
    pub(crate) static EXISTENTIAL_DEPOSIT: RefCell<u128> = RefCell::new(0);
    static TRANSFER_FEE: RefCell<u128> = RefCell::new(0);
    static CREATION_FEE: RefCell<u128> = RefCell::new(0);
}

pub mod constants {
    use super::Test;
    pub const DECIMALS: u128 = 100000000; // satoshi
    pub const USDT: <Test as generic_asset::Trait>::AssetId = 0;
    pub const BTC: <Test as generic_asset::Trait>::AssetId = 1;

    //TODO:     pub const DAVE: <Test as frame_system::Trait>::AccountId = 5;
}

use self::constants::*;

impl_outer_origin! {
    pub enum Origin for Test {}
}

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
    type AccountId = sp_core::sr25519::Public; // <<Signature as Verify>::Signer as IdentifyAccount>::AccountId; // sp_core::sr25519::Public;
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
    type OnReapAccount = ();
}

// type Extrinsic = TestXt<new_oracle::Call<Test>, ()>;
type SubmitTransaction =
    system::offchain::TransactionSubmitter<new_oracle::crypto::Public, Test, Extrinsic>;

// impl system::offchain::CreateTransaction<Test, Extrinsic> for Test {
//     type Public = sp_core::sr25519::Public;
//     type Signature = sp_core::sr25519::Signature;

//     fn create_transaction<F: system::offchain::Signer<Self::Public, Self::Signature>>(
//         call: <Extrinsic as ExtrinsicsT>::Call,
//         _public: Self::Public,
//         _account: <Test as system::Trait>::AccountId,
//         nonce: <Test as system::Trait>::Index,
//     ) -> Option<(
//         <Extrinsic as ExtrinsicsT>::Call,
//         <Extrinsic as ExtrinsicsT>::SignaturePayload,
//     )> {
//         Some((call, (nonce, ())))
//     }
// }

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}
impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
}
impl sudo::Trait for Test {
    type Event = ();
    type Call = Call<Test>;
}
impl generic_asset::Trait for Test {
    type Event = ();
    type Balance = u128;
    type AssetId = u32;
}

type BlockNumber = u64;

parameter_types! {
    pub const AggregateInterval: BlockNumber = 5;
}
impl new_oracle::Trait for Test {
    type Event = ();
    type Call = new_oracle::Call<Test>;
    type SubmitUnsignedTransaction = SubmitTransaction;
    type SubmitSignedTransaction = SubmitTransaction;
    type AggregateInterval = AggregateInterval;
    type PriceInUSDT = u64;
}

impl Trait for Test {
    type Event = ();
}

type Balances = balances::Module<Test>;
type System = system::Module<Test>;
type Sudo = sudo::Module<Test>;

pub type DepositLoanTest = Module<Test>;
pub type SystemTest = system::Module<Test>;
pub type GenericAssetTest = generic_asset::Module<Test>;

pub struct ExtBuilder {}
impl Default for ExtBuilder {
    fn default() -> Self {
        Self {}
    }
}
impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        new_test_ext()
    }
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let root: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Root");
    let market_dtoken_account_id: <Test as system::Trait>::AccountId =
        get_from_seed::<sr25519::Public>("market_dtoken_account_id");
    let total_dtoken_account_id: <Test as system::Trait>::AccountId =
        get_from_seed::<sr25519::Public>("total_dtoken_account_id");
    let collection_account_id: <Test as system::Trait>::AccountId =
        get_from_seed::<sr25519::Public>("collection_account_id");
    let profit_pool: <Test as system::Trait>::AccountId =
        get_from_seed::<sr25519::Public>("profit_pool");
    let pawn_shop: <Test as system::Trait>::AccountId =
        get_from_seed::<sr25519::Public>("pawn_shop");
    let liquidation_account: <Test as system::Trait>::AccountId =
        get_from_seed::<sr25519::Public>("liquidation_account");

    sudo::GenesisConfig::<Test> { key: root }
        .assimilate_storage(&mut t)
        .unwrap();

    // new_oracle::GenesisConfig::<Test> {
    //     crypto_price_sources: vec![],
    //     current_price: vec![
    //         (b"BTC".to_vec(), 10000 * new_oracle::PRICE_SCALE),
    //         (b"DUSD".to_vec(), 1 * new_oracle::PRICE_SCALE),
    //     ],
    // }
    // .assimilate_storage(&mut t)
    // .unwrap();

    generic_asset::GenesisConfig::<Test> {
        next_asset_id: 2,
        assets: vec![],
        initial_balance: 0,
        endowed_accounts: vec![],
        symbols: vec![
            (0, "DUSD".as_bytes().to_vec()),
            (1, "BTC".as_bytes().to_vec()),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    GenesisConfig::<Test> {
        market_dtoken_account_id: market_dtoken_account_id,
        total_dtoken_account_id: total_dtoken_account_id,
        collection_account_id: collection_account_id,
        profit_pool: profit_pool,
        pawn_shop: pawn_shop,
        liquidation_account: liquidation_account,

        collection_asset_id: 0,
        dtoken_asset_id: 1,
        market_dtoken_asset_id: 2,
        total_dtoken_asset_id: 3,

        profit_asset_id: 0,

        collateral_asset_id: 4,
        loan_asset_id: 0,

        global_ltv_limit: 100000000,
        global_liquidation_threshold: 100000000,
        global_warning_threshold: 800000000,

        loan_interest_rate_current: 10,
        next_loan_id: 0,
        current_btc_price: 8899000,
        penalty_rate: 12,
        minimum_collateral: 1,
        liquidation_penalty: 12,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}
