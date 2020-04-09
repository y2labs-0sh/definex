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

//! Basically, there are 3 major roles:
//!     1. maker: those who want to borrow money. they can publish their needs (collateral amount, borrow amount, how long they will repay, a specific interest rate, etc.) on the platform.
//!     2. taker: those who bring liquidity to the platform. they select the borrows that most profitable, and lend the money to the borrower. By doing this, they earn the negotiated interest.
//!     3. liquidator: those who keep monitoring if there is any loan with a ltv lower than the 'LTVLiquidate'. By doing this, they would be rewarded.
//!
//!

#![cfg_attr(not(feature = "std"), no_std)]

#[allow(unused_imports)]
use codec::{Decode, Encode, Error as codecErr, HasCompact, Input, Output};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use sp_runtime::{
    traits::{
        AtLeast32Bit, Bounded, CheckedAdd, CheckedMul, CheckedSub, MaybeDisplay,
        MaybeSerializeDeserialize, Member, One, Saturating, SignedExtension, Zero,
    },
    transaction_validity::{
        InvalidTransaction, TransactionPriority, TransactionValidity, TransactionValidityError,
        ValidTransaction,
    },
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
    dispatch::{DispatchError, DispatchResult, Dispatchable, Parameter},
    ensure,
    traits::{
        Contains, Currency, Get, Imbalance, LockIdentifier, LockableCurrency, ReservableCurrency,
        WithdrawReason, WithdrawReasons,
    },
    weights::{DispatchInfo, SimpleDispatchInfo},
    IsSubType, IterableStorageMap,
};
#[allow(unused_imports)]
use system::{ensure_root, ensure_signed};

pub use p2p_primitives::*;

mod mock;
mod tests;

const LOCK_ID: LockIdentifier = *b"dfxlsbrw";

pub const INTEREST_RATE_PRECISION: u64 = 10000_0000;
pub const LTV_SCALE: u32 = 10000;

/// The module's configuration trait.
pub trait Trait:
    generic_asset::Trait + timestamp::Trait + system::Trait + new_oracle::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Call: Parameter
        + Dispatchable<Origin = <Self as system::Trait>::Origin>
        + IsSubType<Module<Self>, Self>;
    type Days: Get<Self::BlockNumber>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as P2p {
        /// module level switch
        pub Paused get(paused) : bool = false;
        /// hold borrowers' collateral temporarily
        pub MoneyPool get(money_pool) config() : T::AccountId;
        /// Platform is just a account receiving potential fees
        pub Platform get(platform) config() : T::AccountId;
        /// TradingPairs contains all supported trading pairs, oracle should provide price information for all trading pairs.
        pub TradingPairs get(trading_pairs) config() : Vec<TradingPair<T::AssetId>>;
        /// LTV must be greater than this value to create a new borrow
        pub SafeLTV get(safe_ltv) config() : u32;
        /// a loan will be liquidated when LTV is below this
        pub LiquidateLTV get(liquidate_ltv) config() : u32;
        /// minimium borrow terms, count in natural days
        pub MinBorrowTerms get(min_borrow_terms) config() : u64; // days of our lives
        /// minimium interest rate
        pub MinBorrowInterestRate get(min_borrow_interest_rate) config() : u64;
        /// borrow id counter
        pub NextBorrowId get(next_borrow_id) : P2PBorrowId = 1;
        /// loan id counter
        pub NextLoanId get(next_loan_id) : P2PLoanId = 1;

        /// an account can only have one alive borrow at a time
        pub Borrows get(borrows) : map hasher(twox_64_concat) P2PBorrowId => P2PBorrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>;
        pub BorrowIdsByAccountId get(borrow_ids_by_account_id) : map hasher(opaque_blake2_256) T::AccountId => Vec<P2PBorrowId>;
        pub AliveBorrowIds get(alive_borrow_ids) : Vec<P2PBorrowId>;

        /// on the other hand, an account can have multiple alive loans
        pub Loans get(loans) : map hasher(twox_64_concat) P2PLoanId => P2PLoan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>;
        pub LoanIdsByAccountId get(loan_ids_by_account_id) : map hasher(opaque_blake2_256) T::AccountId => Vec<P2PLoanId>;
        pub AliveLoanIdsByAccountId get(alive_loan_ids_by_account_id) : map hasher(opaque_blake2_256) T::AccountId => Vec<P2PLoanId>;
        pub AccountIdsWithLiveLoans get(account_ids_with_loans) : Vec<T::AccountId>;
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
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
        AddCollateralNotAllowed,
        FailToReserve,
    }
}

