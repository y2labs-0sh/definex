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

#![cfg(test)]

use frame_support::{
    impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types, weights::Weight,
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

use super::*;

impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
}
impl_outer_dispatch! {
    pub enum Call for Test where origin: Origin {
        system::System,
        sudo::Sudo,
    }
}

// For testing the pallet, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of pallets we want to use.
#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Call = ();
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<u64>;
    type Header = Header;
    type Event = TestEvent;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type ModuleToIndex = ();
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
}
impl sudo::Trait for Test {
    type Event = TestEvent;
    type Call = Call;
}
impl Trait for Test {
    type Balance = u64;
    type AssetId = u32;
    type Event = TestEvent;
}

mod generic_asset {
    pub use crate::Event;
}

use frame_system as system;
impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        sudo<T>,
        generic_asset<T>,
    }
}

pub type GenericAsset = Module<Test>;
pub type Sudo = sudo::Module<Test>;
pub type System = frame_system::Module<Test>;

pub const root: u64 = 999;
pub const next_asset_id: u32 = 2;

pub struct ExtBuilder {
    next_asset_id: u32,
    symbols: Vec<(u32, Vec<u8>)>,
}

// Returns default values for genesis config
impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            next_asset_id,
            symbols: vec![],
        }
    }
}

impl ExtBuilder {
    // Sets free balance to genesis config
    pub fn symbols(mut self, symbols: Vec<(u32, Vec<u8>)>) -> Self {
        self.symbols = symbols;
        self
    }

    pub fn next_asset_id(mut self, asset_id: u32) -> Self {
        self.next_asset_id = asset_id;
        self
    }

    // builds genesis config
    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        sudo::GenesisConfig::<Test> { key: root }
            .assimilate_storage(&mut t)
            .unwrap();

        GenesisConfig::<Test> {
            next_asset_id: self.next_asset_id,
            symbols: self.symbols,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        t.into()
    }
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap()
        .into()
}
