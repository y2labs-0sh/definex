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
#[allow(unused_imports)]
use sp_runtime::RuntimeDebug;
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

pub const SEC_PER_DAY: u32 = 86400;
pub const DAYS_PER_YEAR: u32 = 365;
pub const INTEREST_RATE_PREC: u32 = 10000_0000;
pub const LTV_PREC: u32 = 10000;
pub const PRICE_PREC: u32 = 10000;

pub type PriceInUSDT = u64;
pub type LoanId = u64;
// pub type CreditLineId = u64;
pub type LTV = u64;
pub type LoanResult<T = ()> = result::Result<T, DispatchError>;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum LoanHealth {
    Well,
    Liquidating(LTV),
}
impl Default for LoanHealth {
    fn default() -> Self {
        Self::Well
    }
}

#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct CollateralLoan<Balance> {
    pub collateral_amount: Balance,
    pub loan_amount: Balance,
}


#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Loan<AccountId, Balance> {
    pub id: LoanId,
    pub who: AccountId,
    pub collateral_balance_original: Balance,
    pub collateral_balance_available: Balance,
    pub loan_balance_total: Balance,
    pub status: LoanHealth,
}

impl<AccountId, Balance> Loan<AccountId, Balance>
where
    Balance: Encode
        + Decode
        + Parameter
        + Member
        + AtLeast32Bit
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug,
    //  Moment: Parameter + Default + SimpleArithmetic + Copy,
    AccountId: Parameter + Member + MaybeSerializeDeserialize + MaybeDisplay + Ord + Default,
{
    pub fn get_ltv(
        collateral_amount: Balance,
        loan_amount: Balance,
        collection_price: u64,
        collateral_price: u64,
    ) -> LTV {
        let collateral_price = <Balance as TryFrom<u128>>::try_from(collateral_price as u128)
            .ok()
            .unwrap();
        let ltv = (loan_amount * Balance::from(collection_price as u32) * Balance::from(PRICE_PREC) * Balance::from(LTV_PREC))
            / (collateral_amount * collateral_price);
        TryInto::<LTV>::try_into(ltv).ok().unwrap()
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