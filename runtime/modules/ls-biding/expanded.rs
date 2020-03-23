#![feature(prelude_import)]
//! This module is meant for Web3 grant.
//! In this module, definex implemented a DeFi model which follows a 'maker-taker'.
//! Basically, there are 3 major roles:
//!     1. borrower: those who want to borrow money. they can publish their needs (collateral amount, borrow amount, how long they will repay, a specific interest rate, etc.) on the platform.
//!     2. loaner: those who bring liquidity to the platform. they select the borrows that most profitable, and lend the money to the borrower. By doing this, they earn the negotiated interest.
//!     3. liquidator: those who keep monitoring if there is any loan with a ltv lower than the 'LTVLiquidate'. By doing this, they would be rewarded.
//!
//!
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std;
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
};
#[allow(unused_imports)]
use system::{ensure_root, ensure_signed};
#[allow(unused_imports)]
use node_primitives::{BlockNumber, Moment};
pub type LoanId = u128;
pub type BorrowId = u128;
const LOCK_ID: LockIdentifier = *b"dfxlsbrw";
pub const INTEREST_RATE_PRECISION: u64 = 10000_0000;
pub const LTV_SCALE: u32 = 10000;
pub enum LoanHealth {
    Well,
    ToBeLiquidated,
    Overdue,
    Liquidated,
    Dead,
    Completed,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for LoanHealth {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match (&*self,) {
            (&LoanHealth::Well,) => {
                let mut debug_trait_builder = f.debug_tuple("Well");
                debug_trait_builder.finish()
            }
            (&LoanHealth::ToBeLiquidated,) => {
                let mut debug_trait_builder = f.debug_tuple("ToBeLiquidated");
                debug_trait_builder.finish()
            }
            (&LoanHealth::Overdue,) => {
                let mut debug_trait_builder = f.debug_tuple("Overdue");
                debug_trait_builder.finish()
            }
            (&LoanHealth::Liquidated,) => {
                let mut debug_trait_builder = f.debug_tuple("Liquidated");
                debug_trait_builder.finish()
            }
            (&LoanHealth::Dead,) => {
                let mut debug_trait_builder = f.debug_tuple("Dead");
                debug_trait_builder.finish()
            }
            (&LoanHealth::Completed,) => {
                let mut debug_trait_builder = f.debug_tuple("Completed");
                debug_trait_builder.finish()
            }
        }
    }
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Encode for LoanHealth {
        fn encode_to<EncOut: _parity_scale_codec::Output>(&self, dest: &mut EncOut) {
            match *self {
                LoanHealth::Well => {
                    dest.push_byte(0usize as u8);
                }
                LoanHealth::ToBeLiquidated => {
                    dest.push_byte(1usize as u8);
                }
                LoanHealth::Overdue => {
                    dest.push_byte(2usize as u8);
                }
                LoanHealth::Liquidated => {
                    dest.push_byte(3usize as u8);
                }
                LoanHealth::Dead => {
                    dest.push_byte(4usize as u8);
                }
                LoanHealth::Completed => {
                    dest.push_byte(5usize as u8);
                }
                _ => (),
            }
        }
    }
    impl _parity_scale_codec::EncodeLike for LoanHealth {}
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Decode for LoanHealth {
        fn decode<DecIn: _parity_scale_codec::Input>(
            input: &mut DecIn,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            match input.read_byte()? {
                x if x == 0usize as u8 => Ok(LoanHealth::Well),
                x if x == 1usize as u8 => Ok(LoanHealth::ToBeLiquidated),
                x if x == 2usize as u8 => Ok(LoanHealth::Overdue),
                x if x == 3usize as u8 => Ok(LoanHealth::Liquidated),
                x if x == 4usize as u8 => Ok(LoanHealth::Dead),
                x if x == 5usize as u8 => Ok(LoanHealth::Completed),
                x => Err("No such variant in enum LoanHealth".into()),
            }
        }
    }
};
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for LoanHealth {
    #[inline]
    fn clone(&self) -> LoanHealth {
        match (&*self,) {
            (&LoanHealth::Well,) => LoanHealth::Well,
            (&LoanHealth::ToBeLiquidated,) => LoanHealth::ToBeLiquidated,
            (&LoanHealth::Overdue,) => LoanHealth::Overdue,
            (&LoanHealth::Liquidated,) => LoanHealth::Liquidated,
            (&LoanHealth::Dead,) => LoanHealth::Dead,
            (&LoanHealth::Completed,) => LoanHealth::Completed,
        }
    }
}
impl ::core::marker::StructuralPartialEq for LoanHealth {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for LoanHealth {
    #[inline]
    fn eq(&self, other: &LoanHealth) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) } as isize;
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) } as isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    _ => true,
                }
            } else {
                false
            }
        }
    }
}
impl ::core::marker::StructuralEq for LoanHealth {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for LoanHealth {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
impl Default for LoanHealth {
    fn default() -> Self {
        Self::Well
    }
}
pub enum LiquidationType {
    JustCollateral,
    SellCollateral,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for LiquidationType {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match (&*self,) {
            (&LiquidationType::JustCollateral,) => {
                let mut debug_trait_builder = f.debug_tuple("JustCollateral");
                debug_trait_builder.finish()
            }
            (&LiquidationType::SellCollateral,) => {
                let mut debug_trait_builder = f.debug_tuple("SellCollateral");
                debug_trait_builder.finish()
            }
        }
    }
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Encode for LiquidationType {
        fn encode_to<EncOut: _parity_scale_codec::Output>(&self, dest: &mut EncOut) {
            match *self {
                LiquidationType::JustCollateral => {
                    dest.push_byte(0usize as u8);
                }
                LiquidationType::SellCollateral => {
                    dest.push_byte(1usize as u8);
                }
                _ => (),
            }
        }
    }
    impl _parity_scale_codec::EncodeLike for LiquidationType {}
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Decode for LiquidationType {
        fn decode<DecIn: _parity_scale_codec::Input>(
            input: &mut DecIn,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            match input.read_byte()? {
                x if x == 0usize as u8 => Ok(LiquidationType::JustCollateral),
                x if x == 1usize as u8 => Ok(LiquidationType::SellCollateral),
                x => Err("No such variant in enum LiquidationType".into()),
            }
        }
    }
};
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for LiquidationType {
    #[inline]
    fn clone(&self) -> LiquidationType {
        match (&*self,) {
            (&LiquidationType::JustCollateral,) => LiquidationType::JustCollateral,
            (&LiquidationType::SellCollateral,) => LiquidationType::SellCollateral,
        }
    }
}
impl ::core::marker::StructuralPartialEq for LiquidationType {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for LiquidationType {
    #[inline]
    fn eq(&self, other: &LiquidationType) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) } as isize;
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) } as isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    _ => true,
                }
            } else {
                false
            }
        }
    }
}
impl ::core::marker::StructuralEq for LiquidationType {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for LiquidationType {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
impl Default for LiquidationType {
    fn default() -> Self {
        LiquidationType::JustCollateral
    }
}
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
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        AssetId: ::core::fmt::Debug,
        Balance: ::core::fmt::Debug,
        BlockNumber: ::core::fmt::Debug,
        AccountId: ::core::fmt::Debug,
    > ::core::fmt::Debug for Loan<AssetId, Balance, BlockNumber, AccountId>
{
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Loan {
                id: ref __self_0_0,
                borrow_id: ref __self_0_1,
                borrower_id: ref __self_0_2,
                loaner_id: ref __self_0_3,
                due: ref __self_0_4,
                collateral_asset_id: ref __self_0_5,
                collateral_balance: ref __self_0_6,
                loan_balance: ref __self_0_7,
                loan_asset_id: ref __self_0_8,
                status: ref __self_0_9,
                interest_rate: ref __self_0_10,
                liquidation_type: ref __self_0_11,
            } => {
                let mut debug_trait_builder = f.debug_struct("Loan");
                let _ = debug_trait_builder.field("id", &&(*__self_0_0));
                let _ = debug_trait_builder.field("borrow_id", &&(*__self_0_1));
                let _ = debug_trait_builder.field("borrower_id", &&(*__self_0_2));
                let _ = debug_trait_builder.field("loaner_id", &&(*__self_0_3));
                let _ = debug_trait_builder.field("due", &&(*__self_0_4));
                let _ = debug_trait_builder.field("collateral_asset_id", &&(*__self_0_5));
                let _ = debug_trait_builder.field("collateral_balance", &&(*__self_0_6));
                let _ = debug_trait_builder.field("loan_balance", &&(*__self_0_7));
                let _ = debug_trait_builder.field("loan_asset_id", &&(*__self_0_8));
                let _ = debug_trait_builder.field("status", &&(*__self_0_9));
                let _ = debug_trait_builder.field("interest_rate", &&(*__self_0_10));
                let _ = debug_trait_builder.field("liquidation_type", &&(*__self_0_11));
                debug_trait_builder.finish()
            }
        }
    }
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<AssetId, Balance, BlockNumber, AccountId> _parity_scale_codec::Encode
        for Loan<AssetId, Balance, BlockNumber, AccountId>
    where
        AccountId: _parity_scale_codec::Encode,
        AccountId: _parity_scale_codec::Encode,
        AccountId: _parity_scale_codec::Encode,
        AccountId: _parity_scale_codec::Encode,
        BlockNumber: _parity_scale_codec::Encode,
        BlockNumber: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
    {
        fn encode_to<EncOut: _parity_scale_codec::Output>(&self, dest: &mut EncOut) {
            dest.push(&self.id);
            dest.push(&self.borrow_id);
            dest.push(&self.borrower_id);
            dest.push(&self.loaner_id);
            dest.push(&self.due);
            dest.push(&self.collateral_asset_id);
            dest.push(&self.collateral_balance);
            dest.push(&self.loan_balance);
            dest.push(&self.loan_asset_id);
            dest.push(&self.status);
            dest.push(&self.interest_rate);
            dest.push(&self.liquidation_type);
        }
    }
    impl<AssetId, Balance, BlockNumber, AccountId> _parity_scale_codec::EncodeLike
        for Loan<AssetId, Balance, BlockNumber, AccountId>
    where
        AccountId: _parity_scale_codec::Encode,
        AccountId: _parity_scale_codec::Encode,
        AccountId: _parity_scale_codec::Encode,
        AccountId: _parity_scale_codec::Encode,
        BlockNumber: _parity_scale_codec::Encode,
        BlockNumber: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
    {
    }
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<AssetId, Balance, BlockNumber, AccountId> _parity_scale_codec::Decode
        for Loan<AssetId, Balance, BlockNumber, AccountId>
    where
        AccountId: _parity_scale_codec::Decode,
        AccountId: _parity_scale_codec::Decode,
        AccountId: _parity_scale_codec::Decode,
        AccountId: _parity_scale_codec::Decode,
        BlockNumber: _parity_scale_codec::Decode,
        BlockNumber: _parity_scale_codec::Decode,
        AssetId: _parity_scale_codec::Decode,
        AssetId: _parity_scale_codec::Decode,
        Balance: _parity_scale_codec::Decode,
        Balance: _parity_scale_codec::Decode,
        Balance: _parity_scale_codec::Decode,
        Balance: _parity_scale_codec::Decode,
        AssetId: _parity_scale_codec::Decode,
        AssetId: _parity_scale_codec::Decode,
    {
        fn decode<DecIn: _parity_scale_codec::Input>(
            input: &mut DecIn,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            Ok(Loan {
                id: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Loan.id".into()),
                        Ok(a) => a,
                    }
                },
                borrow_id: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Loan.borrow_id".into()),
                        Ok(a) => a,
                    }
                },
                borrower_id: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Loan.borrower_id".into()),
                        Ok(a) => a,
                    }
                },
                loaner_id: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Loan.loaner_id".into()),
                        Ok(a) => a,
                    }
                },
                due: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Loan.due".into()),
                        Ok(a) => a,
                    }
                },
                collateral_asset_id: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field Loan.collateral_asset_id".into())
                        }
                        Ok(a) => a,
                    }
                },
                collateral_balance: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Loan.collateral_balance".into()),
                        Ok(a) => a,
                    }
                },
                loan_balance: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Loan.loan_balance".into()),
                        Ok(a) => a,
                    }
                },
                loan_asset_id: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Loan.loan_asset_id".into()),
                        Ok(a) => a,
                    }
                },
                status: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Loan.status".into()),
                        Ok(a) => a,
                    }
                },
                interest_rate: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Loan.interest_rate".into()),
                        Ok(a) => a,
                    }
                },
                liquidation_type: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Loan.liquidation_type".into()),
                        Ok(a) => a,
                    }
                },
            })
        }
    }
};
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        AssetId: ::core::clone::Clone,
        Balance: ::core::clone::Clone,
        BlockNumber: ::core::clone::Clone,
        AccountId: ::core::clone::Clone,
    > ::core::clone::Clone for Loan<AssetId, Balance, BlockNumber, AccountId>
{
    #[inline]
    fn clone(&self) -> Loan<AssetId, Balance, BlockNumber, AccountId> {
        match *self {
            Loan {
                id: ref __self_0_0,
                borrow_id: ref __self_0_1,
                borrower_id: ref __self_0_2,
                loaner_id: ref __self_0_3,
                due: ref __self_0_4,
                collateral_asset_id: ref __self_0_5,
                collateral_balance: ref __self_0_6,
                loan_balance: ref __self_0_7,
                loan_asset_id: ref __self_0_8,
                status: ref __self_0_9,
                interest_rate: ref __self_0_10,
                liquidation_type: ref __self_0_11,
            } => Loan {
                id: ::core::clone::Clone::clone(&(*__self_0_0)),
                borrow_id: ::core::clone::Clone::clone(&(*__self_0_1)),
                borrower_id: ::core::clone::Clone::clone(&(*__self_0_2)),
                loaner_id: ::core::clone::Clone::clone(&(*__self_0_3)),
                due: ::core::clone::Clone::clone(&(*__self_0_4)),
                collateral_asset_id: ::core::clone::Clone::clone(&(*__self_0_5)),
                collateral_balance: ::core::clone::Clone::clone(&(*__self_0_6)),
                loan_balance: ::core::clone::Clone::clone(&(*__self_0_7)),
                loan_asset_id: ::core::clone::Clone::clone(&(*__self_0_8)),
                status: ::core::clone::Clone::clone(&(*__self_0_9)),
                interest_rate: ::core::clone::Clone::clone(&(*__self_0_10)),
                liquidation_type: ::core::clone::Clone::clone(&(*__self_0_11)),
            },
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        AssetId: ::core::default::Default,
        Balance: ::core::default::Default,
        BlockNumber: ::core::default::Default,
        AccountId: ::core::default::Default,
    > ::core::default::Default for Loan<AssetId, Balance, BlockNumber, AccountId>
{
    #[inline]
    fn default() -> Loan<AssetId, Balance, BlockNumber, AccountId> {
        Loan {
            id: ::core::default::Default::default(),
            borrow_id: ::core::default::Default::default(),
            borrower_id: ::core::default::Default::default(),
            loaner_id: ::core::default::Default::default(),
            due: ::core::default::Default::default(),
            collateral_asset_id: ::core::default::Default::default(),
            collateral_balance: ::core::default::Default::default(),
            loan_balance: ::core::default::Default::default(),
            loan_asset_id: ::core::default::Default::default(),
            status: ::core::default::Default::default(),
            interest_rate: ::core::default::Default::default(),
            liquidation_type: ::core::default::Default::default(),
        }
    }
}
impl<AssetId, Balance, BlockNumber, AccountId> ::core::marker::StructuralPartialEq
    for Loan<AssetId, Balance, BlockNumber, AccountId>
{
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        AssetId: ::core::cmp::PartialEq,
        Balance: ::core::cmp::PartialEq,
        BlockNumber: ::core::cmp::PartialEq,
        AccountId: ::core::cmp::PartialEq,
    > ::core::cmp::PartialEq for Loan<AssetId, Balance, BlockNumber, AccountId>
{
    #[inline]
    fn eq(&self, other: &Loan<AssetId, Balance, BlockNumber, AccountId>) -> bool {
        match *other {
            Loan {
                id: ref __self_1_0,
                borrow_id: ref __self_1_1,
                borrower_id: ref __self_1_2,
                loaner_id: ref __self_1_3,
                due: ref __self_1_4,
                collateral_asset_id: ref __self_1_5,
                collateral_balance: ref __self_1_6,
                loan_balance: ref __self_1_7,
                loan_asset_id: ref __self_1_8,
                status: ref __self_1_9,
                interest_rate: ref __self_1_10,
                liquidation_type: ref __self_1_11,
            } => match *self {
                Loan {
                    id: ref __self_0_0,
                    borrow_id: ref __self_0_1,
                    borrower_id: ref __self_0_2,
                    loaner_id: ref __self_0_3,
                    due: ref __self_0_4,
                    collateral_asset_id: ref __self_0_5,
                    collateral_balance: ref __self_0_6,
                    loan_balance: ref __self_0_7,
                    loan_asset_id: ref __self_0_8,
                    status: ref __self_0_9,
                    interest_rate: ref __self_0_10,
                    liquidation_type: ref __self_0_11,
                } => {
                    (*__self_0_0) == (*__self_1_0)
                        && (*__self_0_1) == (*__self_1_1)
                        && (*__self_0_2) == (*__self_1_2)
                        && (*__self_0_3) == (*__self_1_3)
                        && (*__self_0_4) == (*__self_1_4)
                        && (*__self_0_5) == (*__self_1_5)
                        && (*__self_0_6) == (*__self_1_6)
                        && (*__self_0_7) == (*__self_1_7)
                        && (*__self_0_8) == (*__self_1_8)
                        && (*__self_0_9) == (*__self_1_9)
                        && (*__self_0_10) == (*__self_1_10)
                        && (*__self_0_11) == (*__self_1_11)
                }
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Loan<AssetId, Balance, BlockNumber, AccountId>) -> bool {
        match *other {
            Loan {
                id: ref __self_1_0,
                borrow_id: ref __self_1_1,
                borrower_id: ref __self_1_2,
                loaner_id: ref __self_1_3,
                due: ref __self_1_4,
                collateral_asset_id: ref __self_1_5,
                collateral_balance: ref __self_1_6,
                loan_balance: ref __self_1_7,
                loan_asset_id: ref __self_1_8,
                status: ref __self_1_9,
                interest_rate: ref __self_1_10,
                liquidation_type: ref __self_1_11,
            } => match *self {
                Loan {
                    id: ref __self_0_0,
                    borrow_id: ref __self_0_1,
                    borrower_id: ref __self_0_2,
                    loaner_id: ref __self_0_3,
                    due: ref __self_0_4,
                    collateral_asset_id: ref __self_0_5,
                    collateral_balance: ref __self_0_6,
                    loan_balance: ref __self_0_7,
                    loan_asset_id: ref __self_0_8,
                    status: ref __self_0_9,
                    interest_rate: ref __self_0_10,
                    liquidation_type: ref __self_0_11,
                } => {
                    (*__self_0_0) != (*__self_1_0)
                        || (*__self_0_1) != (*__self_1_1)
                        || (*__self_0_2) != (*__self_1_2)
                        || (*__self_0_3) != (*__self_1_3)
                        || (*__self_0_4) != (*__self_1_4)
                        || (*__self_0_5) != (*__self_1_5)
                        || (*__self_0_6) != (*__self_1_6)
                        || (*__self_0_7) != (*__self_1_7)
                        || (*__self_0_8) != (*__self_1_8)
                        || (*__self_0_9) != (*__self_1_9)
                        || (*__self_0_10) != (*__self_1_10)
                        || (*__self_0_11) != (*__self_1_11)
                }
            },
        }
    }
}
impl<AssetId, Balance, BlockNumber, AccountId> ::core::marker::StructuralEq
    for Loan<AssetId, Balance, BlockNumber, AccountId>
{
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        AssetId: ::core::cmp::Eq,
        Balance: ::core::cmp::Eq,
        BlockNumber: ::core::cmp::Eq,
        AccountId: ::core::cmp::Eq,
    > ::core::cmp::Eq for Loan<AssetId, Balance, BlockNumber, AccountId>
{
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<LoanId>;
            let _: ::core::cmp::AssertParamIsEq<BorrowId>;
            let _: ::core::cmp::AssertParamIsEq<AccountId>;
            let _: ::core::cmp::AssertParamIsEq<AccountId>;
            let _: ::core::cmp::AssertParamIsEq<BlockNumber>;
            let _: ::core::cmp::AssertParamIsEq<AssetId>;
            let _: ::core::cmp::AssertParamIsEq<Balance>;
            let _: ::core::cmp::AssertParamIsEq<Balance>;
            let _: ::core::cmp::AssertParamIsEq<AssetId>;
            let _: ::core::cmp::AssertParamIsEq<LoanHealth>;
            let _: ::core::cmp::AssertParamIsEq<u64>;
            let _: ::core::cmp::AssertParamIsEq<LiquidationType>;
        }
    }
}
pub struct Borrow<AssetId, Balance, BlockNumber, AccountId> {
    pub id: BorrowId,
    pub lock_id: u128,
    pub who: AccountId,
    pub status: BorrowStatus,
    pub borrow_asset_id: AssetId,
    pub collateral_asset_id: AssetId,
    pub borrow_balance: Balance,
    pub collateral_balance: Balance,
    pub terms: u64,
    pub interest_rate: u64,
    pub dead_after: Option<BlockNumber>,
    pub loan_id: Option<LoanId>,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        AssetId: ::core::fmt::Debug,
        Balance: ::core::fmt::Debug,
        BlockNumber: ::core::fmt::Debug,
        AccountId: ::core::fmt::Debug,
    > ::core::fmt::Debug for Borrow<AssetId, Balance, BlockNumber, AccountId>
{
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Borrow {
                id: ref __self_0_0,
                lock_id: ref __self_0_1,
                who: ref __self_0_2,
                status: ref __self_0_3,
                borrow_asset_id: ref __self_0_4,
                collateral_asset_id: ref __self_0_5,
                borrow_balance: ref __self_0_6,
                collateral_balance: ref __self_0_7,
                terms: ref __self_0_8,
                interest_rate: ref __self_0_9,
                dead_after: ref __self_0_10,
                loan_id: ref __self_0_11,
            } => {
                let mut debug_trait_builder = f.debug_struct("Borrow");
                let _ = debug_trait_builder.field("id", &&(*__self_0_0));
                let _ = debug_trait_builder.field("lock_id", &&(*__self_0_1));
                let _ = debug_trait_builder.field("who", &&(*__self_0_2));
                let _ = debug_trait_builder.field("status", &&(*__self_0_3));
                let _ = debug_trait_builder.field("borrow_asset_id", &&(*__self_0_4));
                let _ = debug_trait_builder.field("collateral_asset_id", &&(*__self_0_5));
                let _ = debug_trait_builder.field("borrow_balance", &&(*__self_0_6));
                let _ = debug_trait_builder.field("collateral_balance", &&(*__self_0_7));
                let _ = debug_trait_builder.field("terms", &&(*__self_0_8));
                let _ = debug_trait_builder.field("interest_rate", &&(*__self_0_9));
                let _ = debug_trait_builder.field("dead_after", &&(*__self_0_10));
                let _ = debug_trait_builder.field("loan_id", &&(*__self_0_11));
                debug_trait_builder.finish()
            }
        }
    }
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<AssetId, Balance, BlockNumber, AccountId> _parity_scale_codec::Encode
        for Borrow<AssetId, Balance, BlockNumber, AccountId>
    where
        AccountId: _parity_scale_codec::Encode,
        AccountId: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        Option<BlockNumber>: _parity_scale_codec::Encode,
        Option<BlockNumber>: _parity_scale_codec::Encode,
    {
        fn encode_to<EncOut: _parity_scale_codec::Output>(&self, dest: &mut EncOut) {
            dest.push(&self.id);
            dest.push(&self.lock_id);
            dest.push(&self.who);
            dest.push(&self.status);
            dest.push(&self.borrow_asset_id);
            dest.push(&self.collateral_asset_id);
            dest.push(&self.borrow_balance);
            dest.push(&self.collateral_balance);
            dest.push(&self.terms);
            dest.push(&self.interest_rate);
            dest.push(&self.dead_after);
            dest.push(&self.loan_id);
        }
    }
    impl<AssetId, Balance, BlockNumber, AccountId> _parity_scale_codec::EncodeLike
        for Borrow<AssetId, Balance, BlockNumber, AccountId>
    where
        AccountId: _parity_scale_codec::Encode,
        AccountId: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
        AssetId: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        Balance: _parity_scale_codec::Encode,
        Option<BlockNumber>: _parity_scale_codec::Encode,
        Option<BlockNumber>: _parity_scale_codec::Encode,
    {
    }
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<AssetId, Balance, BlockNumber, AccountId> _parity_scale_codec::Decode
        for Borrow<AssetId, Balance, BlockNumber, AccountId>
    where
        AccountId: _parity_scale_codec::Decode,
        AccountId: _parity_scale_codec::Decode,
        AssetId: _parity_scale_codec::Decode,
        AssetId: _parity_scale_codec::Decode,
        AssetId: _parity_scale_codec::Decode,
        AssetId: _parity_scale_codec::Decode,
        Balance: _parity_scale_codec::Decode,
        Balance: _parity_scale_codec::Decode,
        Balance: _parity_scale_codec::Decode,
        Balance: _parity_scale_codec::Decode,
        Option<BlockNumber>: _parity_scale_codec::Decode,
        Option<BlockNumber>: _parity_scale_codec::Decode,
    {
        fn decode<DecIn: _parity_scale_codec::Input>(
            input: &mut DecIn,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            Ok(Borrow {
                id: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Borrow.id".into()),
                        Ok(a) => a,
                    }
                },
                lock_id: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Borrow.lock_id".into()),
                        Ok(a) => a,
                    }
                },
                who: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Borrow.who".into()),
                        Ok(a) => a,
                    }
                },
                status: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Borrow.status".into()),
                        Ok(a) => a,
                    }
                },
                borrow_asset_id: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Borrow.borrow_asset_id".into()),
                        Ok(a) => a,
                    }
                },
                collateral_asset_id: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field Borrow.collateral_asset_id".into())
                        }
                        Ok(a) => a,
                    }
                },
                borrow_balance: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Borrow.borrow_balance".into()),
                        Ok(a) => a,
                    }
                },
                collateral_balance: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field Borrow.collateral_balance".into())
                        }
                        Ok(a) => a,
                    }
                },
                terms: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Borrow.terms".into()),
                        Ok(a) => a,
                    }
                },
                interest_rate: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Borrow.interest_rate".into()),
                        Ok(a) => a,
                    }
                },
                dead_after: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Borrow.dead_after".into()),
                        Ok(a) => a,
                    }
                },
                loan_id: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Borrow.loan_id".into()),
                        Ok(a) => a,
                    }
                },
            })
        }
    }
};
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        AssetId: ::core::clone::Clone,
        Balance: ::core::clone::Clone,
        BlockNumber: ::core::clone::Clone,
        AccountId: ::core::clone::Clone,
    > ::core::clone::Clone for Borrow<AssetId, Balance, BlockNumber, AccountId>
{
    #[inline]
    fn clone(&self) -> Borrow<AssetId, Balance, BlockNumber, AccountId> {
        match *self {
            Borrow {
                id: ref __self_0_0,
                lock_id: ref __self_0_1,
                who: ref __self_0_2,
                status: ref __self_0_3,
                borrow_asset_id: ref __self_0_4,
                collateral_asset_id: ref __self_0_5,
                borrow_balance: ref __self_0_6,
                collateral_balance: ref __self_0_7,
                terms: ref __self_0_8,
                interest_rate: ref __self_0_9,
                dead_after: ref __self_0_10,
                loan_id: ref __self_0_11,
            } => Borrow {
                id: ::core::clone::Clone::clone(&(*__self_0_0)),
                lock_id: ::core::clone::Clone::clone(&(*__self_0_1)),
                who: ::core::clone::Clone::clone(&(*__self_0_2)),
                status: ::core::clone::Clone::clone(&(*__self_0_3)),
                borrow_asset_id: ::core::clone::Clone::clone(&(*__self_0_4)),
                collateral_asset_id: ::core::clone::Clone::clone(&(*__self_0_5)),
                borrow_balance: ::core::clone::Clone::clone(&(*__self_0_6)),
                collateral_balance: ::core::clone::Clone::clone(&(*__self_0_7)),
                terms: ::core::clone::Clone::clone(&(*__self_0_8)),
                interest_rate: ::core::clone::Clone::clone(&(*__self_0_9)),
                dead_after: ::core::clone::Clone::clone(&(*__self_0_10)),
                loan_id: ::core::clone::Clone::clone(&(*__self_0_11)),
            },
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        AssetId: ::core::default::Default,
        Balance: ::core::default::Default,
        BlockNumber: ::core::default::Default,
        AccountId: ::core::default::Default,
    > ::core::default::Default for Borrow<AssetId, Balance, BlockNumber, AccountId>
{
    #[inline]
    fn default() -> Borrow<AssetId, Balance, BlockNumber, AccountId> {
        Borrow {
            id: ::core::default::Default::default(),
            lock_id: ::core::default::Default::default(),
            who: ::core::default::Default::default(),
            status: ::core::default::Default::default(),
            borrow_asset_id: ::core::default::Default::default(),
            collateral_asset_id: ::core::default::Default::default(),
            borrow_balance: ::core::default::Default::default(),
            collateral_balance: ::core::default::Default::default(),
            terms: ::core::default::Default::default(),
            interest_rate: ::core::default::Default::default(),
            dead_after: ::core::default::Default::default(),
            loan_id: ::core::default::Default::default(),
        }
    }
}
impl<AssetId, Balance, BlockNumber, AccountId> ::core::marker::StructuralPartialEq
    for Borrow<AssetId, Balance, BlockNumber, AccountId>
{
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        AssetId: ::core::cmp::PartialEq,
        Balance: ::core::cmp::PartialEq,
        BlockNumber: ::core::cmp::PartialEq,
        AccountId: ::core::cmp::PartialEq,
    > ::core::cmp::PartialEq for Borrow<AssetId, Balance, BlockNumber, AccountId>
{
    #[inline]
    fn eq(&self, other: &Borrow<AssetId, Balance, BlockNumber, AccountId>) -> bool {
        match *other {
            Borrow {
                id: ref __self_1_0,
                lock_id: ref __self_1_1,
                who: ref __self_1_2,
                status: ref __self_1_3,
                borrow_asset_id: ref __self_1_4,
                collateral_asset_id: ref __self_1_5,
                borrow_balance: ref __self_1_6,
                collateral_balance: ref __self_1_7,
                terms: ref __self_1_8,
                interest_rate: ref __self_1_9,
                dead_after: ref __self_1_10,
                loan_id: ref __self_1_11,
            } => match *self {
                Borrow {
                    id: ref __self_0_0,
                    lock_id: ref __self_0_1,
                    who: ref __self_0_2,
                    status: ref __self_0_3,
                    borrow_asset_id: ref __self_0_4,
                    collateral_asset_id: ref __self_0_5,
                    borrow_balance: ref __self_0_6,
                    collateral_balance: ref __self_0_7,
                    terms: ref __self_0_8,
                    interest_rate: ref __self_0_9,
                    dead_after: ref __self_0_10,
                    loan_id: ref __self_0_11,
                } => {
                    (*__self_0_0) == (*__self_1_0)
                        && (*__self_0_1) == (*__self_1_1)
                        && (*__self_0_2) == (*__self_1_2)
                        && (*__self_0_3) == (*__self_1_3)
                        && (*__self_0_4) == (*__self_1_4)
                        && (*__self_0_5) == (*__self_1_5)
                        && (*__self_0_6) == (*__self_1_6)
                        && (*__self_0_7) == (*__self_1_7)
                        && (*__self_0_8) == (*__self_1_8)
                        && (*__self_0_9) == (*__self_1_9)
                        && (*__self_0_10) == (*__self_1_10)
                        && (*__self_0_11) == (*__self_1_11)
                }
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Borrow<AssetId, Balance, BlockNumber, AccountId>) -> bool {
        match *other {
            Borrow {
                id: ref __self_1_0,
                lock_id: ref __self_1_1,
                who: ref __self_1_2,
                status: ref __self_1_3,
                borrow_asset_id: ref __self_1_4,
                collateral_asset_id: ref __self_1_5,
                borrow_balance: ref __self_1_6,
                collateral_balance: ref __self_1_7,
                terms: ref __self_1_8,
                interest_rate: ref __self_1_9,
                dead_after: ref __self_1_10,
                loan_id: ref __self_1_11,
            } => match *self {
                Borrow {
                    id: ref __self_0_0,
                    lock_id: ref __self_0_1,
                    who: ref __self_0_2,
                    status: ref __self_0_3,
                    borrow_asset_id: ref __self_0_4,
                    collateral_asset_id: ref __self_0_5,
                    borrow_balance: ref __self_0_6,
                    collateral_balance: ref __self_0_7,
                    terms: ref __self_0_8,
                    interest_rate: ref __self_0_9,
                    dead_after: ref __self_0_10,
                    loan_id: ref __self_0_11,
                } => {
                    (*__self_0_0) != (*__self_1_0)
                        || (*__self_0_1) != (*__self_1_1)
                        || (*__self_0_2) != (*__self_1_2)
                        || (*__self_0_3) != (*__self_1_3)
                        || (*__self_0_4) != (*__self_1_4)
                        || (*__self_0_5) != (*__self_1_5)
                        || (*__self_0_6) != (*__self_1_6)
                        || (*__self_0_7) != (*__self_1_7)
                        || (*__self_0_8) != (*__self_1_8)
                        || (*__self_0_9) != (*__self_1_9)
                        || (*__self_0_10) != (*__self_1_10)
                        || (*__self_0_11) != (*__self_1_11)
                }
            },
        }
    }
}
impl<AssetId, Balance, BlockNumber, AccountId> ::core::marker::StructuralEq
    for Borrow<AssetId, Balance, BlockNumber, AccountId>
{
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        AssetId: ::core::cmp::Eq,
        Balance: ::core::cmp::Eq,
        BlockNumber: ::core::cmp::Eq,
        AccountId: ::core::cmp::Eq,
    > ::core::cmp::Eq for Borrow<AssetId, Balance, BlockNumber, AccountId>
{
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<BorrowId>;
            let _: ::core::cmp::AssertParamIsEq<u128>;
            let _: ::core::cmp::AssertParamIsEq<AccountId>;
            let _: ::core::cmp::AssertParamIsEq<BorrowStatus>;
            let _: ::core::cmp::AssertParamIsEq<AssetId>;
            let _: ::core::cmp::AssertParamIsEq<AssetId>;
            let _: ::core::cmp::AssertParamIsEq<Balance>;
            let _: ::core::cmp::AssertParamIsEq<Balance>;
            let _: ::core::cmp::AssertParamIsEq<u64>;
            let _: ::core::cmp::AssertParamIsEq<u64>;
            let _: ::core::cmp::AssertParamIsEq<Option<BlockNumber>>;
            let _: ::core::cmp::AssertParamIsEq<Option<LoanId>>;
        }
    }
}
pub enum BorrowStatus {
    Alive,
    Taken,
    Completed,
    Dead,
    Liquidated,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for BorrowStatus {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match (&*self,) {
            (&BorrowStatus::Alive,) => {
                let mut debug_trait_builder = f.debug_tuple("Alive");
                debug_trait_builder.finish()
            }
            (&BorrowStatus::Taken,) => {
                let mut debug_trait_builder = f.debug_tuple("Taken");
                debug_trait_builder.finish()
            }
            (&BorrowStatus::Completed,) => {
                let mut debug_trait_builder = f.debug_tuple("Completed");
                debug_trait_builder.finish()
            }
            (&BorrowStatus::Dead,) => {
                let mut debug_trait_builder = f.debug_tuple("Dead");
                debug_trait_builder.finish()
            }
            (&BorrowStatus::Liquidated,) => {
                let mut debug_trait_builder = f.debug_tuple("Liquidated");
                debug_trait_builder.finish()
            }
        }
    }
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Encode for BorrowStatus {
        fn encode_to<EncOut: _parity_scale_codec::Output>(&self, dest: &mut EncOut) {
            match *self {
                BorrowStatus::Alive => {
                    dest.push_byte(0usize as u8);
                }
                BorrowStatus::Taken => {
                    dest.push_byte(1usize as u8);
                }
                BorrowStatus::Completed => {
                    dest.push_byte(2usize as u8);
                }
                BorrowStatus::Dead => {
                    dest.push_byte(3usize as u8);
                }
                BorrowStatus::Liquidated => {
                    dest.push_byte(4usize as u8);
                }
                _ => (),
            }
        }
    }
    impl _parity_scale_codec::EncodeLike for BorrowStatus {}
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Decode for BorrowStatus {
        fn decode<DecIn: _parity_scale_codec::Input>(
            input: &mut DecIn,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            match input.read_byte()? {
                x if x == 0usize as u8 => Ok(BorrowStatus::Alive),
                x if x == 1usize as u8 => Ok(BorrowStatus::Taken),
                x if x == 2usize as u8 => Ok(BorrowStatus::Completed),
                x if x == 3usize as u8 => Ok(BorrowStatus::Dead),
                x if x == 4usize as u8 => Ok(BorrowStatus::Liquidated),
                x => Err("No such variant in enum BorrowStatus".into()),
            }
        }
    }
};
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for BorrowStatus {
    #[inline]
    fn clone(&self) -> BorrowStatus {
        match (&*self,) {
            (&BorrowStatus::Alive,) => BorrowStatus::Alive,
            (&BorrowStatus::Taken,) => BorrowStatus::Taken,
            (&BorrowStatus::Completed,) => BorrowStatus::Completed,
            (&BorrowStatus::Dead,) => BorrowStatus::Dead,
            (&BorrowStatus::Liquidated,) => BorrowStatus::Liquidated,
        }
    }
}
impl ::core::marker::StructuralPartialEq for BorrowStatus {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for BorrowStatus {
    #[inline]
    fn eq(&self, other: &BorrowStatus) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) } as isize;
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) } as isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    _ => true,
                }
            } else {
                false
            }
        }
    }
}
impl ::core::marker::StructuralEq for BorrowStatus {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for BorrowStatus {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
impl Default for BorrowStatus {
    fn default() -> Self {
        Self::Alive
    }
}
pub struct TradingPair<A> {
    pub collateral: A,
    pub borrow: A,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<A: ::core::fmt::Debug> ::core::fmt::Debug for TradingPair<A> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            TradingPair {
                collateral: ref __self_0_0,
                borrow: ref __self_0_1,
            } => {
                let mut debug_trait_builder = f.debug_struct("TradingPair");
                let _ = debug_trait_builder.field("collateral", &&(*__self_0_0));
                let _ = debug_trait_builder.field("borrow", &&(*__self_0_1));
                debug_trait_builder.finish()
            }
        }
    }
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<A> _parity_scale_codec::Encode for TradingPair<A>
    where
        A: _parity_scale_codec::Encode,
        A: _parity_scale_codec::Encode,
        A: _parity_scale_codec::Encode,
        A: _parity_scale_codec::Encode,
    {
        fn encode_to<EncOut: _parity_scale_codec::Output>(&self, dest: &mut EncOut) {
            dest.push(&self.collateral);
            dest.push(&self.borrow);
        }
    }
    impl<A> _parity_scale_codec::EncodeLike for TradingPair<A>
    where
        A: _parity_scale_codec::Encode,
        A: _parity_scale_codec::Encode,
        A: _parity_scale_codec::Encode,
        A: _parity_scale_codec::Encode,
    {
    }
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<A> _parity_scale_codec::Decode for TradingPair<A>
    where
        A: _parity_scale_codec::Decode,
        A: _parity_scale_codec::Decode,
        A: _parity_scale_codec::Decode,
        A: _parity_scale_codec::Decode,
    {
        fn decode<DecIn: _parity_scale_codec::Input>(
            input: &mut DecIn,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            Ok(TradingPair {
                collateral: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field TradingPair.collateral".into()),
                        Ok(a) => a,
                    }
                },
                borrow: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field TradingPair.borrow".into()),
                        Ok(a) => a,
                    }
                },
            })
        }
    }
};
#[automatically_derived]
#[allow(unused_qualifications)]
impl<A: ::core::clone::Clone> ::core::clone::Clone for TradingPair<A> {
    #[inline]
    fn clone(&self) -> TradingPair<A> {
        match *self {
            TradingPair {
                collateral: ref __self_0_0,
                borrow: ref __self_0_1,
            } => TradingPair {
                collateral: ::core::clone::Clone::clone(&(*__self_0_0)),
                borrow: ::core::clone::Clone::clone(&(*__self_0_1)),
            },
        }
    }
}
impl<A> ::core::marker::StructuralPartialEq for TradingPair<A> {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<A: ::core::cmp::PartialEq> ::core::cmp::PartialEq for TradingPair<A> {
    #[inline]
    fn eq(&self, other: &TradingPair<A>) -> bool {
        match *other {
            TradingPair {
                collateral: ref __self_1_0,
                borrow: ref __self_1_1,
            } => match *self {
                TradingPair {
                    collateral: ref __self_0_0,
                    borrow: ref __self_0_1,
                } => (*__self_0_0) == (*__self_1_0) && (*__self_0_1) == (*__self_1_1),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &TradingPair<A>) -> bool {
        match *other {
            TradingPair {
                collateral: ref __self_1_0,
                borrow: ref __self_1_1,
            } => match *self {
                TradingPair {
                    collateral: ref __self_0_0,
                    borrow: ref __self_0_1,
                } => (*__self_0_0) != (*__self_1_0) || (*__self_0_1) != (*__self_1_1),
            },
        }
    }
}
impl<A> ::core::marker::StructuralEq for TradingPair<A> {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<A: ::core::cmp::Eq> ::core::cmp::Eq for TradingPair<A> {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<A>;
            let _: ::core::cmp::AssertParamIsEq<A>;
        }
    }
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_SERIALIZE_FOR_TradingPair: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl<A> _serde::Serialize for TradingPair<A>
    where
        A: _serde::Serialize,
    {
        fn serialize<__S>(&self, __serializer: __S) -> _serde::export::Result<__S::Ok, __S::Error>
        where
            __S: _serde::Serializer,
        {
            let mut __serde_state = match _serde::Serializer::serialize_struct(
                __serializer,
                "TradingPair",
                false as usize + 1 + 1,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "collateral",
                &self.collateral,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "borrow",
                &self.borrow,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            _serde::ser::SerializeStruct::end(__serde_state)
        }
    }
};
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_DESERIALIZE_FOR_TradingPair: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl<'de, A> _serde::Deserialize<'de> for TradingPair<A>
    where
        A: _serde::Deserialize<'de>,
    {
        fn deserialize<__D>(__deserializer: __D) -> _serde::export::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            enum __Field {
                __field0,
                __field1,
                __ignore,
            }
            struct __FieldVisitor;
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::export::Formatter,
                ) -> _serde::export::fmt::Result {
                    _serde::export::Formatter::write_str(__formatter, "field identifier")
                }
                fn visit_u64<__E>(self, __value: u64) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::export::Ok(__Field::__field0),
                        1u64 => _serde::export::Ok(__Field::__field1),
                        _ => _serde::export::Err(_serde::de::Error::invalid_value(
                            _serde::de::Unexpected::Unsigned(__value),
                            &"field index 0 <= i < 2",
                        )),
                    }
                }
                fn visit_str<__E>(self, __value: &str) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "collateral" => _serde::export::Ok(__Field::__field0),
                        "borrow" => _serde::export::Ok(__Field::__field1),
                        _ => _serde::export::Ok(__Field::__ignore),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"collateral" => _serde::export::Ok(__Field::__field0),
                        b"borrow" => _serde::export::Ok(__Field::__field1),
                        _ => _serde::export::Ok(__Field::__ignore),
                    }
                }
            }
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(__deserializer: __D) -> _serde::export::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                }
            }
            struct __Visitor<'de, A>
            where
                A: _serde::Deserialize<'de>,
            {
                marker: _serde::export::PhantomData<TradingPair<A>>,
                lifetime: _serde::export::PhantomData<&'de ()>,
            }
            impl<'de, A> _serde::de::Visitor<'de> for __Visitor<'de, A>
            where
                A: _serde::Deserialize<'de>,
            {
                type Value = TradingPair<A>;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::export::Formatter,
                ) -> _serde::export::fmt::Result {
                    _serde::export::Formatter::write_str(__formatter, "struct TradingPair")
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    mut __seq: __A,
                ) -> _serde::export::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::SeqAccess<'de>,
                {
                    let __field0 = match match _serde::de::SeqAccess::next_element::<A>(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                0usize,
                                &"struct TradingPair with 2 elements",
                            ));
                        }
                    };
                    let __field1 = match match _serde::de::SeqAccess::next_element::<A>(&mut __seq)
                    {
                        _serde::export::Ok(__val) => __val,
                        _serde::export::Err(__err) => {
                            return _serde::export::Err(__err);
                        }
                    } {
                        _serde::export::Some(__value) => __value,
                        _serde::export::None => {
                            return _serde::export::Err(_serde::de::Error::invalid_length(
                                1usize,
                                &"struct TradingPair with 2 elements",
                            ));
                        }
                    };
                    _serde::export::Ok(TradingPair {
                        collateral: __field0,
                        borrow: __field1,
                    })
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::export::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    let mut __field0: _serde::export::Option<A> = _serde::export::None;
                    let mut __field1: _serde::export::Option<A> = _serde::export::None;
                    while let _serde::export::Some(__key) =
                        match _serde::de::MapAccess::next_key::<__Field>(&mut __map) {
                            _serde::export::Ok(__val) => __val,
                            _serde::export::Err(__err) => {
                                return _serde::export::Err(__err);
                            }
                        }
                    {
                        match __key {
                            __Field::__field0 => {
                                if _serde::export::Option::is_some(&__field0) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "collateral",
                                        ),
                                    );
                                }
                                __field0 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<A>(&mut __map) {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field1 => {
                                if _serde::export::Option::is_some(&__field1) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "borrow",
                                        ),
                                    );
                                }
                                __field1 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<A>(&mut __map) {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            _ => {
                                let _ = match _serde::de::MapAccess::next_value::<
                                    _serde::de::IgnoredAny,
                                >(&mut __map)
                                {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                };
                            }
                        }
                    }
                    let __field0 = match __field0 {
                        _serde::export::Some(__field0) => __field0,
                        _serde::export::None => {
                            match _serde::private::de::missing_field("collateral") {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            }
                        }
                    };
                    let __field1 = match __field1 {
                        _serde::export::Some(__field1) => __field1,
                        _serde::export::None => {
                            match _serde::private::de::missing_field("borrow") {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            }
                        }
                    };
                    _serde::export::Ok(TradingPair {
                        collateral: __field0,
                        borrow: __field1,
                    })
                }
            }
            const FIELDS: &'static [&'static str] = &["collateral", "borrow"];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "TradingPair",
                FIELDS,
                __Visitor {
                    marker: _serde::export::PhantomData::<TradingPair<A>>,
                    lifetime: _serde::export::PhantomData,
                },
            )
        }
    }
};
pub struct TradingPairPrices {
    pub borrow_asset_price: u64,
    pub collateral_asset_price: u64,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for TradingPairPrices {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            TradingPairPrices {
                borrow_asset_price: ref __self_0_0,
                collateral_asset_price: ref __self_0_1,
            } => {
                let mut debug_trait_builder = f.debug_struct("TradingPairPrices");
                let _ = debug_trait_builder.field("borrow_asset_price", &&(*__self_0_0));
                let _ = debug_trait_builder.field("collateral_asset_price", &&(*__self_0_1));
                debug_trait_builder.finish()
            }
        }
    }
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Encode for TradingPairPrices {
        fn encode_to<EncOut: _parity_scale_codec::Output>(&self, dest: &mut EncOut) {
            dest.push(&self.borrow_asset_price);
            dest.push(&self.collateral_asset_price);
        }
    }
    impl _parity_scale_codec::EncodeLike for TradingPairPrices {}
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Decode for TradingPairPrices {
        fn decode<DecIn: _parity_scale_codec::Input>(
            input: &mut DecIn,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            Ok(TradingPairPrices {
                borrow_asset_price: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err(
                                "Error decoding field TradingPairPrices.borrow_asset_price".into()
                            )
                        }
                        Ok(a) => a,
                    }
                },
                collateral_asset_price: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err(
                                "Error decoding field TradingPairPrices.collateral_asset_price"
                                    .into(),
                            )
                        }
                        Ok(a) => a,
                    }
                },
            })
        }
    }
};
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for TradingPairPrices {
    #[inline]
    fn clone(&self) -> TradingPairPrices {
        match *self {
            TradingPairPrices {
                borrow_asset_price: ref __self_0_0,
                collateral_asset_price: ref __self_0_1,
            } => TradingPairPrices {
                borrow_asset_price: ::core::clone::Clone::clone(&(*__self_0_0)),
                collateral_asset_price: ::core::clone::Clone::clone(&(*__self_0_1)),
            },
        }
    }
}
impl ::core::marker::StructuralPartialEq for TradingPairPrices {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for TradingPairPrices {
    #[inline]
    fn eq(&self, other: &TradingPairPrices) -> bool {
        match *other {
            TradingPairPrices {
                borrow_asset_price: ref __self_1_0,
                collateral_asset_price: ref __self_1_1,
            } => match *self {
                TradingPairPrices {
                    borrow_asset_price: ref __self_0_0,
                    collateral_asset_price: ref __self_0_1,
                } => (*__self_0_0) == (*__self_1_0) && (*__self_0_1) == (*__self_1_1),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &TradingPairPrices) -> bool {
        match *other {
            TradingPairPrices {
                borrow_asset_price: ref __self_1_0,
                collateral_asset_price: ref __self_1_1,
            } => match *self {
                TradingPairPrices {
                    borrow_asset_price: ref __self_0_0,
                    collateral_asset_price: ref __self_0_1,
                } => (*__self_0_0) != (*__self_1_0) || (*__self_0_1) != (*__self_1_1),
            },
        }
    }
}
impl ::core::marker::StructuralEq for TradingPairPrices {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for TradingPairPrices {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<u64>;
            let _: ::core::cmp::AssertParamIsEq<u64>;
        }
    }
}
pub struct BorrowOptions<B, N> {
    pub amount: B,
    pub terms: u64,
    pub interest_rate: u64,
    pub warranty: Option<N>,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<B: ::core::fmt::Debug, N: ::core::fmt::Debug> ::core::fmt::Debug for BorrowOptions<B, N> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            BorrowOptions {
                amount: ref __self_0_0,
                terms: ref __self_0_1,
                interest_rate: ref __self_0_2,
                warranty: ref __self_0_3,
            } => {
                let mut debug_trait_builder = f.debug_struct("BorrowOptions");
                let _ = debug_trait_builder.field("amount", &&(*__self_0_0));
                let _ = debug_trait_builder.field("terms", &&(*__self_0_1));
                let _ = debug_trait_builder.field("interest_rate", &&(*__self_0_2));
                let _ = debug_trait_builder.field("warranty", &&(*__self_0_3));
                debug_trait_builder.finish()
            }
        }
    }
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<B, N> _parity_scale_codec::Encode for BorrowOptions<B, N>
    where
        B: _parity_scale_codec::Encode,
        B: _parity_scale_codec::Encode,
        Option<N>: _parity_scale_codec::Encode,
        Option<N>: _parity_scale_codec::Encode,
    {
        fn encode_to<EncOut: _parity_scale_codec::Output>(&self, dest: &mut EncOut) {
            dest.push(&self.amount);
            dest.push(&self.terms);
            dest.push(&self.interest_rate);
            dest.push(&self.warranty);
        }
    }
    impl<B, N> _parity_scale_codec::EncodeLike for BorrowOptions<B, N>
    where
        B: _parity_scale_codec::Encode,
        B: _parity_scale_codec::Encode,
        Option<N>: _parity_scale_codec::Encode,
        Option<N>: _parity_scale_codec::Encode,
    {
    }
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<B, N> _parity_scale_codec::Decode for BorrowOptions<B, N>
    where
        B: _parity_scale_codec::Decode,
        B: _parity_scale_codec::Decode,
        Option<N>: _parity_scale_codec::Decode,
        Option<N>: _parity_scale_codec::Decode,
    {
        fn decode<DecIn: _parity_scale_codec::Input>(
            input: &mut DecIn,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            Ok(BorrowOptions {
                amount: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field BorrowOptions.amount".into()),
                        Ok(a) => a,
                    }
                },
                terms: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field BorrowOptions.terms".into()),
                        Ok(a) => a,
                    }
                },
                interest_rate: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field BorrowOptions.interest_rate".into())
                        }
                        Ok(a) => a,
                    }
                },
                warranty: {
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field BorrowOptions.warranty".into()),
                        Ok(a) => a,
                    }
                },
            })
        }
    }
};
#[automatically_derived]
#[allow(unused_qualifications)]
impl<B: ::core::clone::Clone, N: ::core::clone::Clone> ::core::clone::Clone
    for BorrowOptions<B, N>
{
    #[inline]
    fn clone(&self) -> BorrowOptions<B, N> {
        match *self {
            BorrowOptions {
                amount: ref __self_0_0,
                terms: ref __self_0_1,
                interest_rate: ref __self_0_2,
                warranty: ref __self_0_3,
            } => BorrowOptions {
                amount: ::core::clone::Clone::clone(&(*__self_0_0)),
                terms: ::core::clone::Clone::clone(&(*__self_0_1)),
                interest_rate: ::core::clone::Clone::clone(&(*__self_0_2)),
                warranty: ::core::clone::Clone::clone(&(*__self_0_3)),
            },
        }
    }
}
impl<B, N> ::core::marker::StructuralPartialEq for BorrowOptions<B, N> {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<B: ::core::cmp::PartialEq, N: ::core::cmp::PartialEq> ::core::cmp::PartialEq
    for BorrowOptions<B, N>
{
    #[inline]
    fn eq(&self, other: &BorrowOptions<B, N>) -> bool {
        match *other {
            BorrowOptions {
                amount: ref __self_1_0,
                terms: ref __self_1_1,
                interest_rate: ref __self_1_2,
                warranty: ref __self_1_3,
            } => match *self {
                BorrowOptions {
                    amount: ref __self_0_0,
                    terms: ref __self_0_1,
                    interest_rate: ref __self_0_2,
                    warranty: ref __self_0_3,
                } => {
                    (*__self_0_0) == (*__self_1_0)
                        && (*__self_0_1) == (*__self_1_1)
                        && (*__self_0_2) == (*__self_1_2)
                        && (*__self_0_3) == (*__self_1_3)
                }
            },
        }
    }
    #[inline]
    fn ne(&self, other: &BorrowOptions<B, N>) -> bool {
        match *other {
            BorrowOptions {
                amount: ref __self_1_0,
                terms: ref __self_1_1,
                interest_rate: ref __self_1_2,
                warranty: ref __self_1_3,
            } => match *self {
                BorrowOptions {
                    amount: ref __self_0_0,
                    terms: ref __self_0_1,
                    interest_rate: ref __self_0_2,
                    warranty: ref __self_0_3,
                } => {
                    (*__self_0_0) != (*__self_1_0)
                        || (*__self_0_1) != (*__self_1_1)
                        || (*__self_0_2) != (*__self_1_2)
                        || (*__self_0_3) != (*__self_1_3)
                }
            },
        }
    }
}
impl<B, N> ::core::marker::StructuralEq for BorrowOptions<B, N> {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<B: ::core::cmp::Eq, N: ::core::cmp::Eq> ::core::cmp::Eq for BorrowOptions<B, N> {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<B>;
            let _: ::core::cmp::AssertParamIsEq<u64>;
            let _: ::core::cmp::AssertParamIsEq<u64>;
            let _: ::core::cmp::AssertParamIsEq<Option<N>>;
        }
    }
}
/// The module's configuration trait.
pub trait Trait:
    generic_asset::Trait + timestamp::Trait + system::Trait + new_oracle::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Days: Get<Self::BlockNumber>;
}
use self::sp_api_hidden_includes_decl_storage::hidden_include::{
    StorageValue as _, StorageMap as _, StorageLinkedMap as _, StorageDoubleMap as _,
    StoragePrefixedMap as _,
};
#[doc(hidden)]
mod sp_api_hidden_includes_decl_storage {
    pub extern crate support as hidden_include;
}
trait Store {
    type Paused;
    type MoneyPool;
    type Platform;
    type TradingPairs;
    type SafeLTV;
    type LiquidateLTV;
    type MinBorrowTerms;
    type MinBorrowInterestRate;
    type NextBorrowId;
    type NextLoanId;
    type Borrows;
    type BorrowIdsByAccountId;
    type AliveBorrowIds;
    type Loans;
    type LoanIdsByAccountId;
    type AliveLoanIdsByAccountId;
    type AccountIdsWithLiveLoans;
}
impl<T: Trait + 'static> Store for Module<T> {
    type Paused = Paused;
    type MoneyPool = MoneyPool<T>;
    type Platform = Platform<T>;
    type TradingPairs = TradingPairs<T>;
    type SafeLTV = SafeLTV;
    type LiquidateLTV = LiquidateLTV;
    type MinBorrowTerms = MinBorrowTerms;
    type MinBorrowInterestRate = MinBorrowInterestRate;
    type NextBorrowId = NextBorrowId;
    type NextLoanId = NextLoanId;
    type Borrows = Borrows<T>;
    type BorrowIdsByAccountId = BorrowIdsByAccountId<T>;
    type AliveBorrowIds = AliveBorrowIds;
    type Loans = Loans<T>;
    type LoanIdsByAccountId = LoanIdsByAccountId<T>;
    type AliveLoanIdsByAccountId = AliveLoanIdsByAccountId<T>;
    type AccountIdsWithLiveLoans = AccountIdsWithLiveLoans<T>;
}
impl<T: Trait + 'static> Module<T> {
    /// module level switch
    pub fn paused() -> bool {
        < Paused < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < bool > > :: get ( )
    }
    /// hold borrowers' collateral temporarily
    pub fn money_pool() -> T::AccountId {
        < MoneyPool < T > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < T :: AccountId > > :: get ( )
    }
    /// Platform is just a account receiving potential fees
    pub fn platform() -> T::AccountId {
        < Platform < T > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < T :: AccountId > > :: get ( )
    }
    /// TradingPairs contains all supported trading pairs, oracle should provide price information for all trading pairs.
    pub fn trading_pairs() -> Vec<TradingPair<T::AssetId>> {
        < TradingPairs < T > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < Vec < TradingPair < T :: AssetId > > > > :: get ( )
    }
    /// LTV must be greater than this value to create a new borrow
    pub fn safe_ltv() -> u32 {
        < SafeLTV < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < u32 > > :: get ( )
    }
    /// a loan will be liquidated when LTV is below this
    pub fn liquidate_ltv() -> u32 {
        < LiquidateLTV < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < u32 > > :: get ( )
    }
    /// minimium borrow terms, count in natural days
    pub fn min_borrow_terms() -> u64 {
        < MinBorrowTerms < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < u64 > > :: get ( )
    }
    /// minimium interest rate
    pub fn min_borrow_interest_rate() -> u64 {
        < MinBorrowInterestRate < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < u64 > > :: get ( )
    }
    /// borrow id counter
    pub fn next_borrow_id() -> BorrowId {
        < NextBorrowId < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < BorrowId > > :: get ( )
    }
    /// loan id counter
    pub fn next_loan_id() -> LoanId {
        < NextLoanId < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < LoanId > > :: get ( )
    }
    /// an account can only have one alive borrow at a time
    pub fn borrows<
        K: self::sp_api_hidden_includes_decl_storage::hidden_include::codec::EncodeLike<BorrowId>,
    >(
        key: K,
    ) -> Borrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId> {
        < Borrows < T > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageLinkedMap < BorrowId , Borrow < T :: AssetId , T :: Balance , T :: BlockNumber , T :: AccountId > > > :: get ( key )
    }
    pub fn borrow_ids_by_account_id<
        K: self::sp_api_hidden_includes_decl_storage::hidden_include::codec::EncodeLike<T::AccountId>,
    >(
        key: K,
    ) -> Vec<BorrowId> {
        < BorrowIdsByAccountId < T > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageMap < T :: AccountId , Vec < BorrowId > > > :: get ( key )
    }
    pub fn alive_borrow_ids() -> Vec<BorrowId> {
        < AliveBorrowIds < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < Vec < BorrowId > > > :: get ( )
    }
    /// on the other hand, an account can have multiple alive loans
    pub fn loans<
        K: self::sp_api_hidden_includes_decl_storage::hidden_include::codec::EncodeLike<LoanId>,
    >(
        key: K,
    ) -> Loan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId> {
        < Loans < T > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageLinkedMap < LoanId , Loan < T :: AssetId , T :: Balance , T :: BlockNumber , T :: AccountId > > > :: get ( key )
    }
    pub fn loan_ids_by_account_id<
        K: self::sp_api_hidden_includes_decl_storage::hidden_include::codec::EncodeLike<T::AccountId>,
    >(
        key: K,
    ) -> Vec<LoanId> {
        < LoanIdsByAccountId < T > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageMap < T :: AccountId , Vec < LoanId > > > :: get ( key )
    }
    pub fn alive_loan_ids_by_account_id<
        K: self::sp_api_hidden_includes_decl_storage::hidden_include::codec::EncodeLike<T::AccountId>,
    >(
        key: K,
    ) -> Vec<LoanId> {
        < AliveLoanIdsByAccountId < T > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageMap < T :: AccountId , Vec < LoanId > > > :: get ( key )
    }
    pub fn account_ids_with_loans() -> Vec<T::AccountId> {
        < AccountIdsWithLiveLoans < T > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < Vec < T :: AccountId > > > :: get ( )
    }
}
#[doc(hidden)]
pub struct __GetByteStructPaused<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_Paused:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructPaused<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_Paused
            .get_or_init(|| {
                let def_val: bool = false;
                <bool as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructPaused<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructPaused<T> {}
#[doc(hidden)]
pub struct __GetByteStructMoneyPool<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_MoneyPool:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructMoneyPool<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_MoneyPool
            .get_or_init(|| {
                let def_val: T::AccountId = Default::default();
                <T::AccountId as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructMoneyPool<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructMoneyPool<T> {}
#[doc(hidden)]
pub struct __GetByteStructPlatform<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_Platform:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructPlatform<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_Platform
            .get_or_init(|| {
                let def_val: T::AccountId = Default::default();
                <T::AccountId as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructPlatform<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructPlatform<T> {}
#[doc(hidden)]
pub struct __GetByteStructTradingPairs<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_TradingPairs:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructTradingPairs<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_TradingPairs
            .get_or_init(|| {
                let def_val: Vec<TradingPair<T::AssetId>> = Default::default();
                <Vec<TradingPair<T::AssetId>> as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructTradingPairs<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructTradingPairs<T> {}
#[doc(hidden)]
pub struct __GetByteStructSafeLTV<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_SafeLTV:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructSafeLTV<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_SafeLTV
            .get_or_init(|| {
                let def_val: u32 = Default::default();
                <u32 as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructSafeLTV<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructSafeLTV<T> {}
#[doc(hidden)]
pub struct __GetByteStructLiquidateLTV<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_LiquidateLTV:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructLiquidateLTV<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_LiquidateLTV
            .get_or_init(|| {
                let def_val: u32 = Default::default();
                <u32 as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructLiquidateLTV<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructLiquidateLTV<T> {}
#[doc(hidden)]
pub struct __GetByteStructMinBorrowTerms<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_MinBorrowTerms:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructMinBorrowTerms<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_MinBorrowTerms
            .get_or_init(|| {
                let def_val: u64 = Default::default();
                <u64 as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructMinBorrowTerms<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructMinBorrowTerms<T> {}
#[doc(hidden)]
pub struct __GetByteStructMinBorrowInterestRate<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_MinBorrowInterestRate:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructMinBorrowInterestRate<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_MinBorrowInterestRate
            .get_or_init(|| {
                let def_val: u64 = Default::default();
                <u64 as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructMinBorrowInterestRate<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructMinBorrowInterestRate<T> {}
#[doc(hidden)]
pub struct __GetByteStructNextBorrowId<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_NextBorrowId:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructNextBorrowId<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_NextBorrowId
            .get_or_init(|| {
                let def_val: BorrowId = 1;
                <BorrowId as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructNextBorrowId<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructNextBorrowId<T> {}
#[doc(hidden)]
pub struct __GetByteStructNextLoanId<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_NextLoanId:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructNextLoanId<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_NextLoanId
            .get_or_init(|| {
                let def_val: LoanId = 1;
                <LoanId as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructNextLoanId<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructNextLoanId<T> {}
#[doc(hidden)]
pub struct __GetByteStructBorrows<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_Borrows:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructBorrows<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_Borrows
            .get_or_init(|| {
                let def_val: Borrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId> =
                    Default::default();
                <Borrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId> as Encode>::encode(
                    &def_val,
                )
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructBorrows<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructBorrows<T> {}
#[doc(hidden)]
pub struct __GetByteStructBorrowIdsByAccountId<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_BorrowIdsByAccountId:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructBorrowIdsByAccountId<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_BorrowIdsByAccountId
            .get_or_init(|| {
                let def_val: Vec<BorrowId> = Default::default();
                <Vec<BorrowId> as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructBorrowIdsByAccountId<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructBorrowIdsByAccountId<T> {}
#[doc(hidden)]
pub struct __GetByteStructAliveBorrowIds<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_AliveBorrowIds:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructAliveBorrowIds<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_AliveBorrowIds
            .get_or_init(|| {
                let def_val: Vec<BorrowId> = Default::default();
                <Vec<BorrowId> as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructAliveBorrowIds<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructAliveBorrowIds<T> {}
#[doc(hidden)]
pub struct __GetByteStructLoans<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_Loans:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructLoans<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_Loans
            .get_or_init(|| {
                let def_val: Loan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId> =
                    Default::default();
                <Loan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId> as Encode>::encode(
                    &def_val,
                )
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructLoans<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructLoans<T> {}
#[doc(hidden)]
pub struct __GetByteStructLoanIdsByAccountId<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_LoanIdsByAccountId:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructLoanIdsByAccountId<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_LoanIdsByAccountId
            .get_or_init(|| {
                let def_val: Vec<LoanId> = Default::default();
                <Vec<LoanId> as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructLoanIdsByAccountId<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructLoanIdsByAccountId<T> {}
#[doc(hidden)]
pub struct __GetByteStructAliveLoanIdsByAccountId<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_AliveLoanIdsByAccountId:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructAliveLoanIdsByAccountId<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_AliveLoanIdsByAccountId
            .get_or_init(|| {
                let def_val: Vec<LoanId> = Default::default();
                <Vec<LoanId> as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructAliveLoanIdsByAccountId<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructAliveLoanIdsByAccountId<T> {}
#[doc(hidden)]
pub struct __GetByteStructAccountIdsWithLiveLoans<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_AccountIdsWithLiveLoans:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructAccountIdsWithLiveLoans<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_AccountIdsWithLiveLoans
            .get_or_init(|| {
                let def_val: Vec<T::AccountId> = Default::default();
                <Vec<T::AccountId> as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructAccountIdsWithLiveLoans<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructAccountIdsWithLiveLoans<T> {}
impl<T: Trait + 'static> Module<T> {
    #[doc(hidden)]
    pub fn storage_metadata(
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::StorageMetadata {
        self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageMetadata { prefix : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "LSBiding" ) , entries : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "Paused" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Plain ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "bool" ) ) , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructPaused :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " module level switch" ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "MoneyPool" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Plain ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "T::AccountId" ) ) , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructMoneyPool :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " hold borrowers\' collateral temporarily" ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "Platform" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Plain ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "T::AccountId" ) ) , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructPlatform :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " Platform is just a account receiving potential fees" ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "TradingPairs" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Plain ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "Vec<TradingPair<T::AssetId>>" ) ) , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructTradingPairs :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " TradingPairs contains all supported trading pairs, oracle should provide price information for all trading pairs." ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "SafeLTV" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Plain ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "u32" ) ) , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructSafeLTV :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " LTV must be greater than this value to create a new borrow" ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "LiquidateLTV" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Plain ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "u32" ) ) , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructLiquidateLTV :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " a loan will be liquidated when LTV is below this" ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "MinBorrowTerms" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Plain ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "u64" ) ) , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructMinBorrowTerms :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " minimium borrow terms, count in natural days" ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "MinBorrowInterestRate" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Plain ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "u64" ) ) , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructMinBorrowInterestRate :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " minimium interest rate" ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "NextBorrowId" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Plain ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "BorrowId" ) ) , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructNextBorrowId :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " borrow id counter" ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "NextLoanId" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Plain ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "LoanId" ) ) , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructNextLoanId :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " loan id counter" ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "Borrows" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Map { hasher : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageHasher :: Blake2_256 , key : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "BorrowId" ) , value : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "Borrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>" ) , is_linked : true , } , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructBorrows :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " an account can only have one alive borrow at a time" ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "BorrowIdsByAccountId" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Map { hasher : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageHasher :: Blake2_256 , key : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "T::AccountId" ) , value : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "Vec<BorrowId>" ) , is_linked : false , } , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructBorrowIdsByAccountId :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "AliveBorrowIds" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Plain ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "Vec<BorrowId>" ) ) , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructAliveBorrowIds :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "Loans" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Map { hasher : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageHasher :: Blake2_256 , key : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "LoanId" ) , value : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "Loan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>" ) , is_linked : true , } , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructLoans :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " on the other hand, an account can have multiple alive loans" ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "LoanIdsByAccountId" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Map { hasher : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageHasher :: Blake2_256 , key : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "T::AccountId" ) , value : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "Vec<LoanId>" ) , is_linked : false , } , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructLoanIdsByAccountId :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "AliveLoanIdsByAccountId" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Map { hasher : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageHasher :: Blake2_256 , key : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "T::AccountId" ) , value : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "Vec<LoanId>" ) , is_linked : false , } , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructAliveLoanIdsByAccountId :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "AccountIdsWithLiveLoans" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Plain ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "Vec<T::AccountId>" ) ) , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructAccountIdsWithLiveLoans :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ ] ) , } ] [ .. ] ) , }
    }
}
/// Tag a type as an instance of a module.
///
/// Defines storage prefixes, they must be unique.
#[doc(hidden)]
pub trait __GeneratedInstantiable: 'static {
    /// The prefix used by any storage entry of an instance.
    const PREFIX: &'static str;
}
#[doc(hidden)]
pub struct __InherentHiddenInstance;
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for __InherentHiddenInstance {
    #[inline]
    fn clone(&self) -> __InherentHiddenInstance {
        match *self {
            __InherentHiddenInstance => __InherentHiddenInstance,
        }
    }
}
impl ::core::marker::StructuralEq for __InherentHiddenInstance {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for __InherentHiddenInstance {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
impl ::core::marker::StructuralPartialEq for __InherentHiddenInstance {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for __InherentHiddenInstance {
    #[inline]
    fn eq(&self, other: &__InherentHiddenInstance) -> bool {
        match *other {
            __InherentHiddenInstance => match *self {
                __InherentHiddenInstance => true,
            },
        }
    }
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Encode for __InherentHiddenInstance {
        fn encode_to<EncOut: _parity_scale_codec::Output>(&self, dest: &mut EncOut) {}
    }
    impl _parity_scale_codec::EncodeLike for __InherentHiddenInstance {}
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Decode for __InherentHiddenInstance {
        fn decode<DecIn: _parity_scale_codec::Input>(
            input: &mut DecIn,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            Ok(__InherentHiddenInstance)
        }
    }
};
impl core::fmt::Debug for __InherentHiddenInstance {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        fmt.debug_tuple("__InherentHiddenInstance").finish()
    }
}
impl __GeneratedInstantiable for __InherentHiddenInstance {
    const PREFIX: &'static str = "LSBiding";
}
#[cfg(feature = "std")]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
#[serde(bound(
    serialize = "T :: AccountId : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::Serialize, T :: AccountId : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::Serialize, Vec < TradingPair < T :: AssetId > > : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::Serialize, u32 : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::Serialize, u32 : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::Serialize, u64 : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::Serialize, u64 : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::Serialize, "
))]
#[serde(bound(
    deserialize = "T :: AccountId : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::de::DeserializeOwned, T :: AccountId : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::de::DeserializeOwned, Vec < TradingPair < T :: AssetId > > : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::de::DeserializeOwned, u32 : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::de::DeserializeOwned, u32 : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::de::DeserializeOwned, u64 : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::de::DeserializeOwned, u64 : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::de::DeserializeOwned, "
))]
pub struct GenesisConfig<T: Trait> {
    /// hold borrowers' collateral temporarily
    pub money_pool: T::AccountId,
    /// Platform is just a account receiving potential fees
    pub platform: T::AccountId,
    /// TradingPairs contains all supported trading pairs, oracle should provide price information for all trading pairs.
    pub trading_pairs: Vec<TradingPair<T::AssetId>>,
    /// LTV must be greater than this value to create a new borrow
    pub safe_ltv: u32,
    /// a loan will be liquidated when LTV is below this
    pub liquidate_ltv: u32,
    /// minimium borrow terms, count in natural days
    pub min_borrow_terms: u64,
    /// minimium interest rate
    pub min_borrow_interest_rate: u64,
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_SERIALIZE_FOR_GenesisConfig: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl<T: Trait> _serde::Serialize for GenesisConfig<T>
    where
        T::AccountId: self::sp_api_hidden_includes_decl_storage::hidden_include::serde::Serialize,
        T::AccountId: self::sp_api_hidden_includes_decl_storage::hidden_include::serde::Serialize,
        Vec<TradingPair<T::AssetId>>:
            self::sp_api_hidden_includes_decl_storage::hidden_include::serde::Serialize,
        u32: self::sp_api_hidden_includes_decl_storage::hidden_include::serde::Serialize,
        u32: self::sp_api_hidden_includes_decl_storage::hidden_include::serde::Serialize,
        u64: self::sp_api_hidden_includes_decl_storage::hidden_include::serde::Serialize,
        u64: self::sp_api_hidden_includes_decl_storage::hidden_include::serde::Serialize,
    {
        fn serialize<__S>(&self, __serializer: __S) -> _serde::export::Result<__S::Ok, __S::Error>
        where
            __S: _serde::Serializer,
        {
            let mut __serde_state = match _serde::Serializer::serialize_struct(
                __serializer,
                "GenesisConfig",
                false as usize + 1 + 1 + 1 + 1 + 1 + 1 + 1,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "moneyPool",
                &self.money_pool,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "platform",
                &self.platform,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "tradingPairs",
                &self.trading_pairs,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "safeLtv",
                &self.safe_ltv,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "liquidateLtv",
                &self.liquidate_ltv,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "minBorrowTerms",
                &self.min_borrow_terms,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "minBorrowInterestRate",
                &self.min_borrow_interest_rate,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            _serde::ser::SerializeStruct::end(__serde_state)
        }
    }
};
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_DESERIALIZE_FOR_GenesisConfig: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl<'de, T: Trait> _serde::Deserialize<'de> for GenesisConfig<T>
    where
        T::AccountId:
            self::sp_api_hidden_includes_decl_storage::hidden_include::serde::de::DeserializeOwned,
        T::AccountId:
            self::sp_api_hidden_includes_decl_storage::hidden_include::serde::de::DeserializeOwned,
        Vec<TradingPair<T::AssetId>>:
            self::sp_api_hidden_includes_decl_storage::hidden_include::serde::de::DeserializeOwned,
        u32: self::sp_api_hidden_includes_decl_storage::hidden_include::serde::de::DeserializeOwned,
        u32: self::sp_api_hidden_includes_decl_storage::hidden_include::serde::de::DeserializeOwned,
        u64: self::sp_api_hidden_includes_decl_storage::hidden_include::serde::de::DeserializeOwned,
        u64: self::sp_api_hidden_includes_decl_storage::hidden_include::serde::de::DeserializeOwned,
    {
        fn deserialize<__D>(__deserializer: __D) -> _serde::export::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            enum __Field {
                __field0,
                __field1,
                __field2,
                __field3,
                __field4,
                __field5,
                __field6,
            }
            struct __FieldVisitor;
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::export::Formatter,
                ) -> _serde::export::fmt::Result {
                    _serde::export::Formatter::write_str(__formatter, "field identifier")
                }
                fn visit_u64<__E>(self, __value: u64) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::export::Ok(__Field::__field0),
                        1u64 => _serde::export::Ok(__Field::__field1),
                        2u64 => _serde::export::Ok(__Field::__field2),
                        3u64 => _serde::export::Ok(__Field::__field3),
                        4u64 => _serde::export::Ok(__Field::__field4),
                        5u64 => _serde::export::Ok(__Field::__field5),
                        6u64 => _serde::export::Ok(__Field::__field6),
                        _ => _serde::export::Err(_serde::de::Error::invalid_value(
                            _serde::de::Unexpected::Unsigned(__value),
                            &"field index 0 <= i < 7",
                        )),
                    }
                }
                fn visit_str<__E>(self, __value: &str) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "moneyPool" => _serde::export::Ok(__Field::__field0),
                        "platform" => _serde::export::Ok(__Field::__field1),
                        "tradingPairs" => _serde::export::Ok(__Field::__field2),
                        "safeLtv" => _serde::export::Ok(__Field::__field3),
                        "liquidateLtv" => _serde::export::Ok(__Field::__field4),
                        "minBorrowTerms" => _serde::export::Ok(__Field::__field5),
                        "minBorrowInterestRate" => _serde::export::Ok(__Field::__field6),
                        _ => _serde::export::Err(_serde::de::Error::unknown_field(__value, FIELDS)),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"moneyPool" => _serde::export::Ok(__Field::__field0),
                        b"platform" => _serde::export::Ok(__Field::__field1),
                        b"tradingPairs" => _serde::export::Ok(__Field::__field2),
                        b"safeLtv" => _serde::export::Ok(__Field::__field3),
                        b"liquidateLtv" => _serde::export::Ok(__Field::__field4),
                        b"minBorrowTerms" => _serde::export::Ok(__Field::__field5),
                        b"minBorrowInterestRate" => _serde::export::Ok(__Field::__field6),
                        _ => {
                            let __value = &_serde::export::from_utf8_lossy(__value);
                            _serde::export::Err(_serde::de::Error::unknown_field(__value, FIELDS))
                        }
                    }
                }
            }
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(__deserializer: __D) -> _serde::export::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                }
            }
            struct __Visitor < 'de , T : Trait > where T :: AccountId : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned , T :: AccountId : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned , Vec < TradingPair < T :: AssetId > > : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned , u32 : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned , u32 : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned , u64 : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned , u64 : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned { marker : _serde :: export :: PhantomData < GenesisConfig < T > > , lifetime : _serde :: export :: PhantomData < & 'de ( ) > , }
            impl < 'de , T : Trait > _serde :: de :: Visitor < 'de > for __Visitor < 'de , T > where T :: AccountId : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned , T :: AccountId : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned , Vec < TradingPair < T :: AssetId > > : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned , u32 : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned , u32 : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned , u64 : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned , u64 : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned { type Value = GenesisConfig < T > ; fn expecting ( & self , __formatter : & mut _serde :: export :: Formatter ) -> _serde :: export :: fmt :: Result { _serde :: export :: Formatter :: write_str ( __formatter , "struct GenesisConfig" ) } # [ inline ] fn visit_seq < __A > ( self , mut __seq : __A ) -> _serde :: export :: Result < Self :: Value , __A :: Error > where __A : _serde :: de :: SeqAccess < 'de > { let __field0 = match match _serde :: de :: SeqAccess :: next_element :: < T :: AccountId > ( & mut __seq ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } { _serde :: export :: Some ( __value ) => __value , _serde :: export :: None => { return _serde :: export :: Err ( _serde :: de :: Error :: invalid_length ( 0usize , & "struct GenesisConfig with 7 elements" ) ) ; } } ; let __field1 = match match _serde :: de :: SeqAccess :: next_element :: < T :: AccountId > ( & mut __seq ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } { _serde :: export :: Some ( __value ) => __value , _serde :: export :: None => { return _serde :: export :: Err ( _serde :: de :: Error :: invalid_length ( 1usize , & "struct GenesisConfig with 7 elements" ) ) ; } } ; let __field2 = match match _serde :: de :: SeqAccess :: next_element :: < Vec < TradingPair < T :: AssetId > > > ( & mut __seq ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } { _serde :: export :: Some ( __value ) => __value , _serde :: export :: None => { return _serde :: export :: Err ( _serde :: de :: Error :: invalid_length ( 2usize , & "struct GenesisConfig with 7 elements" ) ) ; } } ; let __field3 = match match _serde :: de :: SeqAccess :: next_element :: < u32 > ( & mut __seq ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } { _serde :: export :: Some ( __value ) => __value , _serde :: export :: None => { return _serde :: export :: Err ( _serde :: de :: Error :: invalid_length ( 3usize , & "struct GenesisConfig with 7 elements" ) ) ; } } ; let __field4 = match match _serde :: de :: SeqAccess :: next_element :: < u32 > ( & mut __seq ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } { _serde :: export :: Some ( __value ) => __value , _serde :: export :: None => { return _serde :: export :: Err ( _serde :: de :: Error :: invalid_length ( 4usize , & "struct GenesisConfig with 7 elements" ) ) ; } } ; let __field5 = match match _serde :: de :: SeqAccess :: next_element :: < u64 > ( & mut __seq ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } { _serde :: export :: Some ( __value ) => __value , _serde :: export :: None => { return _serde :: export :: Err ( _serde :: de :: Error :: invalid_length ( 5usize , & "struct GenesisConfig with 7 elements" ) ) ; } } ; let __field6 = match match _serde :: de :: SeqAccess :: next_element :: < u64 > ( & mut __seq ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } { _serde :: export :: Some ( __value ) => __value , _serde :: export :: None => { return _serde :: export :: Err ( _serde :: de :: Error :: invalid_length ( 6usize , & "struct GenesisConfig with 7 elements" ) ) ; } } ; _serde :: export :: Ok ( GenesisConfig { money_pool : __field0 , platform : __field1 , trading_pairs : __field2 , safe_ltv : __field3 , liquidate_ltv : __field4 , min_borrow_terms : __field5 , min_borrow_interest_rate : __field6 , } ) } # [ inline ] fn visit_map < __A > ( self , mut __map : __A ) -> _serde :: export :: Result < Self :: Value , __A :: Error > where __A : _serde :: de :: MapAccess < 'de > { let mut __field0 : _serde :: export :: Option < T :: AccountId > = _serde :: export :: None ; let mut __field1 : _serde :: export :: Option < T :: AccountId > = _serde :: export :: None ; let mut __field2 : _serde :: export :: Option < Vec < TradingPair < T :: AssetId > > > = _serde :: export :: None ; let mut __field3 : _serde :: export :: Option < u32 > = _serde :: export :: None ; let mut __field4 : _serde :: export :: Option < u32 > = _serde :: export :: None ; let mut __field5 : _serde :: export :: Option < u64 > = _serde :: export :: None ; let mut __field6 : _serde :: export :: Option < u64 > = _serde :: export :: None ; while let _serde :: export :: Some ( __key ) = match _serde :: de :: MapAccess :: next_key :: < __Field > ( & mut __map ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } { match __key { __Field :: __field0 => { if _serde :: export :: Option :: is_some ( & __field0 ) { return _serde :: export :: Err ( < __A :: Error as _serde :: de :: Error > :: duplicate_field ( "moneyPool" ) ) ; } __field0 = _serde :: export :: Some ( match _serde :: de :: MapAccess :: next_value :: < T :: AccountId > ( & mut __map ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } ) ; } __Field :: __field1 => { if _serde :: export :: Option :: is_some ( & __field1 ) { return _serde :: export :: Err ( < __A :: Error as _serde :: de :: Error > :: duplicate_field ( "platform" ) ) ; } __field1 = _serde :: export :: Some ( match _serde :: de :: MapAccess :: next_value :: < T :: AccountId > ( & mut __map ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } ) ; } __Field :: __field2 => { if _serde :: export :: Option :: is_some ( & __field2 ) { return _serde :: export :: Err ( < __A :: Error as _serde :: de :: Error > :: duplicate_field ( "tradingPairs" ) ) ; } __field2 = _serde :: export :: Some ( match _serde :: de :: MapAccess :: next_value :: < Vec < TradingPair < T :: AssetId > > > ( & mut __map ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } ) ; } __Field :: __field3 => { if _serde :: export :: Option :: is_some ( & __field3 ) { return _serde :: export :: Err ( < __A :: Error as _serde :: de :: Error > :: duplicate_field ( "safeLtv" ) ) ; } __field3 = _serde :: export :: Some ( match _serde :: de :: MapAccess :: next_value :: < u32 > ( & mut __map ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } ) ; } __Field :: __field4 => { if _serde :: export :: Option :: is_some ( & __field4 ) { return _serde :: export :: Err ( < __A :: Error as _serde :: de :: Error > :: duplicate_field ( "liquidateLtv" ) ) ; } __field4 = _serde :: export :: Some ( match _serde :: de :: MapAccess :: next_value :: < u32 > ( & mut __map ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } ) ; } __Field :: __field5 => { if _serde :: export :: Option :: is_some ( & __field5 ) { return _serde :: export :: Err ( < __A :: Error as _serde :: de :: Error > :: duplicate_field ( "minBorrowTerms" ) ) ; } __field5 = _serde :: export :: Some ( match _serde :: de :: MapAccess :: next_value :: < u64 > ( & mut __map ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } ) ; } __Field :: __field6 => { if _serde :: export :: Option :: is_some ( & __field6 ) { return _serde :: export :: Err ( < __A :: Error as _serde :: de :: Error > :: duplicate_field ( "minBorrowInterestRate" ) ) ; } __field6 = _serde :: export :: Some ( match _serde :: de :: MapAccess :: next_value :: < u64 > ( & mut __map ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } ) ; } } } let __field0 = match __field0 { _serde :: export :: Some ( __field0 ) => __field0 , _serde :: export :: None => match _serde :: private :: de :: missing_field ( "moneyPool" ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } , } ; let __field1 = match __field1 { _serde :: export :: Some ( __field1 ) => __field1 , _serde :: export :: None => match _serde :: private :: de :: missing_field ( "platform" ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } , } ; let __field2 = match __field2 { _serde :: export :: Some ( __field2 ) => __field2 , _serde :: export :: None => match _serde :: private :: de :: missing_field ( "tradingPairs" ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } , } ; let __field3 = match __field3 { _serde :: export :: Some ( __field3 ) => __field3 , _serde :: export :: None => match _serde :: private :: de :: missing_field ( "safeLtv" ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } , } ; let __field4 = match __field4 { _serde :: export :: Some ( __field4 ) => __field4 , _serde :: export :: None => match _serde :: private :: de :: missing_field ( "liquidateLtv" ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } , } ; let __field5 = match __field5 { _serde :: export :: Some ( __field5 ) => __field5 , _serde :: export :: None => match _serde :: private :: de :: missing_field ( "minBorrowTerms" ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } , } ; let __field6 = match __field6 { _serde :: export :: Some ( __field6 ) => __field6 , _serde :: export :: None => match _serde :: private :: de :: missing_field ( "minBorrowInterestRate" ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } , } ; _serde :: export :: Ok ( GenesisConfig { money_pool : __field0 , platform : __field1 , trading_pairs : __field2 , safe_ltv : __field3 , liquidate_ltv : __field4 , min_borrow_terms : __field5 , min_borrow_interest_rate : __field6 , } ) } }
            const FIELDS: &'static [&'static str] = &[
                "moneyPool",
                "platform",
                "tradingPairs",
                "safeLtv",
                "liquidateLtv",
                "minBorrowTerms",
                "minBorrowInterestRate",
            ];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "GenesisConfig",
                FIELDS,
                __Visitor {
                    marker: _serde::export::PhantomData::<GenesisConfig<T>>,
                    lifetime: _serde::export::PhantomData,
                },
            )
        }
    }
};
#[cfg(feature = "std")]
impl<T: Trait> Default for GenesisConfig<T> {
    fn default() -> Self {
        GenesisConfig {
            money_pool: Default::default(),
            platform: Default::default(),
            trading_pairs: Default::default(),
            safe_ltv: Default::default(),
            liquidate_ltv: Default::default(),
            min_borrow_terms: Default::default(),
            min_borrow_interest_rate: Default::default(),
        }
    }
}
#[cfg(feature = "std")]
impl<T: Trait> GenesisConfig<T> {
    pub fn build_storage(
        &self,
    ) -> std::result::Result<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_runtime::Storage,
        String,
    > {
        let mut storage = Default::default();
        self.assimilate_storage(&mut storage)?;
        Ok(storage)
    }
    /// Assimilate the storage for this module into pre-existing overlays.
    pub fn assimilate_storage(
        &self,
        storage : & mut self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_runtime :: Storage,
    ) -> std::result::Result<(), String> {
        self :: sp_api_hidden_includes_decl_storage :: hidden_include :: BasicExternalities :: execute_with_storage ( storage , | | { { let data = & self . money_pool ; let v : & T :: AccountId = data ; < MoneyPool < T > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < T :: AccountId > > :: put :: < & T :: AccountId > ( v ) ; } { let data = & self . platform ; let v : & T :: AccountId = data ; < Platform < T > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < T :: AccountId > > :: put :: < & T :: AccountId > ( v ) ; } { let data = & self . trading_pairs ; let v : & Vec < TradingPair < T :: AssetId > > = data ; < TradingPairs < T > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < Vec < TradingPair < T :: AssetId > > > > :: put :: < & Vec < TradingPair < T :: AssetId > > > ( v ) ; } { let data = & self . safe_ltv ; let v : & u32 = data ; < SafeLTV < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < u32 > > :: put :: < & u32 > ( v ) ; } { let data = & self . liquidate_ltv ; let v : & u32 = data ; < LiquidateLTV < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < u32 > > :: put :: < & u32 > ( v ) ; } { let data = & self . min_borrow_terms ; let v : & u64 = data ; < MinBorrowTerms < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < u64 > > :: put :: < & u64 > ( v ) ; } { let data = & self . min_borrow_interest_rate ; let v : & u64 = data ; < MinBorrowInterestRate < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageValue < u64 > > :: put :: < & u64 > ( v ) ; } Ok ( ( ) ) } )
    }
}
#[cfg(feature = "std")]
impl<T: Trait, __GeneratedInstance: __GeneratedInstantiable>
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_runtime::BuildModuleGenesisStorage<
        T,
        __GeneratedInstance,
    > for GenesisConfig<T>
{
    fn build_module_genesis_storage(
        &self,
        storage : & mut self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_runtime :: Storage,
    ) -> std::result::Result<(), String> {
        self.assimilate_storage(storage)
    }
}
/// module level switch
pub struct Paused(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<()>,
);
impl
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageValue<
        bool,
    > for Paused
{
    type Query = bool;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "Paused".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<bool>) -> Self::Query {
        v.unwrap_or_else(|| false)
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<bool> {
        Some(v)
    }
}
/// hold borrowers' collateral temporarily
pub struct MoneyPool<T: Trait>(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T,)>,
);
impl<T: Trait>
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageValue<
        T::AccountId,
    > for MoneyPool<T>
{
    type Query = T::AccountId;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "MoneyPool".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<T::AccountId>) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<T::AccountId> {
        Some(v)
    }
}
/// Platform is just a account receiving potential fees
pub struct Platform<T: Trait>(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T,)>,
);
impl<T: Trait>
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageValue<
        T::AccountId,
    > for Platform<T>
{
    type Query = T::AccountId;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "Platform".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<T::AccountId>) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<T::AccountId> {
        Some(v)
    }
}
/// TradingPairs contains all supported trading pairs, oracle should provide price information for all trading pairs.
pub struct TradingPairs<T: Trait>(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T,)>,
);
impl<T: Trait>
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageValue<
        Vec<TradingPair<T::AssetId>>,
    > for TradingPairs<T>
{
    type Query = Vec<TradingPair<T::AssetId>>;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "TradingPairs".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<Vec<TradingPair<T::AssetId>>>) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<Vec<TradingPair<T::AssetId>>> {
        Some(v)
    }
}
/// LTV must be greater than this value to create a new borrow
pub struct SafeLTV(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<()>,
);
impl
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageValue<u32>
    for SafeLTV
{
    type Query = u32;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "SafeLTV".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<u32>) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<u32> {
        Some(v)
    }
}
/// a loan will be liquidated when LTV is below this
pub struct LiquidateLTV(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<()>,
);
impl
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageValue<u32>
    for LiquidateLTV
{
    type Query = u32;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "LiquidateLTV".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<u32>) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<u32> {
        Some(v)
    }
}
/// minimium borrow terms, count in natural days
pub struct MinBorrowTerms(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<()>,
);
impl
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageValue<u64>
    for MinBorrowTerms
{
    type Query = u64;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "MinBorrowTerms".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<u64>) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<u64> {
        Some(v)
    }
}
/// minimium interest rate
pub struct MinBorrowInterestRate(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<()>,
);
impl
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageValue<u64>
    for MinBorrowInterestRate
{
    type Query = u64;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "MinBorrowInterestRate".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<u64>) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<u64> {
        Some(v)
    }
}
/// borrow id counter
pub struct NextBorrowId(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<()>,
);
impl
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageValue<
        BorrowId,
    > for NextBorrowId
{
    type Query = BorrowId;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "NextBorrowId".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<BorrowId>) -> Self::Query {
        v.unwrap_or_else(|| 1)
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<BorrowId> {
        Some(v)
    }
}
/// loan id counter
pub struct NextLoanId(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<()>,
);
impl
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageValue<
        LoanId,
    > for NextLoanId
{
    type Query = LoanId;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "NextLoanId".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<LoanId>) -> Self::Query {
        v.unwrap_or_else(|| 1)
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<LoanId> {
        Some(v)
    }
}
/// an account can only have one alive borrow at a time
pub struct Borrows<T: Trait>(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T,)>,
);
impl<T: Trait>
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageLinkedMap<
        BorrowId,
        Borrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>,
    > for Borrows<T>
{
    type Query = Borrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>;
    type KeyFormat = Self;
    fn from_optional_value_to_query(
        v: Option<Borrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>>,
    ) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(
        v: Self::Query,
    ) -> Option<Borrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>> {
        Some(v)
    }
}
impl < T : Trait > self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: generator :: LinkedMapKeyFormat for Borrows < T > { type Hasher = self :: sp_api_hidden_includes_decl_storage :: hidden_include :: Blake2_256 ; fn module_prefix ( ) -> & 'static [ u8 ] { __InherentHiddenInstance :: PREFIX . as_bytes ( ) } fn storage_prefix ( ) -> & 'static [ u8 ] { "Borrows" . as_bytes ( ) } fn head_prefix ( ) -> & 'static [ u8 ] { "HeadOfBorrows" . as_bytes ( ) } }
pub struct BorrowIdsByAccountId<T: Trait>(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T,)>,
);
impl<T: Trait>
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::StoragePrefixedMap<
        Vec<BorrowId>,
    > for BorrowIdsByAccountId<T>
{
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "BorrowIdsByAccountId".as_bytes()
    }
}
impl<T: Trait>
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageMap<
        T::AccountId,
        Vec<BorrowId>,
    > for BorrowIdsByAccountId<T>
{
    type Query = Vec<BorrowId>;
    type Hasher = self::sp_api_hidden_includes_decl_storage::hidden_include::Blake2_256;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "BorrowIdsByAccountId".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<Vec<BorrowId>>) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<Vec<BorrowId>> {
        Some(v)
    }
}
pub struct AliveBorrowIds(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<()>,
);
impl
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageValue<
        Vec<BorrowId>,
    > for AliveBorrowIds
{
    type Query = Vec<BorrowId>;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "AliveBorrowIds".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<Vec<BorrowId>>) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<Vec<BorrowId>> {
        Some(v)
    }
}
/// on the other hand, an account can have multiple alive loans
pub struct Loans<T: Trait>(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T,)>,
);
impl<T: Trait>
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageLinkedMap<
        LoanId,
        Loan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>,
    > for Loans<T>
{
    type Query = Loan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>;
    type KeyFormat = Self;
    fn from_optional_value_to_query(
        v: Option<Loan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>>,
    ) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(
        v: Self::Query,
    ) -> Option<Loan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>> {
        Some(v)
    }
}
impl < T : Trait > self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: generator :: LinkedMapKeyFormat for Loans < T > { type Hasher = self :: sp_api_hidden_includes_decl_storage :: hidden_include :: Blake2_256 ; fn module_prefix ( ) -> & 'static [ u8 ] { __InherentHiddenInstance :: PREFIX . as_bytes ( ) } fn storage_prefix ( ) -> & 'static [ u8 ] { "Loans" . as_bytes ( ) } fn head_prefix ( ) -> & 'static [ u8 ] { "HeadOfLoans" . as_bytes ( ) } }
pub struct LoanIdsByAccountId<T: Trait>(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T,)>,
);
impl<T: Trait>
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::StoragePrefixedMap<
        Vec<LoanId>,
    > for LoanIdsByAccountId<T>
{
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "LoanIdsByAccountId".as_bytes()
    }
}
impl<T: Trait>
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageMap<
        T::AccountId,
        Vec<LoanId>,
    > for LoanIdsByAccountId<T>
{
    type Query = Vec<LoanId>;
    type Hasher = self::sp_api_hidden_includes_decl_storage::hidden_include::Blake2_256;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "LoanIdsByAccountId".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<Vec<LoanId>>) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<Vec<LoanId>> {
        Some(v)
    }
}
pub struct AliveLoanIdsByAccountId<T: Trait>(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T,)>,
);
impl<T: Trait>
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::StoragePrefixedMap<
        Vec<LoanId>,
    > for AliveLoanIdsByAccountId<T>
{
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "AliveLoanIdsByAccountId".as_bytes()
    }
}
impl<T: Trait>
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageMap<
        T::AccountId,
        Vec<LoanId>,
    > for AliveLoanIdsByAccountId<T>
{
    type Query = Vec<LoanId>;
    type Hasher = self::sp_api_hidden_includes_decl_storage::hidden_include::Blake2_256;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "AliveLoanIdsByAccountId".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<Vec<LoanId>>) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<Vec<LoanId>> {
        Some(v)
    }
}
pub struct AccountIdsWithLiveLoans<T: Trait>(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T,)>,
);
impl<T: Trait>
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageValue<
        Vec<T::AccountId>,
    > for AccountIdsWithLiveLoans<T>
{
    type Query = Vec<T::AccountId>;
    fn module_prefix() -> &'static [u8] {
        __InherentHiddenInstance::PREFIX.as_bytes()
    }
    fn storage_prefix() -> &'static [u8] {
        "AccountIdsWithLiveLoans".as_bytes()
    }
    fn from_optional_value_to_query(v: Option<Vec<T::AccountId>>) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<Vec<T::AccountId>> {
        Some(v)
    }
}
pub enum Error<T: Trait> {
    #[doc(hidden)]
    __Ignore(
        ::frame_support::sp_std::marker::PhantomData<(T,)>,
        ::frame_support::dispatch::Never,
    ),
    Paused,
    MinBorrowTerms,
    MinBorrowInterestRate,
    CanNotReserve,
    MultipleAliveBorrows,
    BorrowNotAlive,
    TradingPairNotAllowed,
    NotOwnerOfBorrow,
    UnknownBorrowId,
    UnknownLoanId,
    NoLockedBalance,
    InitialCollateralRateFail,
    NotEnoughBalance,
    TradingPairPriceMissing,
    BorrowNotLoaned,
    LTVNotMeet,
    ShouldNotBeLiquidated,
    ShouldBeLiquidated,
    LoanNotWell,
}
impl<T: Trait> ::frame_support::sp_std::fmt::Debug for Error<T> {
    fn fmt(
        &self,
        f: &mut ::frame_support::sp_std::fmt::Formatter<'_>,
    ) -> ::frame_support::sp_std::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl<T: Trait> Error<T> {
    fn as_u8(&self) -> u8 {
        match self {
            Error::__Ignore(_, _) => ::std::rt::begin_panic_fmt(&::core::fmt::Arguments::new_v1(
                &["internal error: entered unreachable code: "],
                &match (&"`__Ignore` can never be constructed",) {
                    (arg0,) => [::core::fmt::ArgumentV1::new(
                        arg0,
                        ::core::fmt::Display::fmt,
                    )],
                },
            )),
            Error::Paused => 0,
            Error::MinBorrowTerms => 0 + 1,
            Error::MinBorrowInterestRate => 0 + 1 + 1,
            Error::CanNotReserve => 0 + 1 + 1 + 1,
            Error::MultipleAliveBorrows => 0 + 1 + 1 + 1 + 1,
            Error::BorrowNotAlive => 0 + 1 + 1 + 1 + 1 + 1,
            Error::TradingPairNotAllowed => 0 + 1 + 1 + 1 + 1 + 1 + 1,
            Error::NotOwnerOfBorrow => 0 + 1 + 1 + 1 + 1 + 1 + 1 + 1,
            Error::UnknownBorrowId => 0 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1,
            Error::UnknownLoanId => 0 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1,
            Error::NoLockedBalance => 0 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1,
            Error::InitialCollateralRateFail => 0 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1,
            Error::NotEnoughBalance => 0 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1,
            Error::TradingPairPriceMissing => 0 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1,
            Error::BorrowNotLoaned => 0 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1,
            Error::LTVNotMeet => 0 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1,
            Error::ShouldNotBeLiquidated => {
                0 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1
            }
            Error::ShouldBeLiquidated => {
                0 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1
            }
            Error::LoanNotWell => {
                0 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1
            }
        }
    }
    fn as_str(&self) -> &'static str {
        match self {
            Self::__Ignore(_, _) => ::std::rt::begin_panic_fmt(&::core::fmt::Arguments::new_v1(
                &["internal error: entered unreachable code: "],
                &match (&"`__Ignore` can never be constructed",) {
                    (arg0,) => [::core::fmt::ArgumentV1::new(
                        arg0,
                        ::core::fmt::Display::fmt,
                    )],
                },
            )),
            Error::Paused => "Paused",
            Error::MinBorrowTerms => "MinBorrowTerms",
            Error::MinBorrowInterestRate => "MinBorrowInterestRate",
            Error::CanNotReserve => "CanNotReserve",
            Error::MultipleAliveBorrows => "MultipleAliveBorrows",
            Error::BorrowNotAlive => "BorrowNotAlive",
            Error::TradingPairNotAllowed => "TradingPairNotAllowed",
            Error::NotOwnerOfBorrow => "NotOwnerOfBorrow",
            Error::UnknownBorrowId => "UnknownBorrowId",
            Error::UnknownLoanId => "UnknownLoanId",
            Error::NoLockedBalance => "NoLockedBalance",
            Error::InitialCollateralRateFail => "InitialCollateralRateFail",
            Error::NotEnoughBalance => "NotEnoughBalance",
            Error::TradingPairPriceMissing => "TradingPairPriceMissing",
            Error::BorrowNotLoaned => "BorrowNotLoaned",
            Error::LTVNotMeet => "LTVNotMeet",
            Error::ShouldNotBeLiquidated => "ShouldNotBeLiquidated",
            Error::ShouldBeLiquidated => "ShouldBeLiquidated",
            Error::LoanNotWell => "LoanNotWell",
        }
    }
}
impl<T: Trait> From<Error<T>> for &'static str {
    fn from(err: Error<T>) -> &'static str {
        err.as_str()
    }
}
impl<T: Trait> From<Error<T>> for ::frame_support::sp_runtime::DispatchError {
    fn from(err: Error<T>) -> Self {
        let index = <T::ModuleToIndex as ::frame_support::traits::ModuleToIndex>::module_to_index::<
            Module<T>,
        >()
        .expect("Every active module has an index in the runtime; qed") as u8;
        ::frame_support::sp_runtime::DispatchError::Module {
            index,
            error: err.as_u8(),
            message: Some(err.as_str()),
        }
    }
}
impl<T: Trait> ::frame_support::error::ModuleErrorMetadata for Error<T> {
    fn metadata() -> &'static [::frame_support::error::ErrorMetadata] {
        &[
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("Paused"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("MinBorrowTerms"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("MinBorrowInterestRate"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("CanNotReserve"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("MultipleAliveBorrows"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("BorrowNotAlive"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("TradingPairNotAllowed"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("NotOwnerOfBorrow"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("UnknownBorrowId"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("UnknownLoanId"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("NoLockedBalance"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("InitialCollateralRateFail"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("NotEnoughBalance"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("TradingPairPriceMissing"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("BorrowNotLoaned"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("LTVNotMeet"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("ShouldNotBeLiquidated"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("ShouldBeLiquidated"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("LoanNotWell"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[]),
            },
        ]
    }
}
/// The module declaration.
pub struct Module<T: Trait>(::frame_support::sp_std::marker::PhantomData<(T,)>);
#[automatically_derived]
#[allow(unused_qualifications)]
impl<T: ::core::clone::Clone + Trait> ::core::clone::Clone for Module<T> {
    #[inline]
    fn clone(&self) -> Module<T> {
        match *self {
            Module(ref __self_0_0) => Module(::core::clone::Clone::clone(&(*__self_0_0))),
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<T: ::core::marker::Copy + Trait> ::core::marker::Copy for Module<T> {}
impl<T: Trait> ::core::marker::StructuralPartialEq for Module<T> {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<T: ::core::cmp::PartialEq + Trait> ::core::cmp::PartialEq for Module<T> {
    #[inline]
    fn eq(&self, other: &Module<T>) -> bool {
        match *other {
            Module(ref __self_1_0) => match *self {
                Module(ref __self_0_0) => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Module<T>) -> bool {
        match *other {
            Module(ref __self_1_0) => match *self {
                Module(ref __self_0_0) => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
impl<T: Trait> ::core::marker::StructuralEq for Module<T> {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<T: ::core::cmp::Eq + Trait> ::core::cmp::Eq for Module<T> {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<
                ::frame_support::sp_std::marker::PhantomData<(T,)>,
            >;
        }
    }
}
impl<T: Trait> core::fmt::Debug for Module<T>
where
    T: core::fmt::Debug,
{
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        fmt.debug_tuple("Module").field(&self.0).finish()
    }
}
impl<T: Trait> ::frame_support::sp_runtime::traits::OnInitialize<T::BlockNumber> for Module<T> {
    fn on_initialize(_height: T::BlockNumber) {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "on_initialize",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {}
    }
}
impl<T: Trait> ::frame_support::sp_runtime::traits::OnFinalize<T::BlockNumber> for Module<T> {
    fn on_finalize(block_number: T::BlockNumber) {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "on_finalize",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            if (block_number % 2.into()).is_zero()
                && !((block_number + 1.into()) % 5.into()).is_zero()
            {
                Self::periodic_check_borrows(block_number);
            }
            if ((block_number + 1.into()) % 5.into()).is_zero() {
                Self::periodic_check_loans(block_number);
            }
        }
    }
}
impl<T: Trait> ::frame_support::dispatch::WeighBlock<T::BlockNumber> for Module<T> {
    fn on_initialize(n: T::BlockNumber) -> ::frame_support::dispatch::Weight {
        <dyn ::frame_support::dispatch::WeighData<T::BlockNumber>>::weigh_data(
            &::frame_support::dispatch::SimpleDispatchInfo::zero(),
            n,
        )
    }
    fn on_finalize(n: T::BlockNumber) -> ::frame_support::dispatch::Weight {
        <dyn ::frame_support::dispatch::WeighData<T::BlockNumber>>::weigh_data(
            &::frame_support::dispatch::SimpleDispatchInfo::zero(),
            n,
        )
    }
}
impl<T: Trait> ::frame_support::sp_runtime::traits::OffchainWorker<T::BlockNumber> for Module<T> {}
impl<T: Trait> Module<T> {
    fn deposit_event(event: impl Into<<T as Trait>::Event>) {
        <system::Module<T>>::deposit_event(event.into())
    }
}
/// Can also be called using [`Call`].
///
/// [`Call`]: enum.Call.html
impl<T: Trait> Module<T> {
    pub fn pause(origin: T::Origin) -> DispatchResult {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "pause",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            ensure_root(origin)?;
            Paused::mutate(|v| *v = true);
            Ok(())
        }
    }
    pub fn resume(origin: T::Origin) -> DispatchResult {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "resume",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            ensure_root(origin)?;
            Paused::mutate(|v| *v = false);
            Ok(())
        }
    }
    pub fn change_platform(origin: T::Origin, platform: T::AccountId) -> DispatchResult {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "change_platform",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            ensure_root(origin)?;
            <Platform<T>>::put(platform);
            Ok(())
        }
    }
    pub fn change_money_pool(origin: T::Origin, pool: T::AccountId) -> DispatchResult {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "change_money_pool",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            ensure_root(origin)?;
            <MoneyPool<T>>::put(pool);
            Ok(())
        }
    }
    pub fn change_safe_ltv(origin: T::Origin, ltv: u32) -> DispatchResult {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "change_safe_ltv",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            ensure_root(origin)?;
            SafeLTV::put(ltv);
            Ok(())
        }
    }
    pub fn change_liquidate_ltv(origin: T::Origin, ltv: u32) -> DispatchResult {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "change_liquidate_ltv",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            ensure_root(origin)?;
            LiquidateLTV::put(ltv);
            Ok(())
        }
    }
    pub fn change_min_borrow_terms(origin: T::Origin, t: u64) -> DispatchResult {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "change_min_borrow_terms",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            ensure_root(origin)?;
            MinBorrowTerms::put(t);
            Ok(())
        }
    }
    pub fn change_min_borrow_interest_rate(origin: T::Origin, r: u64) -> DispatchResult {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "change_min_borrow_interest_rate",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            ensure_root(origin)?;
            MinBorrowInterestRate::put(r);
            Ok(())
        }
    }
    pub fn list_borrow(
        origin: T::Origin,
        collateral_balance: T::Balance,
        trading_pair: TradingPair<T::AssetId>,
        borrow_options: BorrowOptions<T::Balance, T::BlockNumber>,
    ) -> DispatchResult {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "list_borrow",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            {
                if !!Self::paused() {
                    {
                        return Err(Error::<T>::Paused.into());
                    };
                }
            };
            let who = ensure_signed(origin)?;
            Self::create_borrow(who, collateral_balance, trading_pair, borrow_options)
        }
    }
    pub fn unlist_borrow(origin: T::Origin, borrow_id: BorrowId) -> DispatchResult {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "unlist_borrow",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            {
                if !!Self::paused() {
                    {
                        return Err(Error::<T>::Paused.into());
                    };
                }
            };
            let who = ensure_signed(origin)?;
            Self::remove_borrow(who, borrow_id)
        }
    }
    pub fn lend(origin: T::Origin, borrow_id: BorrowId) -> DispatchResult {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "lend",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            {
                if !!Self::paused() {
                    {
                        return Err(Error::<T>::Paused.into());
                    };
                }
            };
            let who = ensure_signed(origin)?;
            Self::create_loan(who, borrow_id)
        }
    }
    pub fn liquidate(origin: T::Origin, loan_id: LoanId) -> DispatchResult {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "liquidate",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            {
                if !!Self::paused() {
                    {
                        return Err(Error::<T>::Paused.into());
                    };
                }
            };
            let who = ensure_signed(origin)?;
            Self::liquidate_loan(who, loan_id)
        }
    }
    pub fn add(origin: T::Origin, borrow_id: BorrowId, amount: T::Balance) -> DispatchResult {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "add",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            {
                if !!Self::paused() {
                    {
                        return Err(Error::<T>::Paused.into());
                    };
                }
            };
            let who = ensure_signed(origin)?;
            Self::add_collateral(who, borrow_id, amount)
        }
    }
    pub fn repay(origin: T::Origin, borrow_id: BorrowId) -> DispatchResult {
        use ::frame_support::sp_std::if_std;
        use ::frame_support::tracing;
        let span = {
            if ::tracing::dispatcher::has_been_set()
                && tracing::Level::DEBUG <= ::tracing::level_filters::STATIC_MAX_LEVEL
            {
                use ::tracing::callsite;
                use ::tracing::callsite::Callsite;
                let callsite = {
                    use ::tracing::{callsite, subscriber::Interest, Metadata, __macro_support::*};
                    struct MyCallsite;
                    static META: Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "repay",
                            "ls_biding",
                            tracing::Level::DEBUG,
                            Some("runtime/modules/ls-biding/src/lib.rs"),
                            Some(223u32),
                            Some("ls_biding"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&MyCallsite),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    static INTEREST: AtomicUsize = AtomicUsize::new(0);
                    static REGISTRATION: Once = Once::new();
                    impl MyCallsite {
                        #[inline]
                        fn interest(&self) -> Interest {
                            match INTEREST.load(Ordering::Relaxed) {
                                0 => Interest::never(),
                                2 => Interest::always(),
                                _ => Interest::sometimes(),
                            }
                        }
                    }
                    impl callsite::Callsite for MyCallsite {
                        fn set_interest(&self, interest: Interest) {
                            let interest = match () {
                                _ if interest.is_never() => 0,
                                _ if interest.is_always() => 2,
                                _ => 1,
                            };
                            INTEREST.store(interest, Ordering::SeqCst);
                        }
                        fn metadata(&self) -> &Metadata {
                            &META
                        }
                    }
                    REGISTRATION.call_once(|| {
                        callsite::register(&MyCallsite);
                    });
                    &MyCallsite
                };
                let meta = callsite.metadata();
                if {
                    let interest = callsite.interest();
                    if interest.is_never() {
                        false
                    } else if interest.is_always() {
                        true
                    } else {
                        let meta = callsite.metadata();
                        ::tracing::dispatcher::get_default(|current| current.enabled(meta))
                    }
                } {
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    ::tracing::Span::none()
                }
            } else {
                ::tracing::Span::none()
            }
        };
        let _enter = span.enter();
        {
            {
                if !!Self::paused() {
                    {
                        return Err(Error::<T>::Paused.into());
                    };
                }
            };
            let who = ensure_signed(origin)?;
            Self::repay_loan(who, borrow_id)
        }
    }
}
/// Dispatchable calls.
///
/// Each variant of this enum maps to a dispatchable function from the associated module.
pub enum Call<T: Trait> {
    #[doc(hidden)]
    #[codec(skip)]
    __PhantomItem(
        ::frame_support::sp_std::marker::PhantomData<(T,)>,
        ::frame_support::dispatch::Never,
    ),
    #[allow(non_camel_case_types)]
    pause(),
    #[allow(non_camel_case_types)]
    resume(),
    #[allow(non_camel_case_types)]
    change_platform(T::AccountId),
    #[allow(non_camel_case_types)]
    change_money_pool(T::AccountId),
    #[allow(non_camel_case_types)]
    change_safe_ltv(u32),
    #[allow(non_camel_case_types)]
    change_liquidate_ltv(u32),
    #[allow(non_camel_case_types)]
    change_min_borrow_terms(u64),
    #[allow(non_camel_case_types)]
    change_min_borrow_interest_rate(u64),
    #[allow(non_camel_case_types)]
    list_borrow(
        T::Balance,
        TradingPair<T::AssetId>,
        BorrowOptions<T::Balance, T::BlockNumber>,
    ),
    #[allow(non_camel_case_types)]
    unlist_borrow(BorrowId),
    #[allow(non_camel_case_types)]
    lend(BorrowId),
    #[allow(non_camel_case_types)]
    liquidate(LoanId),
    #[allow(non_camel_case_types)]
    add(BorrowId, T::Balance),
    #[allow(non_camel_case_types)]
    repay(BorrowId),
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<T: Trait> _parity_scale_codec::Encode for Call<T>
    where
        T::AccountId: _parity_scale_codec::Encode,
        T::AccountId: _parity_scale_codec::Encode,
        T::AccountId: _parity_scale_codec::Encode,
        T::AccountId: _parity_scale_codec::Encode,
        T::Balance: _parity_scale_codec::Encode,
        T::Balance: _parity_scale_codec::Encode,
        TradingPair<T::AssetId>: _parity_scale_codec::Encode,
        TradingPair<T::AssetId>: _parity_scale_codec::Encode,
        BorrowOptions<T::Balance, T::BlockNumber>: _parity_scale_codec::Encode,
        BorrowOptions<T::Balance, T::BlockNumber>: _parity_scale_codec::Encode,
        T::Balance: _parity_scale_codec::Encode,
        T::Balance: _parity_scale_codec::Encode,
    {
        fn encode_to<EncOut: _parity_scale_codec::Output>(&self, dest: &mut EncOut) {
            match *self {
                Call::pause() => {
                    dest.push_byte(0usize as u8);
                }
                Call::resume() => {
                    dest.push_byte(1usize as u8);
                }
                Call::change_platform(ref aa) => {
                    dest.push_byte(2usize as u8);
                    dest.push(aa);
                }
                Call::change_money_pool(ref aa) => {
                    dest.push_byte(3usize as u8);
                    dest.push(aa);
                }
                Call::change_safe_ltv(ref aa) => {
                    dest.push_byte(4usize as u8);
                    dest.push(aa);
                }
                Call::change_liquidate_ltv(ref aa) => {
                    dest.push_byte(5usize as u8);
                    dest.push(aa);
                }
                Call::change_min_borrow_terms(ref aa) => {
                    dest.push_byte(6usize as u8);
                    dest.push(aa);
                }
                Call::change_min_borrow_interest_rate(ref aa) => {
                    dest.push_byte(7usize as u8);
                    dest.push(aa);
                }
                Call::list_borrow(ref aa, ref ba, ref ca) => {
                    dest.push_byte(8usize as u8);
                    dest.push(aa);
                    dest.push(ba);
                    dest.push(ca);
                }
                Call::unlist_borrow(ref aa) => {
                    dest.push_byte(9usize as u8);
                    dest.push(aa);
                }
                Call::lend(ref aa) => {
                    dest.push_byte(10usize as u8);
                    dest.push(aa);
                }
                Call::liquidate(ref aa) => {
                    dest.push_byte(11usize as u8);
                    dest.push(aa);
                }
                Call::add(ref aa, ref ba) => {
                    dest.push_byte(12usize as u8);
                    dest.push(aa);
                    dest.push(ba);
                }
                Call::repay(ref aa) => {
                    dest.push_byte(13usize as u8);
                    dest.push(aa);
                }
                _ => (),
            }
        }
    }
    impl<T: Trait> _parity_scale_codec::EncodeLike for Call<T>
    where
        T::AccountId: _parity_scale_codec::Encode,
        T::AccountId: _parity_scale_codec::Encode,
        T::AccountId: _parity_scale_codec::Encode,
        T::AccountId: _parity_scale_codec::Encode,
        T::Balance: _parity_scale_codec::Encode,
        T::Balance: _parity_scale_codec::Encode,
        TradingPair<T::AssetId>: _parity_scale_codec::Encode,
        TradingPair<T::AssetId>: _parity_scale_codec::Encode,
        BorrowOptions<T::Balance, T::BlockNumber>: _parity_scale_codec::Encode,
        BorrowOptions<T::Balance, T::BlockNumber>: _parity_scale_codec::Encode,
        T::Balance: _parity_scale_codec::Encode,
        T::Balance: _parity_scale_codec::Encode,
    {
    }
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<T: Trait> _parity_scale_codec::Decode for Call<T>
    where
        T::AccountId: _parity_scale_codec::Decode,
        T::AccountId: _parity_scale_codec::Decode,
        T::AccountId: _parity_scale_codec::Decode,
        T::AccountId: _parity_scale_codec::Decode,
        T::Balance: _parity_scale_codec::Decode,
        T::Balance: _parity_scale_codec::Decode,
        TradingPair<T::AssetId>: _parity_scale_codec::Decode,
        TradingPair<T::AssetId>: _parity_scale_codec::Decode,
        BorrowOptions<T::Balance, T::BlockNumber>: _parity_scale_codec::Decode,
        BorrowOptions<T::Balance, T::BlockNumber>: _parity_scale_codec::Decode,
        T::Balance: _parity_scale_codec::Decode,
        T::Balance: _parity_scale_codec::Decode,
    {
        fn decode<DecIn: _parity_scale_codec::Input>(
            input: &mut DecIn,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            match input.read_byte()? {
                x if x == 0usize as u8 => Ok(Call::pause()),
                x if x == 1usize as u8 => Ok(Call::resume()),
                x if x == 2usize as u8 => Ok(Call::change_platform({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field Call :: change_platform.0".into())
                        }
                        Ok(a) => a,
                    }
                })),
                x if x == 3usize as u8 => Ok(Call::change_money_pool({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field Call :: change_money_pool.0".into())
                        }
                        Ok(a) => a,
                    }
                })),
                x if x == 4usize as u8 => Ok(Call::change_safe_ltv({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field Call :: change_safe_ltv.0".into())
                        }
                        Ok(a) => a,
                    }
                })),
                x if x == 5usize as u8 => Ok(Call::change_liquidate_ltv({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field Call :: change_liquidate_ltv.0".into())
                        }
                        Ok(a) => a,
                    }
                })),
                x if x == 6usize as u8 => Ok(Call::change_min_borrow_terms({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err(
                                "Error decoding field Call :: change_min_borrow_terms.0".into()
                            )
                        }
                        Ok(a) => a,
                    }
                })),
                x if x == 7usize as u8 => {
                    Ok(Call::change_min_borrow_interest_rate({
                        let res = _parity_scale_codec::Decode::decode(input);
                        match res {
                            Err(_) => return Err(
                                "Error decoding field Call :: change_min_borrow_interest_rate.0"
                                    .into(),
                            ),
                            Ok(a) => a,
                        }
                    }))
                }
                x if x == 8usize as u8 => Ok(Call::list_borrow(
                    {
                        let res = _parity_scale_codec::Decode::decode(input);
                        match res {
                            Err(_) => {
                                return Err("Error decoding field Call :: list_borrow.0".into())
                            }
                            Ok(a) => a,
                        }
                    },
                    {
                        let res = _parity_scale_codec::Decode::decode(input);
                        match res {
                            Err(_) => {
                                return Err("Error decoding field Call :: list_borrow.1".into())
                            }
                            Ok(a) => a,
                        }
                    },
                    {
                        let res = _parity_scale_codec::Decode::decode(input);
                        match res {
                            Err(_) => {
                                return Err("Error decoding field Call :: list_borrow.2".into())
                            }
                            Ok(a) => a,
                        }
                    },
                )),
                x if x == 9usize as u8 => Ok(Call::unlist_borrow({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Call :: unlist_borrow.0".into()),
                        Ok(a) => a,
                    }
                })),
                x if x == 10usize as u8 => Ok(Call::lend({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Call :: lend.0".into()),
                        Ok(a) => a,
                    }
                })),
                x if x == 11usize as u8 => Ok(Call::liquidate({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Call :: liquidate.0".into()),
                        Ok(a) => a,
                    }
                })),
                x if x == 12usize as u8 => Ok(Call::add(
                    {
                        let res = _parity_scale_codec::Decode::decode(input);
                        match res {
                            Err(_) => return Err("Error decoding field Call :: add.0".into()),
                            Ok(a) => a,
                        }
                    },
                    {
                        let res = _parity_scale_codec::Decode::decode(input);
                        match res {
                            Err(_) => return Err("Error decoding field Call :: add.1".into()),
                            Ok(a) => a,
                        }
                    },
                )),
                x if x == 13usize as u8 => Ok(Call::repay({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => return Err("Error decoding field Call :: repay.0".into()),
                        Ok(a) => a,
                    }
                })),
                x => Err("No such variant in enum Call".into()),
            }
        }
    }
};
impl<T: Trait> ::frame_support::dispatch::GetDispatchInfo for Call<T> {
    fn get_dispatch_info(&self) -> ::frame_support::dispatch::DispatchInfo {
        match *self {
            Call::pause() => {
                let weight = <dyn ::frame_support::dispatch::WeighData<()>>::weigh_data(
                    &SimpleDispatchInfo::MaxOperational,
                    (),
                );
                let class =
                    <dyn ::frame_support::dispatch::ClassifyDispatch<()>>::classify_dispatch(
                        &SimpleDispatchInfo::MaxOperational,
                        (),
                    );
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<()>>::pays_fee(
                    &SimpleDispatchInfo::MaxOperational,
                    (),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::resume() => {
                let weight = <dyn ::frame_support::dispatch::WeighData<()>>::weigh_data(
                    &SimpleDispatchInfo::MaxOperational,
                    (),
                );
                let class =
                    <dyn ::frame_support::dispatch::ClassifyDispatch<()>>::classify_dispatch(
                        &SimpleDispatchInfo::MaxOperational,
                        (),
                    );
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<()>>::pays_fee(
                    &SimpleDispatchInfo::MaxOperational,
                    (),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::change_platform(ref platform) => {
                let weight =
                    <dyn ::frame_support::dispatch::WeighData<(&T::AccountId,)>>::weigh_data(
                        &SimpleDispatchInfo::MaxOperational,
                        (platform,),
                    );
                let class = < dyn :: frame_support :: dispatch :: ClassifyDispatch < ( & T :: AccountId , ) > > :: classify_dispatch ( & SimpleDispatchInfo :: MaxOperational , ( platform , ) ) ;
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<(&T::AccountId,)>>::pays_fee(
                    &SimpleDispatchInfo::MaxOperational,
                    (platform,),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::change_money_pool(ref pool) => {
                let weight =
                    <dyn ::frame_support::dispatch::WeighData<(&T::AccountId,)>>::weigh_data(
                        &SimpleDispatchInfo::MaxOperational,
                        (pool,),
                    );
                let class = < dyn :: frame_support :: dispatch :: ClassifyDispatch < ( & T :: AccountId , ) > > :: classify_dispatch ( & SimpleDispatchInfo :: MaxOperational , ( pool , ) ) ;
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<(&T::AccountId,)>>::pays_fee(
                    &SimpleDispatchInfo::MaxOperational,
                    (pool,),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::change_safe_ltv(ref ltv) => {
                let weight = <dyn ::frame_support::dispatch::WeighData<(&u32,)>>::weigh_data(
                    &SimpleDispatchInfo::MaxOperational,
                    (ltv,),
                );
                let class =
                    <dyn ::frame_support::dispatch::ClassifyDispatch<(&u32,)>>::classify_dispatch(
                        &SimpleDispatchInfo::MaxOperational,
                        (ltv,),
                    );
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<(&u32,)>>::pays_fee(
                    &SimpleDispatchInfo::MaxOperational,
                    (ltv,),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::change_liquidate_ltv(ref ltv) => {
                let weight = <dyn ::frame_support::dispatch::WeighData<(&u32,)>>::weigh_data(
                    &SimpleDispatchInfo::MaxOperational,
                    (ltv,),
                );
                let class =
                    <dyn ::frame_support::dispatch::ClassifyDispatch<(&u32,)>>::classify_dispatch(
                        &SimpleDispatchInfo::MaxOperational,
                        (ltv,),
                    );
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<(&u32,)>>::pays_fee(
                    &SimpleDispatchInfo::MaxOperational,
                    (ltv,),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::change_min_borrow_terms(ref t) => {
                let weight = <dyn ::frame_support::dispatch::WeighData<(&u64,)>>::weigh_data(
                    &SimpleDispatchInfo::MaxOperational,
                    (t,),
                );
                let class =
                    <dyn ::frame_support::dispatch::ClassifyDispatch<(&u64,)>>::classify_dispatch(
                        &SimpleDispatchInfo::MaxOperational,
                        (t,),
                    );
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<(&u64,)>>::pays_fee(
                    &SimpleDispatchInfo::MaxOperational,
                    (t,),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::change_min_borrow_interest_rate(ref r) => {
                let weight = <dyn ::frame_support::dispatch::WeighData<(&u64,)>>::weigh_data(
                    &SimpleDispatchInfo::MaxOperational,
                    (r,),
                );
                let class =
                    <dyn ::frame_support::dispatch::ClassifyDispatch<(&u64,)>>::classify_dispatch(
                        &SimpleDispatchInfo::MaxOperational,
                        (r,),
                    );
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<(&u64,)>>::pays_fee(
                    &SimpleDispatchInfo::MaxOperational,
                    (r,),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::list_borrow(ref collateral_balance, ref trading_pair, ref borrow_options) => {
                let weight = <dyn ::frame_support::dispatch::WeighData<(
                    &T::Balance,
                    &TradingPair<T::AssetId>,
                    &BorrowOptions<T::Balance, T::BlockNumber>,
                )>>::weigh_data(
                    &SimpleDispatchInfo::FixedNormal(1_000_000),
                    (collateral_balance, trading_pair, borrow_options),
                );
                let class = <dyn ::frame_support::dispatch::ClassifyDispatch<(
                    &T::Balance,
                    &TradingPair<T::AssetId>,
                    &BorrowOptions<T::Balance, T::BlockNumber>,
                )>>::classify_dispatch(
                    &SimpleDispatchInfo::FixedNormal(1_000_000),
                    (collateral_balance, trading_pair, borrow_options),
                );
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<(
                    &T::Balance,
                    &TradingPair<T::AssetId>,
                    &BorrowOptions<T::Balance, T::BlockNumber>,
                )>>::pays_fee(
                    &SimpleDispatchInfo::FixedNormal(1_000_000),
                    (collateral_balance, trading_pair, borrow_options),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::unlist_borrow(ref borrow_id) => {
                let weight = <dyn ::frame_support::dispatch::WeighData<(&BorrowId,)>>::weigh_data(
                    &SimpleDispatchInfo::FixedNormal(500_000),
                    (borrow_id,),
                );
                let class = < dyn :: frame_support :: dispatch :: ClassifyDispatch < ( & BorrowId , ) > > :: classify_dispatch ( & SimpleDispatchInfo :: FixedNormal ( 500_000 ) , ( borrow_id , ) ) ;
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<(&BorrowId,)>>::pays_fee(
                    &SimpleDispatchInfo::FixedNormal(500_000),
                    (borrow_id,),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::lend(ref borrow_id) => {
                let weight = <dyn ::frame_support::dispatch::WeighData<(&BorrowId,)>>::weigh_data(
                    &SimpleDispatchInfo::FixedNormal(1_000_000),
                    (borrow_id,),
                );
                let class = < dyn :: frame_support :: dispatch :: ClassifyDispatch < ( & BorrowId , ) > > :: classify_dispatch ( & SimpleDispatchInfo :: FixedNormal ( 1_000_000 ) , ( borrow_id , ) ) ;
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<(&BorrowId,)>>::pays_fee(
                    &SimpleDispatchInfo::FixedNormal(1_000_000),
                    (borrow_id,),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::liquidate(ref loan_id) => {
                let weight = <dyn ::frame_support::dispatch::WeighData<(&LoanId,)>>::weigh_data(
                    &SimpleDispatchInfo::FixedNormal(1_000_000),
                    (loan_id,),
                );
                let class = < dyn :: frame_support :: dispatch :: ClassifyDispatch < ( & LoanId , ) > > :: classify_dispatch ( & SimpleDispatchInfo :: FixedNormal ( 1_000_000 ) , ( loan_id , ) ) ;
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<(&LoanId,)>>::pays_fee(
                    &SimpleDispatchInfo::FixedNormal(1_000_000),
                    (loan_id,),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::add(ref borrow_id, ref amount) => {
                let weight = < dyn :: frame_support :: dispatch :: WeighData < ( & BorrowId , & T :: Balance ) > > :: weigh_data ( & SimpleDispatchInfo :: FixedNormal ( 500_000 ) , ( borrow_id , amount ) ) ;
                let class = <dyn ::frame_support::dispatch::ClassifyDispatch<(
                    &BorrowId,
                    &T::Balance,
                )>>::classify_dispatch(
                    &SimpleDispatchInfo::FixedNormal(500_000),
                    (borrow_id, amount),
                );
                let pays_fee =
                    <dyn ::frame_support::dispatch::PaysFee<(&BorrowId, &T::Balance)>>::pays_fee(
                        &SimpleDispatchInfo::FixedNormal(500_000),
                        (borrow_id, amount),
                    );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::repay(ref borrow_id) => {
                let weight = <dyn ::frame_support::dispatch::WeighData<(&BorrowId,)>>::weigh_data(
                    &SimpleDispatchInfo::FixedNormal(1_000_000),
                    (borrow_id,),
                );
                let class = < dyn :: frame_support :: dispatch :: ClassifyDispatch < ( & BorrowId , ) > > :: classify_dispatch ( & SimpleDispatchInfo :: FixedNormal ( 1_000_000 ) , ( borrow_id , ) ) ;
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<(&BorrowId,)>>::pays_fee(
                    &SimpleDispatchInfo::FixedNormal(1_000_000),
                    (borrow_id,),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::__PhantomItem(_, _) => {
                ::std::rt::begin_panic_fmt(&::core::fmt::Arguments::new_v1(
                    &["internal error: entered unreachable code: "],
                    &match (&"__PhantomItem should never be used.",) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ))
            }
        }
    }
}
impl<T: Trait> ::frame_support::dispatch::GetCallName for Call<T> {
    fn get_call_name(&self) -> &'static str {
        match *self {
            Call::pause() => {
                let _ = ();
                "pause"
            }
            Call::resume() => {
                let _ = ();
                "resume"
            }
            Call::change_platform(ref platform) => {
                let _ = (platform);
                "change_platform"
            }
            Call::change_money_pool(ref pool) => {
                let _ = (pool);
                "change_money_pool"
            }
            Call::change_safe_ltv(ref ltv) => {
                let _ = (ltv);
                "change_safe_ltv"
            }
            Call::change_liquidate_ltv(ref ltv) => {
                let _ = (ltv);
                "change_liquidate_ltv"
            }
            Call::change_min_borrow_terms(ref t) => {
                let _ = (t);
                "change_min_borrow_terms"
            }
            Call::change_min_borrow_interest_rate(ref r) => {
                let _ = (r);
                "change_min_borrow_interest_rate"
            }
            Call::list_borrow(ref collateral_balance, ref trading_pair, ref borrow_options) => {
                let _ = (collateral_balance, trading_pair, borrow_options);
                "list_borrow"
            }
            Call::unlist_borrow(ref borrow_id) => {
                let _ = (borrow_id);
                "unlist_borrow"
            }
            Call::lend(ref borrow_id) => {
                let _ = (borrow_id);
                "lend"
            }
            Call::liquidate(ref loan_id) => {
                let _ = (loan_id);
                "liquidate"
            }
            Call::add(ref borrow_id, ref amount) => {
                let _ = (borrow_id, amount);
                "add"
            }
            Call::repay(ref borrow_id) => {
                let _ = (borrow_id);
                "repay"
            }
            Call::__PhantomItem(_, _) => {
                ::std::rt::begin_panic_fmt(&::core::fmt::Arguments::new_v1(
                    &["internal error: entered unreachable code: "],
                    &match (&"__PhantomItem should never be used.",) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ))
            }
        }
    }
    fn get_call_names() -> &'static [&'static str] {
        &[
            "pause",
            "resume",
            "change_platform",
            "change_money_pool",
            "change_safe_ltv",
            "change_liquidate_ltv",
            "change_min_borrow_terms",
            "change_min_borrow_interest_rate",
            "list_borrow",
            "unlist_borrow",
            "lend",
            "liquidate",
            "add",
            "repay",
        ]
    }
}
impl<T: Trait> ::frame_support::dispatch::Clone for Call<T> {
    fn clone(&self) -> Self {
        match *self {
            Call::pause() => Call::pause(),
            Call::resume() => Call::resume(),
            Call::change_platform(ref platform) => Call::change_platform((*platform).clone()),
            Call::change_money_pool(ref pool) => Call::change_money_pool((*pool).clone()),
            Call::change_safe_ltv(ref ltv) => Call::change_safe_ltv((*ltv).clone()),
            Call::change_liquidate_ltv(ref ltv) => Call::change_liquidate_ltv((*ltv).clone()),
            Call::change_min_borrow_terms(ref t) => Call::change_min_borrow_terms((*t).clone()),
            Call::change_min_borrow_interest_rate(ref r) => {
                Call::change_min_borrow_interest_rate((*r).clone())
            }
            Call::list_borrow(ref collateral_balance, ref trading_pair, ref borrow_options) => {
                Call::list_borrow(
                    (*collateral_balance).clone(),
                    (*trading_pair).clone(),
                    (*borrow_options).clone(),
                )
            }
            Call::unlist_borrow(ref borrow_id) => Call::unlist_borrow((*borrow_id).clone()),
            Call::lend(ref borrow_id) => Call::lend((*borrow_id).clone()),
            Call::liquidate(ref loan_id) => Call::liquidate((*loan_id).clone()),
            Call::add(ref borrow_id, ref amount) => {
                Call::add((*borrow_id).clone(), (*amount).clone())
            }
            Call::repay(ref borrow_id) => Call::repay((*borrow_id).clone()),
            _ => ::std::rt::begin_panic("internal error: entered unreachable code"),
        }
    }
}
impl<T: Trait> ::frame_support::dispatch::PartialEq for Call<T> {
    fn eq(&self, _other: &Self) -> bool {
        match *self {
            Call::pause() => {
                let self_params = ();
                if let Call::pause() = *_other {
                    self_params == ()
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::resume() => {
                let self_params = ();
                if let Call::resume() = *_other {
                    self_params == ()
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::change_platform(ref platform) => {
                let self_params = (platform,);
                if let Call::change_platform(ref platform) = *_other {
                    self_params == (platform,)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::change_money_pool(ref pool) => {
                let self_params = (pool,);
                if let Call::change_money_pool(ref pool) = *_other {
                    self_params == (pool,)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::change_safe_ltv(ref ltv) => {
                let self_params = (ltv,);
                if let Call::change_safe_ltv(ref ltv) = *_other {
                    self_params == (ltv,)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::change_liquidate_ltv(ref ltv) => {
                let self_params = (ltv,);
                if let Call::change_liquidate_ltv(ref ltv) = *_other {
                    self_params == (ltv,)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::change_min_borrow_terms(ref t) => {
                let self_params = (t,);
                if let Call::change_min_borrow_terms(ref t) = *_other {
                    self_params == (t,)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::change_min_borrow_interest_rate(ref r) => {
                let self_params = (r,);
                if let Call::change_min_borrow_interest_rate(ref r) = *_other {
                    self_params == (r,)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::list_borrow(ref collateral_balance, ref trading_pair, ref borrow_options) => {
                let self_params = (collateral_balance, trading_pair, borrow_options);
                if let Call::list_borrow(
                    ref collateral_balance,
                    ref trading_pair,
                    ref borrow_options,
                ) = *_other
                {
                    self_params == (collateral_balance, trading_pair, borrow_options)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::unlist_borrow(ref borrow_id) => {
                let self_params = (borrow_id,);
                if let Call::unlist_borrow(ref borrow_id) = *_other {
                    self_params == (borrow_id,)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::lend(ref borrow_id) => {
                let self_params = (borrow_id,);
                if let Call::lend(ref borrow_id) = *_other {
                    self_params == (borrow_id,)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::liquidate(ref loan_id) => {
                let self_params = (loan_id,);
                if let Call::liquidate(ref loan_id) = *_other {
                    self_params == (loan_id,)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::add(ref borrow_id, ref amount) => {
                let self_params = (borrow_id, amount);
                if let Call::add(ref borrow_id, ref amount) = *_other {
                    self_params == (borrow_id, amount)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::repay(ref borrow_id) => {
                let self_params = (borrow_id,);
                if let Call::repay(ref borrow_id) = *_other {
                    self_params == (borrow_id,)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            _ => ::std::rt::begin_panic("internal error: entered unreachable code"),
        }
    }
}
impl<T: Trait> ::frame_support::dispatch::Eq for Call<T> {}
impl<T: Trait> ::frame_support::dispatch::fmt::Debug for Call<T> {
    fn fmt(
        &self,
        _f: &mut ::frame_support::dispatch::fmt::Formatter,
    ) -> ::frame_support::dispatch::result::Result<(), ::frame_support::dispatch::fmt::Error> {
        match *self {
            Call::pause() => _f.write_fmt(::core::fmt::Arguments::new_v1(
                &["", ""],
                &match (&"pause", &()) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                    ],
                },
            )),
            Call::resume() => _f.write_fmt(::core::fmt::Arguments::new_v1(
                &["", ""],
                &match (&"resume", &()) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                    ],
                },
            )),
            Call::change_platform(ref platform) => _f.write_fmt(::core::fmt::Arguments::new_v1(
                &["", ""],
                &match (&"change_platform", &(platform.clone(),)) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                    ],
                },
            )),
            Call::change_money_pool(ref pool) => _f.write_fmt(::core::fmt::Arguments::new_v1(
                &["", ""],
                &match (&"change_money_pool", &(pool.clone(),)) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                    ],
                },
            )),
            Call::change_safe_ltv(ref ltv) => _f.write_fmt(::core::fmt::Arguments::new_v1(
                &["", ""],
                &match (&"change_safe_ltv", &(ltv.clone(),)) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                    ],
                },
            )),
            Call::change_liquidate_ltv(ref ltv) => _f.write_fmt(::core::fmt::Arguments::new_v1(
                &["", ""],
                &match (&"change_liquidate_ltv", &(ltv.clone(),)) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                    ],
                },
            )),
            Call::change_min_borrow_terms(ref t) => _f.write_fmt(::core::fmt::Arguments::new_v1(
                &["", ""],
                &match (&"change_min_borrow_terms", &(t.clone(),)) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                    ],
                },
            )),
            Call::change_min_borrow_interest_rate(ref r) => {
                _f.write_fmt(::core::fmt::Arguments::new_v1(
                    &["", ""],
                    &match (&"change_min_borrow_interest_rate", &(r.clone(),)) {
                        (arg0, arg1) => [
                            ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                            ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                        ],
                    },
                ))
            }
            Call::list_borrow(ref collateral_balance, ref trading_pair, ref borrow_options) => _f
                .write_fmt(::core::fmt::Arguments::new_v1(
                    &["", ""],
                    &match (
                        &"list_borrow",
                        &(
                            collateral_balance.clone(),
                            trading_pair.clone(),
                            borrow_options.clone(),
                        ),
                    ) {
                        (arg0, arg1) => [
                            ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                            ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                        ],
                    },
                )),
            Call::unlist_borrow(ref borrow_id) => _f.write_fmt(::core::fmt::Arguments::new_v1(
                &["", ""],
                &match (&"unlist_borrow", &(borrow_id.clone(),)) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                    ],
                },
            )),
            Call::lend(ref borrow_id) => _f.write_fmt(::core::fmt::Arguments::new_v1(
                &["", ""],
                &match (&"lend", &(borrow_id.clone(),)) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                    ],
                },
            )),
            Call::liquidate(ref loan_id) => _f.write_fmt(::core::fmt::Arguments::new_v1(
                &["", ""],
                &match (&"liquidate", &(loan_id.clone(),)) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                    ],
                },
            )),
            Call::add(ref borrow_id, ref amount) => _f.write_fmt(::core::fmt::Arguments::new_v1(
                &["", ""],
                &match (&"add", &(borrow_id.clone(), amount.clone())) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                    ],
                },
            )),
            Call::repay(ref borrow_id) => _f.write_fmt(::core::fmt::Arguments::new_v1(
                &["", ""],
                &match (&"repay", &(borrow_id.clone(),)) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                    ],
                },
            )),
            _ => ::std::rt::begin_panic("internal error: entered unreachable code"),
        }
    }
}
impl<T: Trait> ::frame_support::dispatch::Dispatchable for Call<T> {
    type Trait = T;
    type Origin = T::Origin;
    fn dispatch(self, _origin: Self::Origin) -> ::frame_support::sp_runtime::DispatchResult {
        match self {
            Call::pause() => <Module<T>>::pause(_origin),
            Call::resume() => <Module<T>>::resume(_origin),
            Call::change_platform(platform) => <Module<T>>::change_platform(_origin, platform),
            Call::change_money_pool(pool) => <Module<T>>::change_money_pool(_origin, pool),
            Call::change_safe_ltv(ltv) => <Module<T>>::change_safe_ltv(_origin, ltv),
            Call::change_liquidate_ltv(ltv) => <Module<T>>::change_liquidate_ltv(_origin, ltv),
            Call::change_min_borrow_terms(t) => <Module<T>>::change_min_borrow_terms(_origin, t),
            Call::change_min_borrow_interest_rate(r) => {
                <Module<T>>::change_min_borrow_interest_rate(_origin, r)
            }
            Call::list_borrow(collateral_balance, trading_pair, borrow_options) => {
                <Module<T>>::list_borrow(_origin, collateral_balance, trading_pair, borrow_options)
            }
            Call::unlist_borrow(borrow_id) => <Module<T>>::unlist_borrow(_origin, borrow_id),
            Call::lend(borrow_id) => <Module<T>>::lend(_origin, borrow_id),
            Call::liquidate(loan_id) => <Module<T>>::liquidate(_origin, loan_id),
            Call::add(borrow_id, amount) => <Module<T>>::add(_origin, borrow_id, amount),
            Call::repay(borrow_id) => <Module<T>>::repay(_origin, borrow_id),
            Call::__PhantomItem(_, _) => {
                ::std::rt::begin_panic_fmt(&::core::fmt::Arguments::new_v1(
                    &["internal error: entered unreachable code: "],
                    &match (&"__PhantomItem should never be used.",) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ))
            }
        }
    }
}
impl<T: Trait> ::frame_support::dispatch::Callable<T> for Module<T> {
    type Call = Call<T>;
}
impl<T: Trait> Module<T> {
    #[doc(hidden)]
    pub fn dispatch<D: ::frame_support::dispatch::Dispatchable<Trait = T>>(
        d: D,
        origin: D::Origin,
    ) -> ::frame_support::sp_runtime::DispatchResult {
        d.dispatch(origin)
    }
}
impl<T: Trait> Module<T> {
    #[doc(hidden)]
    pub fn call_functions() -> &'static [::frame_support::dispatch::FunctionMetadata] {
        &[
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("pause"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("resume"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("change_platform"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("platform"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("T::AccountId"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("change_money_pool"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("pool"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("T::AccountId"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("change_safe_ltv"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("ltv"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("u32"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("change_liquidate_ltv"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("ltv"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("u32"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("change_min_borrow_terms"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("t"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("u64"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode(
                    "change_min_borrow_interest_rate",
                ),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("r"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("u64"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("list_borrow"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode(
                            "collateral_balance",
                        ),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("T::Balance"),
                    },
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("trading_pair"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode(
                            "TradingPair<T::AssetId>",
                        ),
                    },
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("borrow_options"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode(
                            "BorrowOptions<T::Balance, T::BlockNumber>",
                        ),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("unlist_borrow"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("borrow_id"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("BorrowId"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("lend"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("borrow_id"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("BorrowId"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("liquidate"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("loan_id"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("LoanId"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("add"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("borrow_id"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("BorrowId"),
                    },
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("amount"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("T::Balance"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("repay"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("borrow_id"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("BorrowId"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
        ]
    }
}
impl<T: 'static + Trait> Module<T> {
    #[doc(hidden)]
    pub fn module_constants_metadata(
    ) -> &'static [::frame_support::dispatch::ModuleConstantMetadata] {
        #[allow(non_upper_case_types)]
        #[allow(non_camel_case_types)]
        struct LTV_SCALEDefaultByteGetter<T: Trait>(
            ::frame_support::dispatch::marker::PhantomData<(T,)>,
        );
        impl<T: 'static + Trait> ::frame_support::dispatch::DefaultByte for LTV_SCALEDefaultByteGetter<T> {
            fn default_byte(&self) -> ::frame_support::dispatch::Vec<u8> {
                let value: u32 = LTV_SCALE;
                ::frame_support::dispatch::Encode::encode(&value)
            }
        }
        unsafe impl<T: 'static + Trait> Send for LTV_SCALEDefaultByteGetter<T> {}
        unsafe impl<T: 'static + Trait> Sync for LTV_SCALEDefaultByteGetter<T> {}
        #[allow(non_upper_case_types)]
        #[allow(non_camel_case_types)]
        struct INTEREST_SCALEDefaultByteGetter<T: Trait>(
            ::frame_support::dispatch::marker::PhantomData<(T,)>,
        );
        impl<T: 'static + Trait> ::frame_support::dispatch::DefaultByte
            for INTEREST_SCALEDefaultByteGetter<T>
        {
            fn default_byte(&self) -> ::frame_support::dispatch::Vec<u8> {
                let value: u64 = INTEREST_RATE_PRECISION;
                ::frame_support::dispatch::Encode::encode(&value)
            }
        }
        unsafe impl<T: 'static + Trait> Send for INTEREST_SCALEDefaultByteGetter<T> {}
        unsafe impl<T: 'static + Trait> Sync for INTEREST_SCALEDefaultByteGetter<T> {}
        &[
            ::frame_support::dispatch::ModuleConstantMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("LTV_SCALE"),
                ty: ::frame_support::dispatch::DecodeDifferent::Encode("u32"),
                value: ::frame_support::dispatch::DecodeDifferent::Encode(
                    ::frame_support::dispatch::DefaultByteGetter(&LTV_SCALEDefaultByteGetter::<T>(
                        ::frame_support::dispatch::marker::PhantomData,
                    )),
                ),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::dispatch::ModuleConstantMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("INTEREST_SCALE"),
                ty: ::frame_support::dispatch::DecodeDifferent::Encode("u64"),
                value: ::frame_support::dispatch::DecodeDifferent::Encode(
                    ::frame_support::dispatch::DefaultByteGetter(
                        &INTEREST_SCALEDefaultByteGetter::<T>(
                            ::frame_support::dispatch::marker::PhantomData,
                        ),
                    ),
                ),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[]),
            },
        ]
    }
}
impl<T: Trait> ::frame_support::dispatch::ModuleErrorMetadata for Module<T> {
    fn metadata() -> &'static [::frame_support::dispatch::ErrorMetadata] {
        <&'static str as ::frame_support::dispatch::ModuleErrorMetadata>::metadata()
    }
}
/// [`RawEvent`] specialized for the configuration [`Trait`]
///
/// [`RawEvent`]: enum.RawEvent.html
/// [`Trait`]: trait.Trait.html
pub type Event<T> = RawEvent<
    Loan<
        <T as generic_asset::Trait>::AssetId,
        <T as generic_asset::Trait>::Balance,
        <T as system::Trait>::BlockNumber,
        <T as system::Trait>::AccountId,
    >,
    Borrow<
        <T as generic_asset::Trait>::AssetId,
        <T as generic_asset::Trait>::Balance,
        <T as system::Trait>::BlockNumber,
        <T as system::Trait>::AccountId,
    >,
>;
# [ doc = " Events for this module." ] # [ doc = "" ] # [ rustfmt :: skip ] pub enum RawEvent < Loan , Borrow > { BorrowListed ( Borrow ) , BorrowUnlisted ( BorrowId ) , LoanCreated ( Loan ) , LoanLiquidated ( LoanId ) , LoanRepaid ( LoanId ) , CollateralAdded ( BorrowId ) , BorrowDied ( BorrowId ) , LoanOverdue ( LoanId ) , LoanToBeLiquidated ( LoanId ) , }
#[automatically_derived]
#[allow(unused_qualifications)]
impl<Loan: ::core::clone::Clone, Borrow: ::core::clone::Clone> ::core::clone::Clone
    for RawEvent<Loan, Borrow>
{
    #[inline]
    fn clone(&self) -> RawEvent<Loan, Borrow> {
        match (&*self,) {
            (&RawEvent::BorrowListed(ref __self_0),) => {
                RawEvent::BorrowListed(::core::clone::Clone::clone(&(*__self_0)))
            }
            (&RawEvent::BorrowUnlisted(ref __self_0),) => {
                RawEvent::BorrowUnlisted(::core::clone::Clone::clone(&(*__self_0)))
            }
            (&RawEvent::LoanCreated(ref __self_0),) => {
                RawEvent::LoanCreated(::core::clone::Clone::clone(&(*__self_0)))
            }
            (&RawEvent::LoanLiquidated(ref __self_0),) => {
                RawEvent::LoanLiquidated(::core::clone::Clone::clone(&(*__self_0)))
            }
            (&RawEvent::LoanRepaid(ref __self_0),) => {
                RawEvent::LoanRepaid(::core::clone::Clone::clone(&(*__self_0)))
            }
            (&RawEvent::CollateralAdded(ref __self_0),) => {
                RawEvent::CollateralAdded(::core::clone::Clone::clone(&(*__self_0)))
            }
            (&RawEvent::BorrowDied(ref __self_0),) => {
                RawEvent::BorrowDied(::core::clone::Clone::clone(&(*__self_0)))
            }
            (&RawEvent::LoanOverdue(ref __self_0),) => {
                RawEvent::LoanOverdue(::core::clone::Clone::clone(&(*__self_0)))
            }
            (&RawEvent::LoanToBeLiquidated(ref __self_0),) => {
                RawEvent::LoanToBeLiquidated(::core::clone::Clone::clone(&(*__self_0)))
            }
        }
    }
}
impl<Loan, Borrow> ::core::marker::StructuralPartialEq for RawEvent<Loan, Borrow> {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<Loan: ::core::cmp::PartialEq, Borrow: ::core::cmp::PartialEq> ::core::cmp::PartialEq
    for RawEvent<Loan, Borrow>
{
    #[inline]
    fn eq(&self, other: &RawEvent<Loan, Borrow>) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) } as isize;
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) } as isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (
                        &RawEvent::BorrowListed(ref __self_0),
                        &RawEvent::BorrowListed(ref __arg_1_0),
                    ) => (*__self_0) == (*__arg_1_0),
                    (
                        &RawEvent::BorrowUnlisted(ref __self_0),
                        &RawEvent::BorrowUnlisted(ref __arg_1_0),
                    ) => (*__self_0) == (*__arg_1_0),
                    (
                        &RawEvent::LoanCreated(ref __self_0),
                        &RawEvent::LoanCreated(ref __arg_1_0),
                    ) => (*__self_0) == (*__arg_1_0),
                    (
                        &RawEvent::LoanLiquidated(ref __self_0),
                        &RawEvent::LoanLiquidated(ref __arg_1_0),
                    ) => (*__self_0) == (*__arg_1_0),
                    (&RawEvent::LoanRepaid(ref __self_0), &RawEvent::LoanRepaid(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (
                        &RawEvent::CollateralAdded(ref __self_0),
                        &RawEvent::CollateralAdded(ref __arg_1_0),
                    ) => (*__self_0) == (*__arg_1_0),
                    (&RawEvent::BorrowDied(ref __self_0), &RawEvent::BorrowDied(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    (
                        &RawEvent::LoanOverdue(ref __self_0),
                        &RawEvent::LoanOverdue(ref __arg_1_0),
                    ) => (*__self_0) == (*__arg_1_0),
                    (
                        &RawEvent::LoanToBeLiquidated(ref __self_0),
                        &RawEvent::LoanToBeLiquidated(ref __arg_1_0),
                    ) => (*__self_0) == (*__arg_1_0),
                    _ => unsafe { ::core::intrinsics::unreachable() },
                }
            } else {
                false
            }
        }
    }
    #[inline]
    fn ne(&self, other: &RawEvent<Loan, Borrow>) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) } as isize;
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) } as isize;
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (
                        &RawEvent::BorrowListed(ref __self_0),
                        &RawEvent::BorrowListed(ref __arg_1_0),
                    ) => (*__self_0) != (*__arg_1_0),
                    (
                        &RawEvent::BorrowUnlisted(ref __self_0),
                        &RawEvent::BorrowUnlisted(ref __arg_1_0),
                    ) => (*__self_0) != (*__arg_1_0),
                    (
                        &RawEvent::LoanCreated(ref __self_0),
                        &RawEvent::LoanCreated(ref __arg_1_0),
                    ) => (*__self_0) != (*__arg_1_0),
                    (
                        &RawEvent::LoanLiquidated(ref __self_0),
                        &RawEvent::LoanLiquidated(ref __arg_1_0),
                    ) => (*__self_0) != (*__arg_1_0),
                    (&RawEvent::LoanRepaid(ref __self_0), &RawEvent::LoanRepaid(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (
                        &RawEvent::CollateralAdded(ref __self_0),
                        &RawEvent::CollateralAdded(ref __arg_1_0),
                    ) => (*__self_0) != (*__arg_1_0),
                    (&RawEvent::BorrowDied(ref __self_0), &RawEvent::BorrowDied(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    (
                        &RawEvent::LoanOverdue(ref __self_0),
                        &RawEvent::LoanOverdue(ref __arg_1_0),
                    ) => (*__self_0) != (*__arg_1_0),
                    (
                        &RawEvent::LoanToBeLiquidated(ref __self_0),
                        &RawEvent::LoanToBeLiquidated(ref __arg_1_0),
                    ) => (*__self_0) != (*__arg_1_0),
                    _ => unsafe { ::core::intrinsics::unreachable() },
                }
            } else {
                true
            }
        }
    }
}
impl<Loan, Borrow> ::core::marker::StructuralEq for RawEvent<Loan, Borrow> {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<Loan: ::core::cmp::Eq, Borrow: ::core::cmp::Eq> ::core::cmp::Eq for RawEvent<Loan, Borrow> {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<Borrow>;
            let _: ::core::cmp::AssertParamIsEq<BorrowId>;
            let _: ::core::cmp::AssertParamIsEq<Loan>;
            let _: ::core::cmp::AssertParamIsEq<LoanId>;
            let _: ::core::cmp::AssertParamIsEq<LoanId>;
            let _: ::core::cmp::AssertParamIsEq<BorrowId>;
            let _: ::core::cmp::AssertParamIsEq<BorrowId>;
            let _: ::core::cmp::AssertParamIsEq<LoanId>;
            let _: ::core::cmp::AssertParamIsEq<LoanId>;
        }
    }
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<Loan, Borrow> _parity_scale_codec::Encode for RawEvent<Loan, Borrow>
    where
        Borrow: _parity_scale_codec::Encode,
        Borrow: _parity_scale_codec::Encode,
        Loan: _parity_scale_codec::Encode,
        Loan: _parity_scale_codec::Encode,
    {
        fn encode_to<EncOut: _parity_scale_codec::Output>(&self, dest: &mut EncOut) {
            match *self {
                RawEvent::BorrowListed(ref aa) => {
                    dest.push_byte(0usize as u8);
                    dest.push(aa);
                }
                RawEvent::BorrowUnlisted(ref aa) => {
                    dest.push_byte(1usize as u8);
                    dest.push(aa);
                }
                RawEvent::LoanCreated(ref aa) => {
                    dest.push_byte(2usize as u8);
                    dest.push(aa);
                }
                RawEvent::LoanLiquidated(ref aa) => {
                    dest.push_byte(3usize as u8);
                    dest.push(aa);
                }
                RawEvent::LoanRepaid(ref aa) => {
                    dest.push_byte(4usize as u8);
                    dest.push(aa);
                }
                RawEvent::CollateralAdded(ref aa) => {
                    dest.push_byte(5usize as u8);
                    dest.push(aa);
                }
                RawEvent::BorrowDied(ref aa) => {
                    dest.push_byte(6usize as u8);
                    dest.push(aa);
                }
                RawEvent::LoanOverdue(ref aa) => {
                    dest.push_byte(7usize as u8);
                    dest.push(aa);
                }
                RawEvent::LoanToBeLiquidated(ref aa) => {
                    dest.push_byte(8usize as u8);
                    dest.push(aa);
                }
                _ => (),
            }
        }
    }
    impl<Loan, Borrow> _parity_scale_codec::EncodeLike for RawEvent<Loan, Borrow>
    where
        Borrow: _parity_scale_codec::Encode,
        Borrow: _parity_scale_codec::Encode,
        Loan: _parity_scale_codec::Encode,
        Loan: _parity_scale_codec::Encode,
    {
    }
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<Loan, Borrow> _parity_scale_codec::Decode for RawEvent<Loan, Borrow>
    where
        Borrow: _parity_scale_codec::Decode,
        Borrow: _parity_scale_codec::Decode,
        Loan: _parity_scale_codec::Decode,
        Loan: _parity_scale_codec::Decode,
    {
        fn decode<DecIn: _parity_scale_codec::Input>(
            input: &mut DecIn,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            match input.read_byte()? {
                x if x == 0usize as u8 => Ok(RawEvent::BorrowListed({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field RawEvent :: BorrowListed.0".into())
                        }
                        Ok(a) => a,
                    }
                })),
                x if x == 1usize as u8 => Ok(RawEvent::BorrowUnlisted({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field RawEvent :: BorrowUnlisted.0".into())
                        }
                        Ok(a) => a,
                    }
                })),
                x if x == 2usize as u8 => Ok(RawEvent::LoanCreated({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field RawEvent :: LoanCreated.0".into())
                        }
                        Ok(a) => a,
                    }
                })),
                x if x == 3usize as u8 => Ok(RawEvent::LoanLiquidated({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field RawEvent :: LoanLiquidated.0".into())
                        }
                        Ok(a) => a,
                    }
                })),
                x if x == 4usize as u8 => Ok(RawEvent::LoanRepaid({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field RawEvent :: LoanRepaid.0".into())
                        }
                        Ok(a) => a,
                    }
                })),
                x if x == 5usize as u8 => Ok(RawEvent::CollateralAdded({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field RawEvent :: CollateralAdded.0".into())
                        }
                        Ok(a) => a,
                    }
                })),
                x if x == 6usize as u8 => Ok(RawEvent::BorrowDied({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field RawEvent :: BorrowDied.0".into())
                        }
                        Ok(a) => a,
                    }
                })),
                x if x == 7usize as u8 => Ok(RawEvent::LoanOverdue({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err("Error decoding field RawEvent :: LoanOverdue.0".into())
                        }
                        Ok(a) => a,
                    }
                })),
                x if x == 8usize as u8 => Ok(RawEvent::LoanToBeLiquidated({
                    let res = _parity_scale_codec::Decode::decode(input);
                    match res {
                        Err(_) => {
                            return Err(
                                "Error decoding field RawEvent :: LoanToBeLiquidated.0".into()
                            )
                        }
                        Ok(a) => a,
                    }
                })),
                x => Err("No such variant in enum RawEvent".into()),
            }
        }
    }
};
impl<Loan, Borrow> core::fmt::Debug for RawEvent<Loan, Borrow>
where
    Loan: core::fmt::Debug,
    Borrow: core::fmt::Debug,
{
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::BorrowListed(ref a0) => {
                fmt.debug_tuple("RawEvent::BorrowListed").field(a0).finish()
            }
            Self::BorrowUnlisted(ref a0) => fmt
                .debug_tuple("RawEvent::BorrowUnlisted")
                .field(a0)
                .finish(),
            Self::LoanCreated(ref a0) => {
                fmt.debug_tuple("RawEvent::LoanCreated").field(a0).finish()
            }
            Self::LoanLiquidated(ref a0) => fmt
                .debug_tuple("RawEvent::LoanLiquidated")
                .field(a0)
                .finish(),
            Self::LoanRepaid(ref a0) => fmt.debug_tuple("RawEvent::LoanRepaid").field(a0).finish(),
            Self::CollateralAdded(ref a0) => fmt
                .debug_tuple("RawEvent::CollateralAdded")
                .field(a0)
                .finish(),
            Self::BorrowDied(ref a0) => fmt.debug_tuple("RawEvent::BorrowDied").field(a0).finish(),
            Self::LoanOverdue(ref a0) => {
                fmt.debug_tuple("RawEvent::LoanOverdue").field(a0).finish()
            }
            Self::LoanToBeLiquidated(ref a0) => fmt
                .debug_tuple("RawEvent::LoanToBeLiquidated")
                .field(a0)
                .finish(),
            _ => Ok(()),
        }
    }
}
impl<Loan, Borrow> From<RawEvent<Loan, Borrow>> for () {
    fn from(_: RawEvent<Loan, Borrow>) -> () {
        ()
    }
}
impl<Loan, Borrow> RawEvent<Loan, Borrow> {
    #[allow(dead_code)]
    pub fn metadata() -> &'static [::frame_support::event::EventMetadata] {
        &[
            ::frame_support::event::EventMetadata {
                name: ::frame_support::event::DecodeDifferent::Encode("BorrowListed"),
                arguments: ::frame_support::event::DecodeDifferent::Encode(&["Borrow"]),
                documentation: ::frame_support::event::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::event::EventMetadata {
                name: ::frame_support::event::DecodeDifferent::Encode("BorrowUnlisted"),
                arguments: ::frame_support::event::DecodeDifferent::Encode(&["BorrowId"]),
                documentation: ::frame_support::event::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::event::EventMetadata {
                name: ::frame_support::event::DecodeDifferent::Encode("LoanCreated"),
                arguments: ::frame_support::event::DecodeDifferent::Encode(&["Loan"]),
                documentation: ::frame_support::event::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::event::EventMetadata {
                name: ::frame_support::event::DecodeDifferent::Encode("LoanLiquidated"),
                arguments: ::frame_support::event::DecodeDifferent::Encode(&["LoanId"]),
                documentation: ::frame_support::event::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::event::EventMetadata {
                name: ::frame_support::event::DecodeDifferent::Encode("LoanRepaid"),
                arguments: ::frame_support::event::DecodeDifferent::Encode(&["LoanId"]),
                documentation: ::frame_support::event::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::event::EventMetadata {
                name: ::frame_support::event::DecodeDifferent::Encode("CollateralAdded"),
                arguments: ::frame_support::event::DecodeDifferent::Encode(&["BorrowId"]),
                documentation: ::frame_support::event::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::event::EventMetadata {
                name: ::frame_support::event::DecodeDifferent::Encode("BorrowDied"),
                arguments: ::frame_support::event::DecodeDifferent::Encode(&["BorrowId"]),
                documentation: ::frame_support::event::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::event::EventMetadata {
                name: ::frame_support::event::DecodeDifferent::Encode("LoanOverdue"),
                arguments: ::frame_support::event::DecodeDifferent::Encode(&["LoanId"]),
                documentation: ::frame_support::event::DecodeDifferent::Encode(&[]),
            },
            ::frame_support::event::EventMetadata {
                name: ::frame_support::event::DecodeDifferent::Encode("LoanToBeLiquidated"),
                arguments: ::frame_support::event::DecodeDifferent::Encode(&["LoanId"]),
                documentation: ::frame_support::event::DecodeDifferent::Encode(&[]),
            },
        ]
    }
}
impl<T: Trait> Module<T> {
    fn generate_borrow_id() -> BorrowId {
        let id = Self::next_borrow_id();
        NextBorrowId::mutate(|v| *v += 1);
        id
    }
    fn generate_loan_id() -> LoanId {
        let id = Self::next_loan_id();
        NextLoanId::mutate(|v| *v += 1);
        id
    }
    pub fn fetch_trading_pair_prices(
        borrow_asset_id: T::AssetId,
        collateral_asset_id: T::AssetId,
    ) -> Option<TradingPairPrices> {
        let collateral_price = Self::fetch_price(collateral_asset_id);
        let borrow_price = Self::fetch_price(borrow_asset_id);
        if collateral_price.is_some() && borrow_price.is_some() {
            Some(TradingPairPrices {
                borrow_asset_price: borrow_price.unwrap(),
                collateral_asset_price: collateral_price.unwrap(),
            })
        } else {
            None
        }
    }
    pub fn add_collateral(
        who: T::AccountId,
        borrow_id: BorrowId,
        amount: T::Balance,
    ) -> DispatchResult {
        {
            if !<Borrows<T>>::contains_key(borrow_id) {
                {
                    return Err(Error::<T>::UnknownBorrowId.into());
                };
            }
        };
        let borrow = <Borrows<T>>::get(borrow_id);
        {
            if !(<generic_asset::Module<T>>::free_balance(&borrow.collateral_asset_id, &who)
                >= amount)
            {
                {
                    return Err(Error::<T>::NotEnoughBalance.into());
                };
            }
        };
        <generic_asset::Module<T>>::make_transfer_with_event(
            &borrow.collateral_asset_id,
            &who,
            &<MoneyPool<T>>::get(),
            amount,
        )?;
        <Borrows<T>>::mutate(&borrow_id, |v| {
            v.collateral_balance = v.collateral_balance.checked_add(&amount).unwrap();
        });
        if borrow.loan_id.is_some() {
            <Loans<T>>::mutate(borrow.loan_id.unwrap(), |v| {
                v.collateral_balance = v.collateral_balance.checked_add(&amount).unwrap();
            });
        }
        Self::deposit_event(RawEvent::CollateralAdded(borrow_id));
        Ok(())
    }
    pub fn repay_loan(who: T::AccountId, borrow_id: BorrowId) -> DispatchResult {
        {
            if !<Borrows<T>>::contains_key(borrow_id) {
                {
                    return Err(Error::<T>::UnknownBorrowId.into());
                };
            }
        };
        let borrow = <Borrows<T>>::get(borrow_id);
        {
            if !(&borrow.who == &who) {
                {
                    return Err(Error::<T>::NotOwnerOfBorrow.into());
                };
            }
        };
        {
            if !borrow.loan_id.is_some() {
                {
                    return Err(Error::<T>::BorrowNotLoaned.into());
                };
            }
        };
        let trading_pair_prices =
            Self::fetch_trading_pair_prices(borrow.borrow_asset_id, borrow.collateral_asset_id)
                .ok_or(Error::<T>::TradingPairPriceMissing)?;
        {
            if !<Loans<T>>::contains_key(borrow.loan_id.unwrap()) {
                {
                    return Err(Error::<T>::UnknownLoanId.into());
                };
            }
        };
        let loan_id = borrow.loan_id.unwrap();
        let loan = <Loans<T>>::get(loan_id);
        {
            if !(loan.status == LoanHealth::Well) {
                {
                    return Err(Error::<T>::LoanNotWell.into());
                };
            }
        };
        if Self::ltv_meet_liquidation(
            &trading_pair_prices,
            loan.loan_balance,
            loan.collateral_balance,
        ) {
            <Loans<T>>::mutate(&loan.id, |v| {
                v.status = LoanHealth::ToBeLiquidated;
            });
            return Err(Error::<T>::ShouldBeLiquidated.into());
        }
        let expected_interest = Self::calculate_expected_interest(
            borrow.interest_rate,
            borrow.terms,
            borrow.borrow_balance,
        );
        let need_to_pay = borrow
            .borrow_balance
            .checked_add(&expected_interest)
            .unwrap();
        {
            if !(<generic_asset::Module<T>>::free_balance(&borrow.borrow_asset_id, &who)
                >= need_to_pay)
            {
                {
                    return Err(Error::<T>::NotEnoughBalance.into());
                };
            }
        };
        <generic_asset::Module<T>>::make_transfer_with_event(
            &borrow.borrow_asset_id,
            &who,
            &loan.loaner_id,
            need_to_pay,
        )?;
        <generic_asset::Module<T>>::make_transfer_with_event(
            &borrow.collateral_asset_id,
            &<MoneyPool<T>>::get(),
            &who,
            borrow.collateral_balance,
        )
        .or_else(|err| -> DispatchResult {
            <generic_asset::Module<T>>::make_transfer_with_event(
                &borrow.borrow_asset_id,
                &loan.loaner_id,
                &who,
                need_to_pay,
            )?;
            Err(err)
        })?;
        Self::repay_cleanup(borrow, loan);
        Self::deposit_event(RawEvent::LoanRepaid(loan_id));
        Ok(())
    }
    pub fn calculate_expected_interest(
        interest_rate: u64,
        terms: u64,
        amount: T::Balance,
    ) -> T::Balance {
        <T::Balance as TryFrom<u64>>::try_from(interest_rate)
            .ok()
            .unwrap()
            * <T::Balance as TryFrom<u64>>::try_from(terms).ok().unwrap()
            * amount
            / <T::Balance as TryFrom<u64>>::try_from(INTEREST_RATE_PRECISION)
                .ok()
                .unwrap()
    }
    pub fn create_borrow(
        who: T::AccountId,
        collateral_balance: T::Balance,
        trading_pair: TradingPair<T::AssetId>,
        borrow_options: BorrowOptions<T::Balance, T::BlockNumber>,
    ) -> DispatchResult {
        {
            if !(borrow_options.terms >= Self::min_borrow_terms()) {
                {
                    return Err(Error::<T>::MinBorrowTerms.into());
                };
            }
        };
        {
            if !(borrow_options.interest_rate >= Self::min_borrow_interest_rate()) {
                {
                    return Err(Error::<T>::MinBorrowInterestRate.into());
                };
            }
        };
        {
            if !Self::is_trading_pair_allowed(&trading_pair) {
                {
                    return Err(Error::<T>::TradingPairNotAllowed.into());
                };
            }
        };
        if let Some(id) = Self::borrow_ids_by_account_id(&who).last() {
            {
                if !!Self::alive_borrow_ids().contains(id) {
                    {
                        return Err(Error::<T>::MultipleAliveBorrows.into());
                    };
                }
            };
        }
        let trading_pair_prices =
            Self::fetch_trading_pair_prices(trading_pair.borrow, trading_pair.collateral)
                .ok_or(Error::<T>::TradingPairPriceMissing)?;
        {
            if !Self::ltv_meet_safty(
                &trading_pair_prices,
                borrow_options.amount,
                collateral_balance,
            ) {
                {
                    return Err(Error::<T>::InitialCollateralRateFail.into());
                };
            }
        };
        let borrow_id = Self::generate_borrow_id();
        let lock_id = <generic_asset::Module<T>>::reserve(
            &trading_pair.collateral,
            &who,
            collateral_balance,
        )?;
        let b = Borrow {
            id: borrow_id.clone(),
            lock_id: lock_id,
            who: who.clone(),
            status: Default::default(),
            borrow_asset_id: trading_pair.borrow,
            collateral_asset_id: trading_pair.collateral,
            borrow_balance: borrow_options.amount,
            collateral_balance: collateral_balance,
            terms: borrow_options.terms,
            interest_rate: borrow_options.interest_rate,
            dead_after: if let Some(blk_num) = borrow_options.warranty {
                Some(<system::Module<T>>::block_number().saturating_add(blk_num))
            } else {
                None
            },
            loan_id: None,
        };
        <Borrows<T>>::insert(&borrow_id, b.clone());
        AliveBorrowIds::append_or_put(<[_]>::into_vec(box [borrow_id.clone()]));
        <BorrowIdsByAccountId<T>>::append_or_insert(&who, <[_]>::into_vec(box [borrow_id.clone()]));
        Self::deposit_event(RawEvent::BorrowListed(b));
        Ok(())
    }
    pub fn remove_borrow(who: T::AccountId, borrow_id: BorrowId) -> DispatchResult {
        {
            if !<Borrows<T>>::contains_key(&borrow_id) {
                {
                    return Err(Error::<T>::UnknownBorrowId.into());
                };
            }
        };
        {
            if !<BorrowIdsByAccountId<T>>::get(&who).contains(&borrow_id) {
                {
                    return Err(Error::<T>::NotOwnerOfBorrow.into());
                };
            }
        };
        {
            if !AliveBorrowIds::get().contains(&borrow_id) {
                {
                    return Err(Error::<T>::BorrowNotAlive.into());
                };
            }
        };
        let borrow = <Borrows<T>>::get(borrow_id);
        <generic_asset::Module<T>>::unreserve(
            &borrow.collateral_asset_id,
            &who,
            borrow.collateral_balance,
            Some(borrow.lock_id),
        )?;
        AliveBorrowIds::mutate(|v| {
            *v = v
                .clone()
                .into_iter()
                .filter(|v| *v != borrow_id)
                .collect::<Vec<_>>();
        });
        Self::deposit_event(RawEvent::BorrowUnlisted(borrow_id));
        Ok(())
    }
    pub fn create_loan(loaner: T::AccountId, borrow_id: BorrowId) -> DispatchResult {
        let borrow = Self::ensure_borrow_available(borrow_id)?;
        let locked_balance = <generic_asset::Module<T>>::locked_balance(
            &borrow.collateral_asset_id,
            &borrow.who,
            borrow.lock_id,
        );
        match locked_balance {
            None => {
                {
                    let lvl = ::log::Level::Info;
                    if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                        ::log::__private_api_log(
                            ::core::fmt::Arguments::new_v1(
                                &["no locked balance"],
                                &match () {
                                    () => [],
                                },
                            ),
                            lvl,
                            &(
                                "ls_biding",
                                "ls_biding",
                                "runtime/modules/ls-biding/src/lib.rs",
                                651u32,
                            ),
                        );
                    }
                };
                return Err(Error::<T>::NoLockedBalance.into());
            }
            Some(collateral_balance) => {
                {
                    if !(<generic_asset::Module<T>>::free_balance(&borrow.borrow_asset_id, &loaner)
                        >= borrow.borrow_balance)
                    {
                        {
                            return Err(Error::<T>::NotEnoughBalance.into());
                        };
                    }
                };
                {
                    let lvl = ::log::Level::Info;
                    if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                        ::log::__private_api_log(
                            ::core::fmt::Arguments::new_v1(
                                &["enough balance"],
                                &match () {
                                    () => [],
                                },
                            ),
                            lvl,
                            &(
                                "ls_biding",
                                "ls_biding",
                                "runtime/modules/ls-biding/src/lib.rs",
                                660u32,
                            ),
                        );
                    }
                };
                let trading_pair_prices = Self::fetch_trading_pair_prices(
                    borrow.borrow_asset_id,
                    borrow.collateral_asset_id,
                )
                .ok_or(Error::<T>::TradingPairPriceMissing)?;
                {
                    if !Self::ltv_meet_safty(
                        &trading_pair_prices,
                        borrow.borrow_balance,
                        collateral_balance,
                    ) {
                        {
                            return Err(Error::<T>::InitialCollateralRateFail.into());
                        };
                    }
                };
                {
                    let lvl = ::log::Level::Info;
                    if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                        ::log::__private_api_log(
                            ::core::fmt::Arguments::new_v1(
                                &["meet init collateral rate"],
                                &match () {
                                    () => [],
                                },
                            ),
                            lvl,
                            &(
                                "ls_biding",
                                "ls_biding",
                                "runtime/modules/ls-biding/src/lib.rs",
                                675u32,
                            ),
                        );
                    }
                };
                let current_block_number = <system::Module<T>>::block_number();
                let loan = Loan {
                    id: Self::generate_loan_id(),
                    borrow_id: borrow_id,
                    borrower_id: borrow.who.clone(),
                    loaner_id: loaner.clone(),
                    due: current_block_number
                        + T::Days::get()
                            * <T::BlockNumber as TryFrom<u64>>::try_from(borrow.terms)
                                .ok()
                                .unwrap(),
                    collateral_asset_id: borrow.collateral_asset_id,
                    loan_asset_id: borrow.borrow_asset_id,
                    collateral_balance: collateral_balance,
                    loan_balance: borrow.borrow_balance,
                    status: LoanHealth::Well,
                    interest_rate: borrow.interest_rate,
                    liquidation_type: Default::default(),
                };
                let loan_id = loan.id;
                <Loans<T>>::insert(loan_id, loan.clone());
                <LoanIdsByAccountId<T>>::append_or_insert(&loaner, <[_]>::into_vec(box [loan_id]));
                <AliveLoanIdsByAccountId<T>>::append_or_insert(
                    &loaner,
                    <[_]>::into_vec(box [loan_id]),
                );
                let lenders = <AccountIdsWithLiveLoans<T>>::get();
                if !lenders.contains(&loaner) {
                    <AccountIdsWithLiveLoans<T>>::append_or_put(<[_]>::into_vec(box [
                        loaner.clone()
                    ]));
                }
                <generic_asset::Module<T>>::unreserve(
                    &borrow.collateral_asset_id,
                    &borrow.who,
                    collateral_balance,
                    Some(borrow.lock_id),
                )?;
                <generic_asset::Module<T>>::make_transfer_with_event(
                    &borrow.collateral_asset_id,
                    &borrow.who,
                    &<MoneyPool<T>>::get(),
                    collateral_balance,
                )?;
                <generic_asset::Module<T>>::make_transfer_with_event(
                    &borrow.borrow_asset_id,
                    &loaner,
                    &borrow.who,
                    borrow.borrow_balance,
                )?;
                <Borrows<T>>::mutate(&borrow_id, |v| {
                    v.status = BorrowStatus::Taken;
                    v.loan_id = Some(loan_id);
                });
                Self::deposit_event(RawEvent::LoanCreated(loan));
                Ok(())
            }
        }
    }
    pub fn ltv_meet_liquidation(
        prices: &TradingPairPrices,
        borrow_balance: T::Balance,
        collateral_balance: T::Balance,
    ) -> bool {
        (<T::Balance as TryFrom<u64>>::try_from(prices.collateral_asset_price)
            .ok()
            .unwrap()
            * collateral_balance
            * LTV_SCALE.into())
            / (<T::Balance as TryFrom<u64>>::try_from(prices.borrow_asset_price)
                .ok()
                .unwrap()
                * borrow_balance)
            <= Self::liquidate_ltv().into()
    }
    pub fn ltv_meet_safty(
        prices: &TradingPairPrices,
        borrow_balance: T::Balance,
        collateral_balance: T::Balance,
    ) -> bool {
        (<T::Balance as TryFrom<u64>>::try_from(prices.collateral_asset_price)
            .ok()
            .unwrap()
            * collateral_balance
            * LTV_SCALE.into())
            / (<T::Balance as TryFrom<u64>>::try_from(prices.borrow_asset_price)
                .ok()
                .unwrap()
                * borrow_balance)
            >= Self::safe_ltv().into()
    }
    pub fn liquidate_loan(liquidator: T::AccountId, loan_id: LoanId) -> DispatchResult {
        let loan = <Loans<T>>::get(loan_id);
        {
            if !(loan.status == LoanHealth::Overdue
                || loan.status == LoanHealth::Well
                || loan.status == LoanHealth::ToBeLiquidated)
            {
                {
                    return Err(Error::<T>::ShouldNotBeLiquidated.into());
                };
            }
        };
        let trading_pair_prices =
            Self::fetch_trading_pair_prices(loan.loan_asset_id, loan.collateral_asset_id)
                .ok_or(Error::<T>::TradingPairPriceMissing)?;
        if loan.status != LoanHealth::Overdue {
            {
                if !Self::ltv_meet_liquidation(
                    &trading_pair_prices,
                    loan.loan_balance,
                    loan.collateral_balance,
                ) {
                    {
                        return Err(Error::<T>::LTVNotMeet.into());
                    };
                }
            };
        }
        let borrow = <Borrows<T>>::get(loan.borrow_id);
        let expected_interest = Self::calculate_expected_interest(
            borrow.interest_rate,
            borrow.terms,
            borrow.borrow_balance,
        );
        let need_to_pay = loan.loan_balance + expected_interest;
        let collateral_in_borrow_asset_balance =
            <T::Balance as TryFrom<u64>>::try_from(trading_pair_prices.collateral_asset_price)
                .ok()
                .unwrap()
                * loan.collateral_balance
                / <T::Balance as TryFrom<u64>>::try_from(trading_pair_prices.borrow_asset_price)
                    .ok()
                    .unwrap();
        match loan.liquidation_type {
            LiquidationType::SellCollateral => {
                {
                    if !(<generic_asset::Module<T>>::free_balance(
                        &borrow.borrow_asset_id,
                        &liquidator,
                    ) >= need_to_pay)
                    {
                        {
                            return Err(Error::<T>::NotEnoughBalance.into());
                        };
                    }
                };
            }
            LiquidationType::JustCollateral => {
                if need_to_pay >= collateral_in_borrow_asset_balance {
                    let balance_to_loaner = loan.collateral_balance * 95.into() / 100.into();
                    let balance_to_liquidator = loan.collateral_balance - balance_to_loaner;
                    <generic_asset::Module<T>>::make_transfer_with_event(
                        &loan.collateral_asset_id,
                        &Self::money_pool(),
                        &loan.loaner_id,
                        balance_to_loaner,
                    )?;
                    <generic_asset::Module<T>>::make_transfer_with_event(
                        &loan.collateral_asset_id,
                        &Self::money_pool(),
                        &liquidator,
                        balance_to_liquidator,
                    )
                    .or_else(|err| -> DispatchResult {
                        <generic_asset::Module<T>>::make_transfer_with_event(
                            &loan.collateral_asset_id,
                            &loan.loaner_id,
                            &Self::money_pool(),
                            balance_to_loaner,
                        )?;
                        Err(err)
                    })?;
                } else {
                    let balance_to_loaner = loan.collateral_balance * 9.into() / 10.into();
                    let balance_to_liquidator =
                        (loan.collateral_balance - balance_to_loaner) / 2.into();
                    let balance_to_platform =
                        loan.collateral_balance - balance_to_loaner - balance_to_liquidator;
                    <generic_asset::Module<T>>::make_transfer_with_event(
                        &loan.collateral_asset_id,
                        &Self::money_pool(),
                        &loan.loaner_id,
                        balance_to_loaner,
                    )?;
                    <generic_asset::Module<T>>::make_transfer_with_event(
                        &loan.collateral_asset_id,
                        &Self::money_pool(),
                        &liquidator,
                        balance_to_liquidator,
                    )
                    .or_else(|err| -> DispatchResult {
                        <generic_asset::Module<T>>::make_transfer_with_event(
                            &loan.collateral_asset_id,
                            &loan.loaner_id,
                            &Self::money_pool(),
                            balance_to_loaner,
                        )?;
                        Err(err)
                    })?;
                    <generic_asset::Module<T>>::make_transfer_with_event(
                        &loan.collateral_asset_id,
                        &Self::money_pool(),
                        &Self::platform(),
                        balance_to_platform,
                    )
                    .or_else(|err| -> DispatchResult {
                        <generic_asset::Module<T>>::make_transfer_with_event(
                            &loan.collateral_asset_id,
                            &liquidator,
                            &Self::money_pool(),
                            balance_to_liquidator,
                        )?;
                        <generic_asset::Module<T>>::make_transfer_with_event(
                            &loan.collateral_asset_id,
                            &loan.loaner_id,
                            &Self::money_pool(),
                            balance_to_loaner,
                        )?;
                        Err(err)
                    })?;
                }
                Self::liquidation_cleanup(loan);
                Self::deposit_event(RawEvent::LoanLiquidated(loan_id));
            }
        }
        Ok(())
    }
    fn repay_cleanup(
        borrow: Borrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>,
        loan: Loan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>,
    ) {
        <Borrows<T>>::mutate(loan.borrow_id, |v| {
            v.status = BorrowStatus::Completed;
        });
        AliveBorrowIds::mutate(|v| {
            *v = v
                .clone()
                .into_iter()
                .filter(|id| *id != loan.borrow_id)
                .collect::<Vec<_>>();
        });
        <AliveLoanIdsByAccountId<T>>::mutate(&loan.loaner_id, |v| {
            *v = v
                .clone()
                .into_iter()
                .filter(|id| *id != loan.id)
                .collect::<Vec<_>>();
        });
        if <AliveLoanIdsByAccountId<T>>::get(&loan.loaner_id).len() == 0 {
            <AccountIdsWithLiveLoans<T>>::mutate(|v| {
                *v = v
                    .clone()
                    .into_iter()
                    .filter(|id| *id != loan.loaner_id)
                    .collect::<Vec<_>>();
            });
        }
        <Loans<T>>::mutate(loan.id, |v| {
            v.status = LoanHealth::Completed;
        });
    }
    fn liquidation_cleanup(loan: Loan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>) {
        <Borrows<T>>::mutate(loan.borrow_id, |v| {
            v.status = BorrowStatus::Liquidated;
        });
        AliveBorrowIds::mutate(|v| {
            *v = v
                .clone()
                .into_iter()
                .filter(|id| *id != loan.borrow_id)
                .collect::<Vec<_>>();
        });
        <AliveLoanIdsByAccountId<T>>::mutate(&loan.loaner_id, |v| {
            *v = v
                .clone()
                .into_iter()
                .filter(|id| *id != loan.id)
                .collect::<Vec<_>>();
        });
        if <AliveLoanIdsByAccountId<T>>::get(&loan.loaner_id).len() == 0 {
            <AccountIdsWithLiveLoans<T>>::mutate(|v| {
                *v = v
                    .clone()
                    .into_iter()
                    .filter(|id| *id != loan.loaner_id)
                    .collect::<Vec<T::AccountId>>();
            });
        }
        <Loans<T>>::mutate(loan.id, |v| {
            v.status = LoanHealth::Liquidated;
        });
    }
    pub fn is_trading_pair_allowed(trading_pair: &TradingPair<T::AssetId>) -> bool {
        <TradingPairs<T>>::get().contains(trading_pair)
    }
    pub fn ensure_borrow_available(
        borrow_id: BorrowId,
    ) -> Result<Borrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>, DispatchError> {
        {
            if !AliveBorrowIds::get().contains(&borrow_id) {
                {
                    return Err(Error::<T>::BorrowNotAlive.into());
                };
            }
        };
        let block_number = <system::Module<T>>::block_number();
        let borrow = <Borrows<T>>::get(borrow_id);
        if borrow.dead_after.is_some() && borrow.dead_after.unwrap() <= block_number {
            <Borrows<T>>::mutate(borrow_id, |v| {
                v.status = BorrowStatus::Dead;
            });
            let new_alives = AliveBorrowIds::take()
                .into_iter()
                .filter(|v| *v != borrow_id)
                .collect::<Vec<_>>();
            AliveBorrowIds::put(new_alives);
            return Err(Error::<T>::BorrowNotAlive.into());
        }
        if borrow.status != BorrowStatus::Alive {
            return Err(Error::<T>::BorrowNotAlive.into());
        }
        Ok(borrow)
    }
    /// this will go through all borrows currently alive,
    /// mark those who have reached the end of lives to be dead.
    pub fn periodic_check_borrows(block_number: T::BlockNumber) {
        let mut new_alives: Vec<BorrowId> = Vec::new();
        AliveBorrowIds::take().into_iter().for_each(|borrow_id| {
            let borrow = <Borrows<T>>::get(borrow_id);
            if borrow.dead_after.is_some() && borrow.dead_after.unwrap() <= block_number {
                <Borrows<T>>::mutate(borrow_id, |v| {
                    v.status = BorrowStatus::Dead;
                });
                Self::deposit_event(RawEvent::BorrowDied(borrow_id.clone()));
            } else {
                new_alives.push(borrow_id.clone());
            }
        });
        AliveBorrowIds::put(new_alives);
    }
    /// this will go through all loans currently alive,
    /// calculate ltv instantly and mark loans 'ToBeLiquidated' if any whos ltv is below LTVLiquidate.
    pub fn periodic_check_loans(block_number: T::BlockNumber) {
        let account_ids = <AccountIdsWithLiveLoans<T>>::get();
        for account_id in account_ids {
            let loan_ids = <AliveLoanIdsByAccountId<T>>::get(account_id);
            for loan_id in loan_ids {
                let mut loan = <Loans<T>>::get(&loan_id);
                let trading_pair_prices =
                    Self::fetch_trading_pair_prices(loan.loan_asset_id, loan.collateral_asset_id);
                if trading_pair_prices.is_none() {
                    continue;
                } else {
                    let trading_pair_prices = trading_pair_prices.unwrap();
                    if Self::ltv_meet_liquidation(
                        &trading_pair_prices,
                        loan.loan_balance,
                        loan.collateral_balance,
                    ) {
                        loan.status = LoanHealth::ToBeLiquidated;
                        <Loans<T>>::insert(&loan_id, loan);
                        Self::deposit_event(RawEvent::LoanToBeLiquidated(loan_id.clone()));
                    } else if block_number > loan.due {
                        loan.status = LoanHealth::Overdue;
                        <Loans<T>>::insert(&loan_id, loan);
                        Self::deposit_event(RawEvent::LoanOverdue(loan_id.clone()));
                    }
                }
            }
        }
    }
    fn fetch_price(asset_id: T::AssetId) -> Option<u64> {
        if !<generic_asset::Module<T>>::asset_id_exists(asset_id) {
            return None;
        }
        let token = <generic_asset::Module<T>>::symbols(asset_id);
        if !<new_oracle::Module<T>>::is_token_known(&token) {
            return None;
        }
        let current_price = <new_oracle::Module<T>>::current_price(&token);
        let price: u64 = TryInto::<u64>::try_into(current_price).unwrap_or(0);
        if price == 0 {
            return None;
        } else {
            return Some(price);
        }
    }
}
