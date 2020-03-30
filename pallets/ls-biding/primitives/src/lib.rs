#![cfg_attr(not(feature = "std"), no_std)]

#[allow(unused_imports)]
use codec::{Decode, Encode, Error as codecErr, HasCompact, Input, Output};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use sp_runtime::traits::{
    AtLeast32Bit, Bounded, CheckedAdd, CheckedMul, CheckedSub, MaybeDisplay,
    MaybeSerializeDeserialize, Member, One, Saturating, SignedExtension, Zero,
};
use sp_std::prelude::*;
#[allow(unused_imports)]
use sp_std::{
    self,
    convert::{TryFrom, TryInto},
    fmt::Debug,
    marker::PhantomData,
    result,
};
#[allow(unused_imports)]
use support::{
    debug, decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult, Parameter},
    ensure,
    traits::{
        Contains, Currency, Get, Imbalance, LockIdentifier, LockableCurrency, ReservableCurrency,
        WithdrawReason, WithdrawReasons,
    },
    weights::SimpleDispatchInfo,
    IterableStorageMap,
};

pub type LoanId = u128;
pub type BorrowId = u128;

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum LoanHealth {
    Well,
    ToBeLiquidated,
    Overdue,
    Liquidated,
    Dead,
    Completed,
}
impl Default for LoanHealth {
    fn default() -> Self {
        Self::Well
    }
}

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum LiquidationType {
    JustCollateral,
    SellCollateral,
}
impl Default for LiquidationType {
    fn default() -> Self {
        LiquidationType::JustCollateral
    }
}

#[derive(Debug, Encode, Decode, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Loan<AssetId, Balance, BlockNumber, AccountId> {
    pub id: LoanId,
    pub borrow_id: BorrowId,
    pub borrower_id: AccountId,
    pub loaner_id: AccountId,
    pub due: BlockNumber,
    pub collateral_asset_id: AssetId,
    pub collateral_balance: Balance,
    pub loan_balance: Balance,
    pub loan_asset_id: AssetId,
    pub status: LoanHealth,
    pub interest_rate: u64,
    pub liquidation_type: LiquidationType,
}

#[derive(Debug, Encode, Decode, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Borrow<AssetId, Balance, BlockNumber, AccountId> {
    pub id: BorrowId,
    pub lock_id: u128,
    pub who: AccountId,
    pub status: BorrowStatus,
    pub borrow_asset_id: AssetId,
    pub collateral_asset_id: AssetId,
    pub borrow_balance: Balance,
    pub collateral_balance: Balance,
    pub terms: u64, // days of our lives
    pub interest_rate: u64,
    pub dead_after: Option<BlockNumber>,
    pub loan_id: Option<LoanId>,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum BorrowStatus {
    Alive,
    Taken,
    Completed,
    Dead,
    Liquidated,
}
impl Default for BorrowStatus {
    fn default() -> Self {
        Self::Alive
    }
}

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TradingPair<A> {
    pub collateral: A,
    pub borrow: A,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TradingPairPrices {
    pub borrow_asset_price: u64,
    pub collateral_asset_price: u64,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct BorrowOptions<B, N> {
    pub amount: B,
    pub terms: u64,
    pub interest_rate: u64,
    pub warranty: Option<N>,
}