// The module's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        const LTV_SCALE: u32 = LTV_SCALE;
        const INTEREST_SCALE: u64 = INTEREST_RATE_PRECISION;

        type Error = Error<T>;

        fn deposit_event() = default;

        fn on_finalize(block_number: T::BlockNumber) {
            if (block_number % 2.into()).is_zero() && !((block_number + 1.into()) % 5.into()).is_zero() {
                Self::periodic_check_borrows(block_number);
            }
            if ((block_number + 1.into()) % 5.into()).is_zero()  {
                Self::periodic_check_loans(block_number);
            }
        }

        #[weight = SimpleDispatchInfo::MaxOperational]
        pub fn pause(origin) -> DispatchResult {
            ensure_root(origin)?;
            Paused::mutate(|v| *v = true);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::MaxOperational]
        pub fn resume(origin) -> DispatchResult {
            ensure_root(origin)?;
            Paused::mutate(|v| *v = false);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::MaxOperational]
        pub fn change_platform(origin, platform: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            <Platform<T>>::put(platform);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::MaxOperational]
        pub fn change_money_pool(origin, pool: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            <MoneyPool<T>>::put(pool);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::MaxOperational]
        pub fn change_safe_ltv(origin, ltv: u32) -> DispatchResult {
            ensure_root(origin)?;
            SafeLTV::put(ltv);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::MaxOperational]
        pub fn change_liquidate_ltv(origin, ltv: u32) -> DispatchResult {
            ensure_root(origin)?;
            LiquidateLTV::put(ltv);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::MaxOperational]
        pub fn change_min_borrow_terms(origin, t: u64) -> DispatchResult {
            ensure_root(origin)?;
            MinBorrowTerms::put(t);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::MaxOperational]
        pub fn change_min_borrow_interest_rate(origin, r: u64) -> DispatchResult {
            ensure_root(origin)?;
            MinBorrowInterestRate::put(r);
            Ok(())
        }

        /// a borrower place a make order to ask for some money
        #[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
        pub fn make(origin, collateral_balance: T::Balance, trading_pair: TradingPair<T::AssetId>, borrow_options: P2PBorrowOptions<T::Balance,T::BlockNumber>) -> DispatchResult {
            ensure!(!Self::paused(), Error::<T>::Paused);
            let who = ensure_signed(origin)?;
            Self::create_borrow(who, collateral_balance, trading_pair, borrow_options)
        }

        /// the owner of a make order is allowed to cancel this order before someone takes it
        #[weight = SimpleDispatchInfo::FixedNormal(500_000)]
        pub fn cancel(origin, borrow_id: P2PBorrowId) -> DispatchResult {
            ensure!(!Self::paused(), Error::<T>::Paused);
            let who = ensure_signed(origin)?;
            Self::remove_borrow(who, borrow_id)
        }

        /// a lender sees a make order profitable, takes it and lends the amount of money to the borrower
        #[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
        pub fn take(origin, borrow_id: P2PBorrowId) -> DispatchResult {
            ensure!(!Self::paused(), Error::<T>::Paused);
            let who = ensure_signed(origin)?;
            Self::create_loan(who, borrow_id)
        }

        /// anyone can liquidate a loan if the loan meets the liquidation requirements
        #[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
        pub fn liquidate(origin, loan_id: P2PLoanId) -> DispatchResult {
            ensure!(!Self::paused(), Error::<T>::Paused);
            let who = ensure_signed(origin)?;
            Self::liquidate_loan(who, loan_id)
        }

        /// the borrower of a loan can add additional collaterals to lower the risk of being liquidated
        #[weight = SimpleDispatchInfo::FixedNormal(500_000)]
        pub fn add(origin, borrow_id: P2PBorrowId, amount: T::Balance) -> DispatchResult {
            ensure!(!Self::paused(), Error::<T>::Paused);
            let who = ensure_signed(origin)?;
            Self::add_collateral(who, borrow_id, amount)
        }

        /// before due, the borrower returns what he borrowed and pays fee
        #[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
        pub fn repay(origin, borrow_id: P2PBorrowId) -> DispatchResult {
            ensure!(!Self::paused(), Error::<T>::Paused);
            let who = ensure_signed(origin)?;
            Self::repay_loan(who, borrow_id)
        }
    }
}

