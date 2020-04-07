#![cfg_attr(not(feature = "std"), no_std)]

#[allow(unused_imports)]
use codec::{Decode, Encode, Error as codecErr, HasCompact, Input, Output};
#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

#[cfg(feature = "std")]
fn serialize_as_string<S: Serializer, T: std::fmt::Display>(
    t: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&t.to_string())
}

#[cfg(feature = "std")]
fn deserialize_from_string<'de, D: Deserializer<'de>, T: std::str::FromStr>(
    deserializer: D,
) -> Result<T, D::Error> {
    let s = String::deserialize(deserializer)?;
    s.parse::<T>()
        .map_err(|_| serde::de::Error::custom("Parse from string failed"))
}

pub type P2PLoanId = u128;
pub type P2PBorrowId = u128;

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum P2PLoanHealth {
    Well,
    ToBeLiquidated,
    Overdue,
    Liquidated,
    Dead,
    Completed,
}
impl Default for P2PLoanHealth {
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
pub struct P2PLoan<AssetId, Balance, BlockNumber, AccountId> {
    #[cfg_attr(
        feature = "std",
        serde(bound(serialize = "Balance: std::fmt::Display"))
    )]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(
        feature = "std",
        serde(bound(deserialize = "Balance: std::str::FromStr"))
    )]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub id: P2PLoanId,

    #[cfg_attr(
        feature = "std",
        serde(bound(serialize = "Balance: std::fmt::Display"))
    )]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(
        feature = "std",
        serde(bound(deserialize = "Balance: std::str::FromStr"))
    )]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub borrow_id: P2PBorrowId,

    pub borrower_id: AccountId,
    pub loaner_id: AccountId,
    pub due: BlockNumber,
    pub collateral_asset_id: AssetId,

    #[cfg_attr(
        feature = "std",
        serde(bound(serialize = "Balance: std::fmt::Display"))
    )]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(
        feature = "std",
        serde(bound(deserialize = "Balance: std::str::FromStr"))
    )]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub collateral_balance: Balance,

    #[cfg_attr(
        feature = "std",
        serde(bound(serialize = "Balance: std::fmt::Display"))
    )]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(
        feature = "std",
        serde(bound(deserialize = "Balance: std::str::FromStr"))
    )]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub loan_balance: Balance,

    pub loan_asset_id: AssetId,
    pub status: P2PLoanHealth,
    pub interest_rate: u64,
    pub liquidation_type: LiquidationType,
}

#[derive(Debug, Encode, Decode, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct P2PBorrow<AssetId, Balance, BlockNumber, AccountId> {
    #[cfg_attr(
        feature = "std",
        serde(bound(serialize = "Balance: std::fmt::Display"))
    )]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(
        feature = "std",
        serde(bound(deserialize = "Balance: std::str::FromStr"))
    )]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub id: P2PBorrowId,

    #[cfg_attr(
        feature = "std",
        serde(bound(serialize = "Balance: std::fmt::Display"))
    )]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(
        feature = "std",
        serde(bound(deserialize = "Balance: std::str::FromStr"))
    )]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub lock_id: u128,

    pub who: AccountId,
    pub status: P2PBorrowStatus,
    pub borrow_asset_id: AssetId,
    pub collateral_asset_id: AssetId,

    #[cfg_attr(
        feature = "std",
        serde(bound(serialize = "Balance: std::fmt::Display"))
    )]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(
        feature = "std",
        serde(bound(deserialize = "Balance: std::str::FromStr"))
    )]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub borrow_balance: Balance,

    #[cfg_attr(
        feature = "std",
        serde(bound(serialize = "Balance: std::fmt::Display"))
    )]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(
        feature = "std",
        serde(bound(deserialize = "Balance: std::str::FromStr"))
    )]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub collateral_balance: Balance,

    pub terms: u64, // days of our lives
    pub interest_rate: u64,
    pub dead_after: Option<BlockNumber>,
    pub loan_id: Option<P2PLoanId>,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum P2PBorrowStatus {
    Alive,
    Taken,
    Canceled,
    Completed,
    Dead,
    Liquidated,
}
impl Default for P2PBorrowStatus {
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
pub struct P2PBorrowOptions<B, N> {
    pub amount: B,
    pub terms: u64,
    pub interest_rate: u64,
    pub warranty: Option<N>,
}
