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
    impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types,
    traits::{OnFinalize, OnInitialize},
    weights::Weight,
};
// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use crate::{GenesisConfig, Module, Trait};
use sp_core::H256;
pub use sp_core::{sr25519, Pair, Public};
use sp_std::convert::TryFrom;
use sp_std::str::FromStr;
use sp_version::RuntimeVersion;
use std::cell::RefCell;

use balances::Call as BalancesCall;

#[allow(unused_imports)]
pub use sp_runtime::{
    testing::{Header, TestXt},
    traits::{
        BlakeTwo256, ConvertInto, Extrinsic as ExtrinsicsT, IdentifyAccount, IdentityLookup, Verify,
    },
    MultiSignature, Perbill, RuntimeAppPublic,
};

pub type Signature = MultiSignature;
pub type AccountPublic = <Signature as Verify>::Signer;

pub type BlockNumber = u64;

thread_local! {
      pub(crate) static EXISTENTIAL_DEPOSIT: RefCell<u64> = RefCell::new(0);
      static TRANSFER_FEE: RefCell<u128> = RefCell::new(0);
      static CREATION_FEE: RefCell<u128> = RefCell::new(0);
}

pub mod constants {
    use super::Test;

    pub const DECIMALS: u128 = 100000000; // satoshi

    pub const USDT: <Test as generic_asset::Trait>::AssetId = 0;
    pub const BTC: <Test as generic_asset::Trait>::AssetId = 1;
}

impl_outer_origin! {
    pub enum Origin for Test where system = system {}
}
// I don't wanna know anything of this MACRO, just follow this pattern will do
impl_outer_dispatch! {
    pub enum Call for Test where origin: Origin {
        system::SystemTest,
        balances::Balances,
        p2p::P2PTest,
    }
}
impl_outer_event! {
    pub enum MetaEvent for Test {
        system<T>,
        balances<T>,
        sudo<T>,
        new_oracle<T>,
        generic_asset<T>,
        p2p<T>,
    }
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
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = sp_core::sr25519::Public; // <<Signature as Verify>::Signer as IdentifyAccount>::AccountId; // sp_core::sr25519::Public;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = MetaEvent;
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type ModuleToIndex = ();
    type AccountData = balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
}

type Extrinsic = TestXt<new_oracle::Call<Test>, ()>;
type SubmitTransaction =
    system::offchain::TransactionSubmitter<new_oracle::crypto::Public, Test, Extrinsic>;

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

pub struct ExistentialDeposit;
impl Get<u64> for ExistentialDeposit {
    fn get() -> u64 {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
    }
}
impl balances::Trait for Test {
    type Balance = u64;
    type Event = MetaEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = SystemTest;
}

parameter_types! {
        pub const MinimumPeriod: u64 = 5;
}
impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
}
impl sudo::Trait for Test {
    type Event = MetaEvent;
    type Call = Call;
}

impl generic_asset::Trait for Test {
    type Event = MetaEvent;
    type Balance = u128;
    type AssetId = u32;
}

parameter_types! {
    pub const AggregateInterval: BlockNumber = 5;
}
impl new_oracle::Trait for Test {
    type Event = MetaEvent;
    type Call = new_oracle::Call<Test>;
    type SubmitUnsignedTransaction = SubmitTransaction;
    type SubmitSignedTransaction = SubmitTransaction;
    type AggregateInterval = AggregateInterval;
    type PriceInUSDT = u64;
}

mod p2p {
    pub use super::super::*;
}
parameter_types! {
    pub const DaysInBlockNumber: BlockNumber = 86400u32.into();
}
impl Trait for Test {
    type Event = MetaEvent;
    type Days = DaysInBlockNumber;
    type Call = Call;
}

pub type P2PTest = Module<Test>;
pub type SystemTest = system::Module<Test>;
pub type GenericAssetTest = generic_asset::Module<Test>;
pub type Balances = balances::Module<Test>;

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

/// Helper function to generate an account ID from seed
// pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> <Test as system::Trait>::AccountId
// where
//     AccountPublic: From<<TPublic::Pair as Pair>::Public>,
// {
//     AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
// }

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let root: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Root");
    let money_pool: <Test as system::Trait>::AccountId =
        get_from_seed::<sr25519::Public>("Money_Pool");
    let platform: <Test as system::Trait>::AccountId = get_from_seed::<sr25519::Public>("Platform");

    sudo::GenesisConfig::<Test> { key: root }
        .assimilate_storage(&mut t)
        .unwrap();

    new_oracle::GenesisConfig::<Test> {
        crypto_price_sources: vec![],
        current_price: vec![
            (b"DUSD".to_vec(), 1 * new_oracle::PRICE_SCALE),
            (b"BTC".to_vec(), 10000 * new_oracle::PRICE_SCALE),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    generic_asset::GenesisConfig::<Test> {
        next_asset_id: 2,
        symbols: vec![
            (0, "DUSD".as_bytes().to_vec()),
            (1, "BTC".as_bytes().to_vec()),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    GenesisConfig::<Test> {
        money_pool,
        platform,
        trading_pairs: vec![crate::TradingPair {
            collateral: 1 as u32,
            borrow: 0 as u32,
        }],
        safe_ltv: 30000,
        liquidate_ltv: 15000,
        min_borrow_terms: 1,
        min_borrow_interest_rate: 10000,
        charge_penalty: true,
        liquidator_discount: 90,
        liquidation_penalty: 50,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}