decl_event!(
    #[rustfmt::skip]
    pub enum Event<T>
    where
        // AccountId = <T as system::Trait>::AccountId,
        // Balance = <T as generic_asset::Trait>::Balance,
        P2PLoan = P2PLoan<<T as generic_asset::Trait>::AssetId, <T as generic_asset::Trait>::Balance, <T as system::Trait>::BlockNumber, <T as system::Trait>::AccountId>,
        P2PBorrow = P2PBorrow<<T as generic_asset::Trait>::AssetId, <T as generic_asset::Trait>::Balance, <T as system::Trait>::BlockNumber, <T as system::Trait>::AccountId>,
    {
        CheckingAliveBorrows,
        CheckingAliveLoans,
        CheckingAliveBorrowsDone,
        CheckingAliveLoansDone,

        BorrowCreated(P2PBorrow),
        BorrowCanceled(P2PBorrowId),
        LoanCreated(P2PLoan),
        LoanLiquidated(P2PLoanId),
        LoanRepaid(P2PLoanId),
        CollateralAdded(P2PBorrowId),

        // issue when the current block number is greater than the dead_after of a borrow
        BorrowDied(P2PBorrowId),

        // issue when the current block number is greater than the due of a loan
        LoanOverdue(P2PLoanId),

        // issue when status of a loan changed from P2PLoanHealth::Well to P2PLoanHealth::ToBeLiquidated
        LoanToBeLiquidated(P2PLoanId),
    }
);

impl<T: Trait> Module<T> {
    /// immutable for RPC

    /// reverse the alive borrow list
    pub fn get_alive_borrows(
        size: Option<u64>,
        offset: Option<u64>,
    ) -> Vec<P2PBorrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>> {
        let offset = offset.unwrap_or(0) as usize;
        let size = size.unwrap_or(10) as usize;
        let mut res = Vec::with_capacity(size);
        let alive_borrow_ids = AliveBorrowIds::get();

        for i in alive_borrow_ids.iter().rev().skip(offset).take(size) {
            res.push(<Borrows<T>>::get(i));
        }

        res
    }

    /// reverse the user's borrow list,
    pub fn get_user_borrows(
        who: T::AccountId,
        size: Option<u64>,
        offset: Option<u64>,
    ) -> Vec<P2PBorrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>> {
        let offset = offset.unwrap_or(0) as usize;
        let size = size.unwrap_or(10) as usize;
        let mut res = Vec::with_capacity(size);
        let account_borrow_ids = <BorrowIdsByAccountId<T>>::get(&who);

        for i in account_borrow_ids.iter().rev().skip(offset).take(size) {
            res.push(<Borrows<T>>::get(i));
        }

        res
    }

    /// the alive loan list
    pub fn get_alive_loans(
        size: Option<u64>,
        offset: Option<u64>,
    ) -> Vec<P2PLoan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>> {
        let offset = offset.unwrap_or(0) as usize;
        let size = size.unwrap_or(10) as usize;
        let mut res = Vec::with_capacity(size);

        for account_id in <AccountIdsWithLiveLoans<T>>::get() {
            for id in <AliveLoanIdsByAccountId<T>>::get(&account_id) {
                res.push(<Loans<T>>::get(&id));
            }
        }

        res.into_iter().skip(offset).take(size).collect()
    }

    /// the user's loan list with no rev
    pub fn get_user_loans(
        who: T::AccountId,
        size: Option<u64>,
        offset: Option<u64>,
    ) -> Vec<P2PLoan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>> {
        let offset = offset.unwrap_or(0) as usize;
        let size = size.unwrap_or(10) as usize;
        let mut res = Vec::with_capacity(size);
        let account_loan_ids = <LoanIdsByAccountId<T>>::get(&who);
        for i in account_loan_ids.iter().skip(offset).take(size) {
            res.push(<Loans<T>>::get(i));
        }
        res
    }

    /// get_borrows will just iterate over <Borrows>, and since IterableStorageMap doesn't provide rev()
    /// we can only go through the list from the very begin.
    pub fn get_borrows(
        size: Option<u64>,
        offset: Option<u64>,
    ) -> Vec<P2PBorrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>> {
        let offset = offset.unwrap_or(0);
        let size = size.unwrap_or(10);
        let mut res = Vec::with_capacity(size as usize);

        for (_, b) in <Borrows<T>>::iter()
            .skip(offset as usize)
            .take(size as usize)
        {
            res.push(b);
        }

        res
    }

    /// get_loans will just iterate over <Loans>, and since IterableStorageMap doesn't provide rev()
    /// we can only go through the list from the very begin.
    pub fn get_loans(
        size: Option<u64>,
        offset: Option<u64>,
    ) -> Vec<P2PLoan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>> {
        let offset = offset.unwrap_or(0);
        let size = size.unwrap_or(10);
        let mut res = Vec::with_capacity(size as usize);

        for (_, l) in <Loans<T>>::iter().skip(offset as usize).take(size as usize) {
            res.push(l);
        }

        res
    }

    fn generate_borrow_id() -> P2PBorrowId {
        let id = Self::next_borrow_id();
        NextBorrowId::mutate(|v| *v += 1);
        id
    }

    fn generate_loan_id() -> P2PLoanId {
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
        borrow_id: P2PBorrowId,
        amount: T::Balance,
    ) -> DispatchResult {
        ensure!(
            <Borrows<T>>::contains_key(borrow_id),
            Error::<T>::UnknownBorrowId
        );
        let borrow = <Borrows<T>>::get(borrow_id);

        // different borrow status
        match borrow.status {
            P2PBorrowStatus::Alive => {
                // alive borrows will just reserve user's collateral asset
                // so we just move the addition into the reserved and update the lock
                <generic_asset::Module<T>>::increase_reserved_balance(
                    &borrow.collateral_asset_id,
                    borrow.lock_id,
                    &who,
                    amount,
                )
                .or(Err(Error::<T>::FailToReserve))?;
                <Borrows<T>>::mutate(&borrow_id, |v| {
                    v.collateral_balance = v.collateral_balance.checked_add(&amount).unwrap();
                });
                Self::deposit_event(RawEvent::CollateralAdded(borrow_id));
                Ok(())
            }
            P2PBorrowStatus::Taken => {
                // after been taken, the collateral asset has been transfered into the money pool
                // so this addition should also go to the pool directly
                ensure!(
                    <generic_asset::Module<T>>::free_balance(&borrow.collateral_asset_id, &who)
                        >= amount,
                    Error::<T>::NotEnoughBalance
                );
                <generic_asset::Module<T>>::make_transfer_with_event(
                    &borrow.collateral_asset_id,
                    &who,
                    &<MoneyPool<T>>::get(),
                    amount,
                )?;
                <Borrows<T>>::mutate(&borrow_id, |v| {
                    v.collateral_balance = v.collateral_balance.checked_add(&amount).unwrap();
                });
                <Loans<T>>::mutate(borrow.loan_id.unwrap(), |v| {
                    v.collateral_balance = v.collateral_balance.checked_add(&amount).unwrap();
                });
                Self::deposit_event(RawEvent::CollateralAdded(borrow_id));
                Ok(())
            }
            _ => Err(Error::<T>::AddCollateralNotAllowed.into()),
        }
    }

    pub fn repay_loan(who: T::AccountId, borrow_id: P2PBorrowId) -> DispatchResult {
        ensure!(
            <Borrows<T>>::contains_key(borrow_id),
            Error::<T>::UnknownBorrowId
        );
        let borrow = <Borrows<T>>::get(borrow_id);
        ensure!(&borrow.who == &who, Error::<T>::NotOwnerOfBorrow);
        ensure!(borrow.loan_id.is_some(), Error::<T>::BorrowNotLoaned);

        let trading_pair_prices =
            Self::fetch_trading_pair_prices(borrow.borrow_asset_id, borrow.collateral_asset_id)
                .ok_or(Error::<T>::TradingPairPriceMissing)?;
        ensure!(
            <Loans<T>>::contains_key(borrow.loan_id.unwrap()),
            Error::<T>::UnknownLoanId
        );
        let loan_id = borrow.loan_id.unwrap();
        let loan = <Loans<T>>::get(loan_id);
        ensure!(loan.status == P2PLoanHealth::Well, Error::<T>::LoanNotWell);

        if Self::ltv_meet_liquidation(
            &trading_pair_prices,
            loan.loan_balance,
            loan.collateral_balance,
        ) {
            <Loans<T>>::mutate(&loan.id, |v| {
                v.status = P2PLoanHealth::ToBeLiquidated;
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
        // ensure borrower can afford the expected interest
        ensure!(
            <generic_asset::Module<T>>::free_balance(&borrow.borrow_asset_id, &who) >= need_to_pay,
            Error::<T>::NotEnoughBalance
        );

        // transfer borrowed assert + interest into loaner's account
        <generic_asset::Module<T>>::make_transfer_with_event(
            &borrow.borrow_asset_id,
            &who,
            &loan.loaner_id,
            need_to_pay,
        )?;
        // transfer former collateralized asset back into borrower's account
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
        borrow_options: P2PBorrowOptions<T::Balance, T::BlockNumber>,
    ) -> DispatchResult {
        ensure!(
            borrow_options.terms >= Self::min_borrow_terms(),
            Error::<T>::MinBorrowTerms
        );
        ensure!(
            borrow_options.interest_rate >= Self::min_borrow_interest_rate(),
            Error::<T>::MinBorrowInterestRate
        );
        ensure!(
            Self::is_trading_pair_allowed(&trading_pair),
            Error::<T>::TradingPairNotAllowed
        );
        // ensure one user can only have one borrow alive at a time
        if let Some(id) = Self::borrow_ids_by_account_id(&who).last() {
            ensure!(
                !Self::alive_borrow_ids().contains(id),
                Error::<T>::MultipleAliveBorrows
            );
        }
        // ensure essential price info is provided
        let trading_pair_prices =
            Self::fetch_trading_pair_prices(trading_pair.borrow, trading_pair.collateral)
                .ok_or(Error::<T>::TradingPairPriceMissing)?;

        // collateral - expected_interest meet safty ltv
        // let expected_interest = Self::calculate_expected_interest(
        //     borrow_options.interest_rate,
        //     borrow_options.terms,
        //     borrow_options.amount,
        // );

        ensure!(
            Self::ltv_meet_safty(
                &trading_pair_prices,
                borrow_options.amount,
                collateral_balance
            ),
            Error::<T>::InitialCollateralRateFail
        );

        let borrow_id = Self::generate_borrow_id();
        let lock_id = <generic_asset::Module<T>>::reserve(
            &trading_pair.collateral,
            &who,
            collateral_balance,
        )?;
        let b = P2PBorrow {
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
        AliveBorrowIds::append_or_put(vec![borrow_id.clone()]);
        <BorrowIdsByAccountId<T>>::append_or_insert(&who, vec![borrow_id.clone()]);

        Self::deposit_event(RawEvent::BorrowCreated(b));
        Ok(())
    }

    pub fn remove_borrow(who: T::AccountId, borrow_id: P2PBorrowId) -> DispatchResult {
        ensure!(
            <Borrows<T>>::contains_key(&borrow_id),
            Error::<T>::UnknownBorrowId
        );
        ensure!(
            <BorrowIdsByAccountId<T>>::get(&who).contains(&borrow_id),
            Error::<T>::NotOwnerOfBorrow
        );
        ensure!(
            AliveBorrowIds::get().contains(&borrow_id),
            Error::<T>::BorrowNotAlive
        );

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
        <Borrows<T>>::mutate(borrow_id, |v| {
            v.status = P2PBorrowStatus::Canceled;
        });

        Self::deposit_event(RawEvent::BorrowCanceled(borrow_id));
        Ok(())
    }

    pub fn create_loan(loaner: T::AccountId, borrow_id: P2PBorrowId) -> DispatchResult {
        let borrow = Self::ensure_borrow_available(borrow_id)?;

        // get collateral amount from locked balance
        // to make sure that amount of asset is indeed reserved
        let locked_balance = <generic_asset::Module<T>>::locked_balance(
            &borrow.collateral_asset_id,
            &borrow.who,
            borrow.lock_id,
        );
        match locked_balance {
            None => {
                debug::info!("no locked balance");
                return Err(Error::<T>::NoLockedBalance.into());
            }
            Some(collateral_balance) => {
                ensure!(
                    <generic_asset::Module<T>>::free_balance(&borrow.borrow_asset_id, &loaner)
                        >= borrow.borrow_balance,
                    Error::<T>::NotEnoughBalance
                );
                debug::info!("enough balance");

                let trading_pair_prices = Self::fetch_trading_pair_prices(
                    borrow.borrow_asset_id,
                    borrow.collateral_asset_id,
                )
                .ok_or(Error::<T>::TradingPairPriceMissing)?;
                ensure!(
                    Self::ltv_meet_safty(
                        &trading_pair_prices,
                        borrow.borrow_balance,
                        collateral_balance
                    ),
                    Error::<T>::InitialCollateralRateFail
                );
                debug::info!("meet init collateral rate");

                let current_block_number = <system::Module<T>>::block_number();

                // generate a loan
                let loan = P2PLoan {
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
                    status: P2PLoanHealth::Well,
                    interest_rate: borrow.interest_rate,
                    liquidation_type: Default::default(),
                };

                let loan_id = loan.id;
                <Loans<T>>::insert(loan_id, loan.clone());
                <LoanIdsByAccountId<T>>::append_or_insert(&loaner, vec![loan_id]);
                <AliveLoanIdsByAccountId<T>>::append_or_insert(&loaner, vec![loan_id]);

                let lenders = <AccountIdsWithLiveLoans<T>>::get();
                if !lenders.contains(&loaner) {
                    <AccountIdsWithLiveLoans<T>>::append_or_put(vec![loaner.clone()]);
                }

                // unreserve the locked balance
                <generic_asset::Module<T>>::unreserve(
                    &borrow.collateral_asset_id,
                    &borrow.who,
                    collateral_balance,
                    Some(borrow.lock_id),
                )?;
                // transfer the collateral balance into money pool
                <generic_asset::Module<T>>::make_transfer_with_event(
                    &borrow.collateral_asset_id,
                    &borrow.who,
                    &<MoneyPool<T>>::get(),
                    collateral_balance,
                )?;
                // transfer loan into borrower's account
                <generic_asset::Module<T>>::make_transfer_with_event(
                    &borrow.borrow_asset_id,
                    &loaner,
                    &borrow.who,
                    borrow.borrow_balance,
                )?;

                // mark borrow taken and save the borrow
                <Borrows<T>>::mutate(&borrow_id, |v| {
                    v.status = P2PBorrowStatus::Taken;
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

    pub fn liquidate_loan(liquidator: T::AccountId, loan_id: P2PLoanId) -> DispatchResult {
        let loan = <Loans<T>>::get(loan_id);
        ensure!(
            loan.status == P2PLoanHealth::Overdue
                || loan.status == P2PLoanHealth::Well
                || loan.status == P2PLoanHealth::ToBeLiquidated,
            Error::<T>::ShouldNotBeLiquidated
        );

        let trading_pair_prices =
            Self::fetch_trading_pair_prices(loan.loan_asset_id, loan.collateral_asset_id)
                .ok_or(Error::<T>::TradingPairPriceMissing)?;
        if loan.status != P2PLoanHealth::Overdue {
            ensure!(
                Self::ltv_meet_liquidation(
                    &trading_pair_prices,
                    loan.loan_balance,
                    loan.collateral_balance
                ),
                Error::<T>::LTVNotMeet
            );
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
                ensure!(
                    <generic_asset::Module<T>>::free_balance(&borrow.borrow_asset_id, &liquidator)
                        >= need_to_pay,
                    Error::<T>::NotEnoughBalance
                );
                // TODO:: exchange with liquidator
            }
            LiquidationType::JustCollateral => {
                if need_to_pay >= collateral_in_borrow_asset_balance {
                    // move 95% of collateral to loaner and give 5% to liquidator
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
                    // move 90% of collateral to loaner and give 5% to liquidator and 5% to platform
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
        borrow: P2PBorrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>,
        loan: P2PLoan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>,
    ) {
        <Borrows<T>>::mutate(loan.borrow_id, |v| {
            v.status = P2PBorrowStatus::Completed;
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
            v.status = P2PLoanHealth::Completed;
        });
    }

    // make sure all the internal states are consistent
    fn liquidation_cleanup(loan: P2PLoan<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>) {
        <Borrows<T>>::mutate(loan.borrow_id, |v| {
            v.status = P2PBorrowStatus::Liquidated;
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
            v.status = P2PLoanHealth::Liquidated;
        });
    }

    pub fn is_trading_pair_allowed(trading_pair: &TradingPair<T::AssetId>) -> bool {
        <TradingPairs<T>>::get().contains(trading_pair)
    }

    // when found a unavailable borrow, write the new borrow status
    pub fn ensure_borrow_available(
        borrow_id: P2PBorrowId,
    ) -> Result<P2PBorrow<T::AssetId, T::Balance, T::BlockNumber, T::AccountId>, DispatchError>
    {
        ensure!(
            AliveBorrowIds::get().contains(&borrow_id),
            Error::<T>::BorrowNotAlive
        );

        let block_number = <system::Module<T>>::block_number();
        let borrow = <Borrows<T>>::get(borrow_id);
        if borrow.dead_after.is_some() && borrow.dead_after.unwrap() <= block_number {
            <Borrows<T>>::mutate(borrow_id, |v| {
                v.status = P2PBorrowStatus::Dead;
            });
            let new_alives = AliveBorrowIds::take()
                .into_iter()
                .filter(|v| *v != borrow_id)
                .collect::<Vec<_>>();
            AliveBorrowIds::put(new_alives);

            return Err(Error::<T>::BorrowNotAlive.into());
        }

        if borrow.status != P2PBorrowStatus::Alive {
            return Err(Error::<T>::BorrowNotAlive.into());
        }

        Ok(borrow)
    }

    /// this will go through all borrows currently alive,
    /// mark those who have reached the end of lives to be dead.
    pub fn periodic_check_borrows(block_number: T::BlockNumber) {
        Self::deposit_event(RawEvent::CheckingAliveBorrows);

        // check alive borrows
        let mut new_alives: Vec<P2PBorrowId> = Vec::new();
        AliveBorrowIds::take().into_iter().for_each(|borrow_id| {
            let borrow = <Borrows<T>>::get(borrow_id);
            if borrow.dead_after.is_some() && borrow.dead_after.unwrap() <= block_number {
                <Borrows<T>>::mutate(borrow_id, |v| {
                    v.status = P2PBorrowStatus::Dead;
                });
                Self::deposit_event(RawEvent::BorrowDied(borrow_id.clone()));
            } else {
                new_alives.push(borrow_id.clone());
            }
        });
        AliveBorrowIds::put(new_alives);

        Self::deposit_event(RawEvent::CheckingAliveBorrowsDone);
    }

    /// this will go through all loans currently alive,
    /// calculate ltv instantly and mark loans 'ToBeLiquidated' if any whos ltv is below LTVLiquidate.
    pub fn periodic_check_loans(block_number: T::BlockNumber) {
        Self::deposit_event(RawEvent::CheckingAliveLoans);

        // check alive loans
        let account_ids = <AccountIdsWithLiveLoans<T>>::get();
        for account_id in account_ids {
            let loan_ids = <AliveLoanIdsByAccountId<T>>::get(account_id);
            for loan_id in loan_ids {
                let mut loan = <Loans<T>>::get(&loan_id);
                if loan.status == P2PLoanHealth::Well {
                    let trading_pair_prices = Self::fetch_trading_pair_prices(
                        loan.loan_asset_id,
                        loan.collateral_asset_id,
                    );
                    trading_pair_prices.map(|trading_pair_prices| {
                        if Self::ltv_meet_liquidation(
                            &trading_pair_prices,
                            loan.loan_balance,
                            loan.collateral_balance,
                        ) {
                            loan.status = P2PLoanHealth::ToBeLiquidated;
                            <Loans<T>>::insert(&loan_id, loan);
                            Self::deposit_event(RawEvent::LoanToBeLiquidated(loan_id.clone()));
                        } else if block_number > loan.due {
                            loan.status = P2PLoanHealth::Overdue;
                            <Loans<T>>::insert(&loan_id, loan);
                            Self::deposit_event(RawEvent::LoanOverdue(loan_id.clone()));
                        }
                    });
                }
            }
        }

        Self::deposit_event(RawEvent::CheckingAliveLoansDone);
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

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct P2PTxChecker<T: Trait + Send + Sync>(PhantomData<T>);

impl<T: Trait + Sync + Send> Default for P2PTxChecker<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Trait + Send + Sync> sp_std::fmt::Debug for P2PTxChecker<T> {
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(f, "P2PTxChecker")
    }
    #[cfg(not(feature = "std"))]
    fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        Ok(())
    }
}

impl<T: Trait + Send + Sync> SignedExtension for P2PTxChecker<T> {
    const IDENTIFIER: &'static str = "CheckP2PTxs";
    type AccountId = T::AccountId;
    type Call = <T as Trait>::Call;
    type AdditionalSigned = ();
    type DispatchInfo = DispatchInfo;
    type Pre = ();

    fn additional_signed(&self) -> sp_std::result::Result<(), TransactionValidityError> {
        Ok(())
    }

    fn validate(
        &self,
        who: &Self::AccountId,
        call: &Self::Call,
        _: Self::DispatchInfo,
        _: usize,
    ) -> TransactionValidity {
        let call = match call.is_sub_type() {
            Some(call) => call,
            None => return Ok(ValidTransaction::default()),
        };

        match call {
            Call::make(collateral_balance, trading_pair, borrow_options) => {
                if <generic_asset::Module<T>>::free_balance(&trading_pair.collateral, &who)
                    < *collateral_balance
                {
                    return InvalidTransaction::from(Error::<T>::NotEnoughBalance).into();
                }
                if !<Module<T>>::is_trading_pair_allowed(&trading_pair) {
                    return InvalidTransaction::from(Error::<T>::TradingPairNotAllowed).into();
                }
                if borrow_options.terms < <Module<T>>::min_borrow_terms() {
                    return InvalidTransaction::from(Error::<T>::MinBorrowTerms).into();
                }
                if borrow_options.interest_rate < <Module<T>>::min_borrow_interest_rate() {
                    return InvalidTransaction::from(Error::<T>::MinBorrowInterestRate).into();
                }

                let trading_pair_prices = <Module<T>>::fetch_trading_pair_prices(
                    trading_pair.borrow,
                    trading_pair.collateral,
                );
                match trading_pair_prices {
                    None => {
                        return InvalidTransaction::from(Error::<T>::TradingPairPriceMissing)
                            .into();
                    }
                    Some(tps) => {
                        if !<Module<T>>::ltv_meet_safty(
                            &tps,
                            borrow_options.amount,
                            *collateral_balance,
                        ) {
                            return InvalidTransaction::from(Error::<T>::InitialCollateralRateFail)
                                .into();
                        }
                    }
                }

                Ok(ValidTransaction::default())
            }
            _ => Ok(ValidTransaction::default()),
        }
    }
}

impl<T: Trait> From<Error<T>> for InvalidTransaction {
    fn from(e: Error<T>) -> Self {
        InvalidTransaction::Custom(e.as_u8())
    }
}
