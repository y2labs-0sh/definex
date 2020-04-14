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

//! **deposit-loan** is an implementation of Financial market protocol that
//! provides both liquid money markets for cross-chain assets and capital markets
//! for longer-term cryptocurrency  loans.
//!
//! ## How it works
//!
//! + It will automatically adjust the interest rates based on the amount saved and the amount borrowed.
//!
//! + We are working on a three-level interest rate based on cash utilization rate that is
//! partially influenced by the economic pricing for scarce resources and our belief that the
//! demand for stable coin is relatively inelastic in different utilization rate intervals.
//! The exact loan interest rate is yet to be determined but it would look like this :
//!
//!   `f(x) = 0.1x + 0.05 （0≤x＜0.4）|| 0.2x + 0.01 (0.4≤x<0.8) || 0.3x^6 + 0.1x^3 + 0.06 (0.8≤x≤1)`
//!
//!    In which, Utilization rate X = Total borrows / (Total deposits + Total Borrows)
//!
//! + Each time when a block is issued, the interest generated in that interval will be calculated
//! based on the last time interest was calculated versus the current time interval versus realtime
//! interest,  and the interest is transferred to collection_account. At the same time, based on the
//! price of the collateralized asset, it is calculated whether any loan has reached the liquidation
//! threshold and those loans will be marked as liquidation status.
//!
//! Here is a simple way to calculate Compound interest within every block without calculate each account.
//! The initial value of token is set as 1. When a user depoist some money, he will get some dtoken:
//!    dtoken_user_will_get = deposit_amount / value_of_token
//!    total_deposit += deposit_amount
//! When interest is deposited, the value of token will be calculated as:
//!    value_of_token = value_of_token * interest_amount / total_deposit
//!    total_deposit += interest_amount
//!
//! Simply example will be show here:
//!     At the begining User_A deposit 100 usdt, the price of token is 1; so User_A will get 100 dtoken.
//!     After some time, 3 usdt interest generated, so the price of token will be: (100 + 3)/100 = 1.03.
//!     That is, if User_A want to redeem all money, he will get: `100 dtoken * 1.03 value_of_dtoken = 103 usdt`
//!     Then, User_B deposit 50 usdt, he will get `50 usdt / 1.03 value_of_dtoken` dtoken;
//!     After some time, 10 usdt interest generated, the value of token will be: `1.03 * (1 + 10 / 153)`
//!     If User_A want to redeem all now, he will get: `100 dtoken * 1.03 * (1 + 10 / 153)` usdt
//!     User_B will get: `50 usdt / 1.03 * 1.03 * (1 + 10 / 153)` usdt
//!     As for the 10 usdt interest:
//!     `User_A get:User_B get == 103:50 == (100 * 1.03 * (1 + 10 / 153) - 103):(50 / 1.03 * 1.03 * (1 + 10 / 153) - 50)`
//!

#![cfg_attr(not(feature = "std"), no_std)]

#[allow(unused_imports)]
use codec::{Decode, Encode, Error as CodecErr, HasCompact, Input, Output};

#[allow(unused_imports)]
use sp_std::{
    self, cmp,
    collections::btree_map,
    convert::{TryFrom, TryInto},
    fmt::Debug,
    prelude::*,
    result, vec,
};

#[allow(unused_imports)]
use sp_runtime::traits::{
    AtLeast32Bit, Bounded, CheckedAdd, CheckedMul, CheckedSub, MaybeDisplay,
    MaybeSerializeDeserialize, Member, One, Saturating, Zero,
};

#[allow(unused_imports)]
use sp_runtime::{DispatchError, DispatchResult, RuntimeDebug};

#[allow(unused_imports)]
use support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::Parameter,
    ensure,
    weights::{SimpleDispatchInfo, WeighData, Weight},
    IterableStorageMap,
};

#[allow(unused_imports)]
use frame_system::{self as system, ensure_root, ensure_signed};

mod mock;
mod tests;

pub use deposit_loan_primitives::*;

pub trait Trait:
    frame_system::Trait + timestamp::Trait + generic_asset::Trait + new_oracle::Trait
{
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Saving {

        /// module level switch
        Paused get(paused) : bool = false;

        /// the asset that user saves into our program
        CollectionAssetId get(collection_asset_id) config() : T::AssetId;

        /// the account where user saves go and it can be either a normal account which held by or a totally random account
        /// probably need to be supervised by the public
        CollectionAccountId get(collection_account_id) build(|config: &GenesisConfig<T>| {
            config.collection_account_id.clone()
        }) : T::AccountId;

        /// User will get dtoken when make saving
        /// This will be used to calculate the amount when redeem.
        // pub UserDtoken get(user_dtoken) : linked_map hasher(blake2_256) T::AccountId => T::Balance;
        pub UserDtoken get(user_dtoken) : map hasher(opaque_blake2_256) T::AccountId => T::Balance;

        // used to calculate interest rate, default accuracy 1_0000_0000
        pub ValueOfTokens get(value_of_tokens) config(): T::Balance;

        /// time of last distribution of interest
        BonusTime get(bonus_time) : T::Moment;

        /// Annualized interest rate of loan
        pub LoanInterestRateCurrent get(loan_interest_rate_current) config(): T::Balance;

        /// use "ProfitAsset" for bonus
        ProfitAssetId get(profit_asset_id) config() : T::AssetId;

        /// use a specific account as "ProfitPool"
        /// might be supervised by the public
        ProfitPool get(profit_pool) config() : T::AccountId;

        /// the account that user makes loans from, (and assets are all burnt from this account by design)
        PawnShop get(pawn_shop) config() : T::AccountId;

        /// the asset that user uses as collateral when making loans
        CollateralAssetId get(collateral_asset_id) config() : T::AssetId;

        /// the asset that defi
        LoanAssetId get(loan_asset_id) config() : T::AssetId;

        /// the maximum LTV that a loan package can be set initially
        pub GlobalLTVLimit get(global_ltv_limit) config() : LTV;

        /// when a loan's LTV reaches or is above this threshold, this loan must be been liquidating
        pub GlobalLiquidationThreshold get(global_liquidation_threshold) config() : LTV;

        /// when a loan's LTV reaches or is above this threshold, a warning event will be fired and there should be a centralized system monitoring on this
        pub GlobalWarningThreshold get(global_warning_threshold) config() : LTV;

        /// increase monotonically
        NextLoanId get(next_loan_id) config() : LoanId;

        /// currently running loans
        pub Loans get(get_loan_by_id) : map hasher(twox_64_concat) LoanId => Loan<T::AccountId, T::Balance>;

        /// all loans id
        pub LoanIdWithAllLoans get(loan_id_with_all_loans) : Vec<LoanId>;

        /// loan id aggregated by account
        pub LoansByAccount get(loans_by_account) : map hasher(opaque_blake2_256) T::AccountId => Vec<LoanId>;

        /// store account_id for loans
        pub AccountIdsWithLiveLoans get(account_ids_with_loans) : Vec<T::AccountId>;

        /// total balance of loan asset in circulation
        pub TotalLoan get(total_loan) : T::Balance;

        /// total balance of collateral asset locked in the pawnshop
        pub TotalCollateral get(total_collateral) : T::Balance;

        /// when a loan is overdue, a small portion of its collateral will be cut as penalty
        pub PenaltyRate get(penalty_rate) config() : u32;

        /// the official account take charge of selling the collateral asset of liquidating loans
        LiquidationAccount get(liquidation_account) config() : T::AccountId;

        /// loans which are in liquidating, these loans will not be in "Loans" & "LoansByAccount"
        pub LiquidatingLoans get(liquidating_loans) : Vec<LoanId>;

        /// a global cap of loan balance, no caps at all if None
        pub LoanCap get(loan_cap) : Option<T::Balance>;

        /// for each loan, the amount of collateral asset must be greater than this
        pub MinimumCollateral get(minimum_collateral) config() : T::Balance;

        pub LiquidationPenalty get(liquidation_penalty) config() : u32;

        pub SavingInterestRate get(saving_interest_rate) config() : T::Balance;
    }

    add_extra_genesis {
        config(collection_account_id): T::AccountId;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;
        type Error = Error<T>;

        fn on_initialize(height: T::BlockNumber) -> Weight {
            if !Self::paused() {
                Self::on_each_block(height);
            }
            SimpleDispatchInfo::default().weigh_data(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn pause(origin) -> DispatchResult {
            ensure_root(origin)?;
            Paused::mutate(|v| *v = true);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn resume(origin) -> DispatchResult {
            ensure_root(origin)?;
            Paused::mutate(|v| *v = false);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn set_collection_asset_id(origin, asset_id: T::AssetId) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(<generic_asset::Module<T>>::asset_id_exists(asset_id), "invalid collection asset id");
            <CollectionAssetId<T>>::put(asset_id);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn set_collection_account(origin, account_id: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            <CollectionAccountId<T>>::put(account_id.clone());
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn set_collateral_asset_id(origin, asset_id: T::AssetId) -> LoanResult {
            ensure_root(origin)?;
            <CollateralAssetId<T>>::put(asset_id);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn set_global_ltv_limit(origin, limit: LTV) -> LoanResult {
            ensure_root(origin)?;
            GlobalLTVLimit::put(limit);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn set_loan_asset_id(origin, asset_id: T::AssetId) -> LoanResult {
            ensure_root(origin)?;
            <LoanAssetId<T>>::put(asset_id);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn set_global_liquidation_threshold(origin, threshold: LTV) -> LoanResult {
            ensure_root(origin)?;
            GlobalWarningThreshold::put(threshold);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn set_global_warning_threshold(origin, threshold: LTV) -> LoanResult {
            ensure_root(origin)?;
            GlobalLiquidationThreshold::put(threshold);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn set_loan_cap(origin, balance: T::Balance) -> LoanResult {
            ensure_root(origin)?;
            if balance.is_zero() {
                <LoanCap<T>>::kill();
            } else {
                <LoanCap<T>>::put(balance);
            }
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn set_liquidation_account(origin, account_id: T::AccountId) -> LoanResult {
            ensure_root(origin)?;
            <LiquidationAccount<T>>::put(account_id);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn set_profit_asset_id(origin, asset_id: T::AssetId) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(<generic_asset::Module<T>>::asset_id_exists(asset_id), "invalid collection asset id");
            <ProfitAssetId<T>>::put(asset_id);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn set_profit_pool(origin, account_id: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            <ProfitPool<T>>::put(account_id);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn set_penalty_rate(origin, rate: u32) -> LoanResult {
            ensure_root(origin)?;
            PenaltyRate::put(rate);
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn staking(origin, asset_id: T::AssetId, amount: T::Balance) -> DispatchResult {
            ensure!(!Self::paused(), Error::<T>::Paused);
            let who = ensure_signed(origin)?;
            ensure!(<CollectionAssetId<T>>::get() == asset_id, Error::<T>::SavingTypeNotAllowed);
            ensure!(<generic_asset::Module<T>>::free_balance(&asset_id, &who) >= amount, Error::<T>::NotEnoughBalance);
            Self::create_staking(who.clone(), asset_id, amount)?;
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FixedNormal(0)]
        pub fn redeem(origin, iou_asset_id: T::AssetId, iou_asset_amount: T::Balance) -> DispatchResult {
            ensure!(!Self::paused(), Error::<T>::Paused);
            let who = ensure_signed(origin)?;
            let collection_asset_id = Self::collection_asset_id();
            let collection_account_id = Self::collection_account_id();
            ensure!(<generic_asset::Module<T>>::free_balance(&collection_asset_id, &collection_account_id) >= iou_asset_amount, Error::<T>::NotEnoughBalance);
            ensure!(collection_asset_id == iou_asset_id, Error::<T>::UnknowAssetId);

            Self::make_redeem(
                &who,
                &collection_asset_id,
                &collection_account_id,
                iou_asset_amount,
            )?;
            Ok(())
        }

        /// a user can apply for a loan choosing one active loan package, providing the collateral and loan amount he wants,
        #[weight = SimpleDispatchInfo::FixedNormal(10)]
        pub fn apply_loan(origin, collateral_amount: T::Balance, loan_amount: T::Balance) -> LoanResult {
            ensure!(!Self::paused(), Error::<T>::Paused);
            let who = ensure_signed(origin)?;
            Self::apply_for_loan(who.clone(), collateral_amount, loan_amount)
        }

        /// a user repay a loan he has made before, by providing the loan id and he should make sure there is enough related assets in his account
        #[weight = SimpleDispatchInfo::FixedNormal(10)]
        pub fn repay_loan(origin, loan_id: LoanId) -> LoanResult {
            ensure!(!Self::paused(), Error::<T>::Paused);
            let who = ensure_signed(origin)?;
            ensure!(<Loans<T>>::contains_key(loan_id), Error::<T>::UnknownLoanId);
            let loan = <Loans<T>>::get(loan_id);
            ensure!(loan.who == who, Error::<T>::NotLoanOwner);
            Self::repay_for_loan(who.clone(), loan_id)
        }

        /// when a liquidating loan has been handled well, platform mananger should call "mark_liquidated" to update the chain
        /// loan id is the loan been handled and auction_balance is what the liquidation got by selling the collateral asset
        /// auction_balance will be first used to make up the loan, then what so ever left will be returned to the loan's owner account
        #[weight = SimpleDispatchInfo::FixedNormal(10)]
        pub fn mark_liquidated(origin, loan_id: LoanId, auction_balance: T::Balance) -> DispatchResult {
            ensure!(!Self::paused(), Error::<T>::Paused);
            let liquidation_account = ensure_signed(origin)?;
            // ensure!(liquidation_account == Self::liquidation_account(), "liquidation account only");
            ensure!(<Loans<T>>::contains_key(loan_id), Error::<T>::UnknownLoanId);

            Self::mark_loan_liquidated(&Self::get_loan_by_id(loan_id), liquidation_account, auction_balance)
        }

        /// when user got a warning of high-risk LTV, user can lower the LTV by add more collateral
        #[weight = SimpleDispatchInfo::FixedNormal(10)]
        pub fn add_collateral(origin, loan_id: LoanId, amount: T::Balance) -> DispatchResult {
            ensure!(!Self::paused(), Error::<T>::Paused);
            ensure_signed(origin)?;
            ensure!(<Loans<T>>::contains_key(loan_id), Error::<T>::UnknownLoanId);
            let loan = Self::get_loan_by_id(loan_id);
            // ensure!(who == loan.who, "adding collateral to other's loan is not allowed");

            Self::add_loan_collateral(&loan, loan.who.clone(), amount)
        }

        /// as long as the LTV of this loan is below the "GlobalLTVLimit", user can keep drawing TBD from this loan
        #[weight = SimpleDispatchInfo::FixedNormal(10)]
        pub fn draw(origin, loan_id: LoanId, amount: T::Balance) -> DispatchResult {
            ensure!(!Self::paused(), Error::<T>::Paused);
            let who = ensure_signed(origin)?;
            let loan = Self::get_loan_by_id(loan_id);
            ensure!(loan.who == who, Error::<T>::NotLoanOwner);
            Self::draw_from_loan(who, loan_id, amount)
        }
    }
}

impl<T: Trait> Module<T> {
    /// immutable for RPC
    pub fn get_loans(
        size: Option<u64>,
        offset: Option<u64>,
    ) -> Option<Vec<Loan<T::AccountId, T::Balance>>> {
        let offset = offset.unwrap_or(0);
        let size = size.unwrap_or(10);
        let mut res = Vec::with_capacity(size as usize);

        for (_, l) in <Loans<T>>::iter().skip(offset as usize).take(size as usize) {
            res.push(l);
        }

        if res.len() > 0 {
            Some(res)
        } else {
            None
        }
    }

    pub fn create_staking(
        who: T::AccountId,
        asset_id: T::AssetId,
        amount: T::Balance,
    ) -> DispatchResult {
        ensure!(!amount.is_zero(), Error::<T>::SavingIsZero);

        let collection_account_id = Self::collection_account_id();
        let value_of_tokens = Self::value_of_tokens();

        <generic_asset::Module<T>>::make_transfer_with_event(
            &asset_id,
            &who,
            &collection_account_id,
            amount,
        )?;

        // TODO: front end have multipled 10^8 alreadly
        let user_dtoken = <T::Balance as TryFrom<u128>>::try_from(10_u128.pow(8))
            .ok()
            .unwrap()
            * amount
            / value_of_tokens;

        // in case This user is the second deposit of the user
        if <UserDtoken<T>>::contains_key(who.clone()) {
            <UserDtoken<T>>::mutate(who.clone(), |v| {
                *v = v.checked_add(&user_dtoken).expect("overflow!");
            });
        } else {
            <UserDtoken<T>>::insert(&who, user_dtoken);
        }

        Ok(())
    }

    fn make_redeem(
        who: &T::AccountId,
        collection_asset_id: &T::AssetId,
        collection_account_id: &T::AccountId,
        amount: T::Balance,
    ) -> DispatchResult {
        let user_dtoken_amount = Self::user_dtoken(&who);
        let value_of_tokens = Self::value_of_tokens();

        // let user_will_get = user_dtoken_amount / (market_dtoken_amount / total_dtoken_amount);
        let user_will_get = user_dtoken_amount * value_of_tokens
            / <T::Balance as TryFrom<u128>>::try_from(10_u128.pow(8))
                .ok()
                .unwrap();

        ensure!(user_will_get >= amount, Error::<T>::NotEnoughBalance);

        // money user will get / money user have == dtoken will cut / dtoken user have
        let dtoken_will_cut = amount
            * <T::Balance as TryFrom<u128>>::try_from(10_u128.pow(8))
                .ok()
                .unwrap()
            / value_of_tokens;

        // TODO: if user deposit all saving, can delete this saving.

        <UserDtoken<T>>::mutate(who.clone(), |v| {
            *v -= dtoken_will_cut;
        });

        <generic_asset::Module<T>>::make_transfer_with_event(
            &collection_asset_id,
            &collection_account_id,
            &who,
            amount,
        )?;

        Ok(())
    }

    fn apply_for_loan(
        who: T::AccountId,
        collateral_amount: T::Balance,
        loan_amount: T::Balance,
    ) -> DispatchResult {
        let collection_asset_id = Self::collection_asset_id();
        let collateral_asset_id = Self::collateral_asset_id();
        let collection_account_id = Self::collection_account_id();

        ensure!(
            <generic_asset::Module<T>>::free_balance(&collection_asset_id, &collection_account_id)
                >= loan_amount,
            Error::<T>::NotEnoughBalance
        );

        let price_pair = Self::fetch_trading_pair_prices(collection_asset_id, collateral_asset_id)
            .ok_or(Error::<T>::TradingPairPriceMissing)?;

        // collateral asset will be transfered to this shop
        let shop = <PawnShop<T>>::get();
        let loan_cap = <LoanCap<T>>::get();
        let total_loan = <TotalLoan<T>>::get();

        if loan_cap.is_some() && total_loan >= loan_cap.unwrap() {
            return Err(Error::<T>::ReachLoanCap)?;
        }

        match Self::get_collateral_loan(
            collateral_amount,
            collateral_asset_id,
            loan_amount,
            collection_asset_id,
        ) {
            Err(err) => Err(err),
            Ok(CollateralLoan {
                collateral_amount: actual_collateral_amount,
                loan_amount: actual_loan_amount,
            }) => {
                ensure!(
                    collateral_amount >= Self::minimum_collateral(),
                    Error::<T>::MinCollateralAmount
                );

                let loan_id = Self::get_next_loan_id();

                let price_pair_borrow_asset_price =
                    <T::Balance as TryFrom<u128>>::try_from(price_pair.borrow_asset_price as u128)
                        .ok()
                        .unwrap();

                let collateral_balance_available = actual_collateral_amount
                    - loan_amount * price_pair_borrow_asset_price
                        / <T::Balance as TryFrom<u128>>::try_from(
                            price_pair.collateral_asset_price as u128,
                        )
                        .ok()
                        .unwrap();

                // transfer collateral to pawnshop
                <generic_asset::Module<T>>::make_transfer_with_event(
                    &collateral_asset_id,
                    &who,
                    &shop,
                    actual_collateral_amount,
                )?;

                <generic_asset::Module<T>>::make_transfer_with_event(
                    &collection_asset_id,
                    &collection_account_id,
                    &who,
                    loan_amount,
                )?;

                let loan = Loan {
                    id: loan_id,
                    who: who.clone(),
                    collateral_balance_original: actual_collateral_amount,
                    collateral_balance_available: collateral_balance_available,
                    loan_balance_total: actual_loan_amount,
                    status: Default::default(),
                };

                <Loans<T>>::insert(loan_id, loan.clone());
                <LoansByAccount<T>>::mutate(&who, |v| {
                    v.push(loan_id);
                });

                LoanIdWithAllLoans::append_or_put(vec![loan_id.clone()]);

                if ! <AccountIdsWithLiveLoans<T>>::get().contains(&who) {
                    <AccountIdsWithLiveLoans<T>>::append_or_put(vec![who.clone()]);
                }

                <TotalLoan<T>>::mutate(|v| *v += actual_loan_amount);
                <TotalCollateral<T>>::mutate(|v| *v += actual_collateral_amount);

                Self::deposit_event(RawEvent::LoanCreated(loan));
                Ok(())
            }
        }
    }

    pub fn get_collateral_loan(
        collateral_amount: T::Balance,
        collateral_asset_id: T::AssetId,
        loan_amount: T::Balance,
        loan_asset_id: T::AssetId,
    ) -> Result<CollateralLoan<T::Balance>, DispatchError> {
        if collateral_amount.is_zero() && loan_amount.is_zero() {
            return Err(Error::<T>::InvalidCollateralLoanAmounts)?;
        }

        let price_pair = Self::fetch_trading_pair_prices(loan_asset_id, collateral_asset_id)
            .ok_or(Error::<T>::TradingPairPriceMissing)?;

        let price_prec_in_balance = T::Balance::from(PRICE_PREC);
        let ltv_prec_in_balance = T::Balance::from(LTV_PREC);

        let ltv = GlobalLTVLimit::get();
        let ltv_in_balance = <T::Balance as TryFrom<u64>>::try_from(ltv).ok().unwrap();

        let price_pair_collateral_asset_price =
            <T::Balance as TryFrom<u128>>::try_from(price_pair.collateral_asset_price as u128)
                .ok()
                .unwrap();
        let price_pair_borrow_asset_price =
            <T::Balance as TryFrom<u128>>::try_from(price_pair.borrow_asset_price as u128)
                .ok()
                .unwrap();

        if collateral_amount.is_zero() {
            let must_collateral_amount = loan_amount
                * ltv_prec_in_balance
                * price_prec_in_balance
                * price_pair_borrow_asset_price
                / (price_pair_collateral_asset_price * ltv_in_balance);

            return Ok(CollateralLoan {
                collateral_amount: must_collateral_amount,
                loan_amount: loan_amount,
            });
        }

        if loan_amount.is_zero() {
            let can_loan_amount =
                (collateral_amount * price_pair_collateral_asset_price * ltv_in_balance)
                    / (ltv_prec_in_balance * price_prec_in_balance * price_pair_borrow_asset_price);
            return Ok(CollateralLoan {
                collateral_amount: collateral_amount,
                loan_amount: can_loan_amount,
            });
        }

        if (loan_amount * ltv_prec_in_balance * price_pair_borrow_asset_price)
            * price_prec_in_balance
            / (collateral_amount * price_pair_collateral_asset_price)
            >= ltv_in_balance
        {
            Err(Error::<T>::OverLTVLimit)?
        } else {
            Ok(CollateralLoan {
                collateral_amount,
                loan_amount,
            })
        }
    }

    pub fn repay_for_loan(who: T::AccountId, loan_id: LoanId) -> DispatchResult {
        let loan_asset_id = Self::loan_asset_id();
        let collateral_asset_id = Self::collateral_asset_id();
        let collection_account_id = Self::collection_account_id();
        let pawn_shop = Self::pawn_shop();
        let loan = <Loans<T>>::get(loan_id);

        ensure!(
            <generic_asset::Module<T>>::free_balance(&loan_asset_id, &who)
                >= loan.loan_balance_total,
            Error::<T>::NotEnoughBalance
        );
        ensure!(
            <generic_asset::Module<T>>::free_balance(&collateral_asset_id, &pawn_shop)
                >= loan.collateral_balance_available,
            Error::<T>::NotEnoughBalance
        );
        ensure!(
            !Self::check_loan_in_liquidation(&loan_id),
            Error::<T>::LoanInLiquidation
        );

        <LoansByAccount<T>>::mutate(&who, |v| {
            *v = v
                .clone()
                .into_iter()
                .filter(|ele| *ele != loan_id)
                .collect::<Vec<LoanId>>();
        });
        <Loans<T>>::remove(&loan.id);

        LoanIdWithAllLoans::mutate(|v| {
            *v = v
                .clone()
                .into_iter()
                .filter(|v| *v != loan.id)
                .collect::<Vec<_>>();
        });

        if <LoansByAccount<T>>::get(&loan.who).len() == 0 {
            <AccountIdsWithLiveLoans<T>>::mutate(|v| {
                *v = v
                    .clone()
                    .into_iter()
                    .filter(|v| *v != loan.who)
                    .collect::<Vec<_>>();
            });
        }

        let revert_callback = || {
            <Loans<T>>::insert(&loan.id, &loan);
            <LoansByAccount<T>>::mutate(&who, |v| {
                v.push(loan.id);
            });
            <TotalLoan<T>>::mutate(|v| *v += loan.loan_balance_total);
            <TotalCollateral<T>>::mutate(|v| *v += loan.collateral_balance_available);
        };

        <generic_asset::Module<T>>::make_transfer_with_event(
            &loan_asset_id,
            &who,
            &collection_account_id,
            loan.loan_balance_total,
        )
        .or_else(|err| -> DispatchResult {
            revert_callback();
            Err(err)
        })?;
        <generic_asset::Module<T>>::make_transfer_with_event(
            &collateral_asset_id,
            &pawn_shop,
            &who,
            loan.collateral_balance_original,
        )
        .or_else(|err| -> DispatchResult {
            revert_callback();
            <generic_asset::Module<T>>::make_transfer_with_event(
                &loan_asset_id,
                &collection_account_id,
                &who,
                loan.loan_balance_total,
            )?;
            Err(err)
        })?;

        <Loans<T>>::remove(&loan.id);
        <TotalLoan<T>>::mutate(|v| *v -= loan.loan_balance_total);
        // <TotalCollateral<T>>::mutate(|v| *v -= loan.collateral_balance_available);
        <TotalCollateral<T>>::mutate(|v| *v -= loan.collateral_balance_original);

        Self::deposit_event(RawEvent::LoanRepaid(
            loan_id,
            loan.loan_balance_total,
            loan.collateral_balance_available,
        ));
        Ok(())
    }

    fn check_loan_in_liquidation(loan_id: &LoanId) -> bool {
        LiquidatingLoans::get().contains(loan_id)
    }

    pub fn mark_loan_liquidated(
        loan: &Loan<T::AccountId, T::Balance>,
        liquidation_account: T::AccountId,
        auction_balance: T::Balance,
    ) -> DispatchResult {
        let pawnshop = Self::pawn_shop();
        let collateral_asset_id = Self::collateral_asset_id();
        let collection_account_id = Self::collection_account_id();
        let loan_asset_id = Self::loan_asset_id();

        ensure!(
            Self::check_loan_in_liquidation(&loan.id),
            Error::<T>::LoanNotInLiquidation
        );

        ensure!(
            <generic_asset::Module<T>>::free_balance(&loan_asset_id, &liquidation_account)
                >= auction_balance,
            Error::<T>::NotEnoughBalance
        );

        ensure!(
            auction_balance >= loan.loan_balance_total,
            Error::<T>::NotEnoughBalance
        );

        <generic_asset::Module<T>>::make_transfer_with_event(
            &loan_asset_id,
            &liquidation_account,
            &collection_account_id,
            loan.loan_balance_total,
        )?;

        let leftover = auction_balance.checked_sub(&loan.loan_balance_total);

        if leftover.is_some() && leftover.unwrap() > T::Balance::zero() {
            let penalty_rate = Self::liquidation_penalty();
            let penalty = leftover.unwrap() * T::Balance::from(penalty_rate) / 100.into();

            <generic_asset::Module<T>>::make_transfer_with_event(
                &loan_asset_id,
                &collection_account_id,
                &Self::profit_pool(), // TODO: can change to team account
                penalty,
            )
            .or_else(|err| -> DispatchResult {
                <generic_asset::Module<T>>::make_transfer_with_event(
                    &loan_asset_id,
                    &pawnshop,
                    &liquidation_account,
                    loan.loan_balance_total,
                )?;
                Err(err)
            })?;
            // part of the penalty will transfer to the loan owner
            <generic_asset::Module<T>>::make_transfer_with_event(
                &loan_asset_id,
                &collection_account_id,
                &loan.who,
                leftover.unwrap() - penalty,
            )
            .or_else(|err| -> DispatchResult {
                <generic_asset::Module<T>>::make_transfer_with_event(
                    &loan_asset_id,
                    &Self::profit_pool(),
                    &liquidation_account,
                    penalty,
                )?;

                // TODO: ensure pawnshop have enough collateral_asset
                <generic_asset::Module<T>>::make_transfer_with_event(
                    &collateral_asset_id,
                    &pawnshop,
                    &liquidation_account,
                    loan.collateral_balance_original,
                )?;
                Err(err)
            })?;
        }
        <Loans<T>>::remove(&loan.id);

        LoanIdWithAllLoans::mutate(|v| {
            *v = v
                .clone()
                .into_iter()
                .filter(|v| *v != loan.id)
                .collect::<Vec<_>>();
        });

        <LoansByAccount<T>>::mutate(&loan.who, |v| {
            *v = v
                .clone()
                .into_iter()
                .filter(|ele| ele != &loan.id)
                .collect::<Vec<LoanId>>();
        });

        if <LoansByAccount<T>>::get(&loan.who).len() == 1 {
            <AccountIdsWithLiveLoans<T>>::mutate(|v| {
                *v = v
                    .clone()
                    .into_iter()
                    .filter(|v| *v != loan.who)
                    .collect::<Vec<_>>();
            });
        }

        LiquidatingLoans::mutate(|v| {
            *v = v
                .clone()
                .into_iter()
                .filter(|ele| ele != &loan.id)
                .collect::<Vec<LoanId>>();
        });
        Self::deposit_event(RawEvent::Liquidated(
            loan.id,
            loan.collateral_balance_original,
            loan.collateral_balance_available,
            auction_balance,
            loan.loan_balance_total,
        ));

        Ok(())
    }

    pub fn add_loan_collateral(
        loan: &Loan<T::AccountId, T::Balance>,
        from: T::AccountId,
        amount: T::Balance,
    ) -> DispatchResult {
        let pawnshop = Self::pawn_shop();
        let collateral_asset_id = Self::collateral_asset_id();

        ensure!(
            <generic_asset::Module<T>>::free_balance(&collateral_asset_id, &from) >= amount,
            Error::<T>::NotEnoughBalance
        );

        <generic_asset::Module<T>>::make_transfer_with_event(
            &collateral_asset_id,
            &from,
            &pawnshop,
            amount,
        )?;

        <Loans<T>>::mutate(loan.id, |l| {
            l.collateral_balance_original =
                l.collateral_balance_original.checked_add(&amount).unwrap();
            l.collateral_balance_available =
                l.collateral_balance_available.checked_add(&amount).unwrap();
        });

        <TotalCollateral<T>>::mutate(|c| {
            *c += amount;
        });

        Self::deposit_event(RawEvent::AddCollateral(loan.id, amount));

        Ok(())
    }

    fn check_loan_health(
        loan: &Loan<T::AccountId, T::Balance>,
        collection_asset_price: u64,
        collateral_asset_price: u64,
        liquidation: LTV,
        warning: LTV,
    ) -> LoanHealth {
        let current_ltv = <Loan<T::AccountId, T::Balance>>::get_ltv(
            loan.collateral_balance_available,
            loan.loan_balance_total,
            collection_asset_price,
            collateral_asset_price,
        );

        if current_ltv >= liquidation {
            return LoanHealth::Liquidating(current_ltv);
        }

        if current_ltv >= warning {
            return LoanHealth::Warning(current_ltv);
        }

        LoanHealth::Well
    }

    fn liquidate_loan(loan_id: LoanId, liquidating_ltv: LTV) {
        <Loans<T>>::mutate(loan_id, |v| {
            v.status = LoanHealth::Liquidating(liquidating_ltv)
        });
        if LiquidatingLoans::exists() {
            LiquidatingLoans::mutate(|v| v.push(loan_id));
        } else {
            let ll: Vec<LoanId> = vec![loan_id];
            LiquidatingLoans::put(ll);
        }
    }

    pub fn draw_from_loan(
        who: T::AccountId,
        loan_id: LoanId,
        amount: T::Balance,
    ) -> DispatchResult {
        let loan = Self::get_loan_by_id(loan_id);
        ensure!(<Loans<T>>::contains_key(loan_id), Error::<T>::UnknownLoanId);
        ensure!(loan.who == who, Error::<T>::NotLoanOwner);

        let collateral_asset_id = Self::collateral_asset_id();
        let collection_asset_id = Self::collection_asset_id();
        let collection_account_id = Self::collection_account_id();

        ensure!(
            <generic_asset::Module<T>>::free_balance(&collection_asset_id, &collection_account_id)
                >= amount,
            Error::<T>::NotEnoughBalance
        );

        let price_pair = Self::fetch_trading_pair_prices(collection_asset_id, collateral_asset_id)
            .ok_or(Error::<T>::TradingPairPriceMissing)?;

        let price_pair_borrow_asset_price =
            <T::Balance as TryFrom<u128>>::try_from(price_pair.borrow_asset_price as u128)
                .ok()
                .unwrap();

        let price_pair_collateral_asset_price =
            <T::Balance as TryFrom<u128>>::try_from(price_pair.collateral_asset_price as u128)
                .ok()
                .unwrap();

        let global_ltv = Self::global_ltv_limit();
        let available_credit = loan.collateral_balance_available
            * price_pair_collateral_asset_price
            * T::Balance::from(global_ltv as u32)
            / T::Balance::from(LTV_PREC * PRICE_PREC)
            / price_pair_borrow_asset_price;

        ensure!(amount <= available_credit, Error::<T>::NotEnoughBalance);

        <Loans<T>>::mutate(loan_id, |v| {
            v.loan_balance_total = v.loan_balance_total + amount;
            v.collateral_balance_available = v.collateral_balance_available
                - amount * price_pair_borrow_asset_price / price_pair_collateral_asset_price;
        });

        <generic_asset::Module<T>>::make_transfer_with_event(
            &collection_asset_id,
            &collection_account_id,
            &who,
            amount,
        )?;

        <TotalLoan<T>>::mutate(|v| *v += amount);

        Self::deposit_event(RawEvent::LoanDrawn(loan_id, amount));

        Ok(())
    }

    fn _pause(linum: u32) {
        Paused::mutate(|v| {
            *v = true;
        });
        Self::deposit_event(RawEvent::Paused(
            linum,
            <frame_system::Module<T>>::block_number(),
            <frame_system::Module<T>>::extrinsic_index().unwrap(),
        ));
    }

    fn on_each_block(_height: T::BlockNumber) {
        let collateral_asset_id = Self::collateral_asset_id();
        let liquidation_thd = Self::global_liquidation_threshold();
        let warning_thd = Self::global_warning_threshold();
        let collection_asset_id = Self::collection_asset_id();

        let price_pair = Self::fetch_trading_pair_prices(collection_asset_id, collateral_asset_id);

        if price_pair.is_none() {
            return;
        }

        let price_pair = price_pair.unwrap();

        let all_loans = <LoanIdWithAllLoans>::get();

        for loan_id in all_loans {
            let loan = <Loans<T>>::get(&loan_id);
            // for (loan_id, loan) in <Loans<T>>::enumerate() {
            if Self::check_loan_in_liquidation(&loan_id) {
                continue;
            }

            match Self::check_loan_health(
                &loan,
                price_pair.borrow_asset_price,
                price_pair.collateral_asset_price,
                liquidation_thd,
                warning_thd,
            ) {
                LoanHealth::Well => {}
                LoanHealth::Warning(ltv) => {
                    if loan.status != LoanHealth::Warning(ltv) {
                        <Loans<T>>::mutate(&loan.id, |v| v.status = LoanHealth::Warning(ltv));
                        Self::deposit_event(RawEvent::Warning(loan_id, ltv));
                    }
                }

                LoanHealth::Liquidating(l) => {
                    Self::liquidate_loan(loan_id, l);
                    Self::deposit_event(RawEvent::Liquidating(
                        loan_id,
                        loan.who.clone(),
                        loan.collateral_balance_available,
                        loan.loan_balance_total,
                    ));
                }
            }
        }
        Self::calculate_loan_interest_rate();
    }

    fn calculate_loan_interest_rate() {
        let collection_asset_id = Self::collection_asset_id();
        let collection_account_id = Self::collection_account_id();
        let total_loan = Self::total_loan();

        let total_deposit =
            <generic_asset::Module<T>>::free_balance(&collection_asset_id, &collection_account_id)
                + Self::total_loan();

        let last_bonus_time: T::Moment = Self::bonus_time();
        let current_time = <timestamp::Module<T>>::get();
        <BonusTime<T>>::put(current_time);

        // if !(total_deposit + total_loan).is_zero() {
        if total_deposit > T::Balance::from(0) && total_loan > T::Balance::from(0) {
            let current_loan_interest_rate = Self::current_loan_interest_rate();
            let time_duration = TryInto::<u32>::try_into(current_time - last_bonus_time)
                .ok()
                .unwrap();

            // after 1500s, ValueOfTokens will change.
            // TODO: uncomment next line while doing testcase
            // let time_duration = TryInto::<u32>::try_into(1500).ok().unwrap();

            let interest_generated =
                T::Balance::from(time_duration) * total_loan * current_loan_interest_rate
                    / T::Balance::from(SEC_PER_DAY)
                    / T::Balance::from(DAYS_PER_YEAR)
                    / T::Balance::from(1_0000_0000);

            let all_loans = <LoanIdWithAllLoans>::get();
            for loan_id in all_loans {
                let loan = <Loans<T>>::get(&loan_id);

                let amount = interest_generated * loan.loan_balance_total / total_loan;

                Self::draw_from_loan(loan.who.clone(), loan_id, amount).unwrap_or_default();

                <generic_asset::Module<T>>::make_transfer_with_event(
                    &collection_asset_id,
                    &loan.who,
                    &collection_account_id,
                    amount,
                )
                .unwrap_or_default();
            }

            let value_of_tokens = Self::value_of_tokens();

            <ValueOfTokens<T>>::put(
                value_of_tokens * (total_deposit + interest_generated) / total_deposit,
            );

            <LoanInterestRateCurrent<T>>::put(current_loan_interest_rate);

            let current_saving_interest_rate = Self::current_saving_interest_rate();

            <SavingInterestRate<T>>::put(current_saving_interest_rate);
        }
    }

    // Obtain current annualized loan interest rate
    #[rustfmt::skip]
    fn current_loan_interest_rate() -> T::Balance {

        let collection_asset_id = Self::collection_asset_id();
        let collection_account_id = Self::collection_account_id();

        let total_deposit = <generic_asset::Module<T>>::free_balance(&collection_asset_id, &collection_account_id)
                + Self::total_loan();
        let total_loan = Self::total_loan();

        let mut loan_interest_rate_current = T::Balance::from(0);

        if !(total_deposit + total_loan).is_zero() {

            let utilization_rate_x: T::Balance = total_loan.checked_mul(&T::Balance::from(10_u32.pow(8))).expect("saving share overflow")
                / (total_deposit + total_loan);

            // This is the real interest rate * 10^8
            loan_interest_rate_current = if utilization_rate_x < T::Balance::from(4000_0000) {
                (utilization_rate_x + T::Balance::from(5000_0000)) / T::Balance::from(10)
            } else if utilization_rate_x >= T::Balance::from(8000_0000) {
                let utilization_rate_x_pow3 = utilization_rate_x * utilization_rate_x * utilization_rate_x;
                let utilization_rate_x_pow6 = utilization_rate_x_pow3 * utilization_rate_x_pow3;
                (utilization_rate_x_pow6 * 30.into() +  utilization_rate_x_pow3 * T::Balance::from(10_u32.pow(25)) + T::Balance::from(6) * <T::Balance as TryFrom<u128>>::try_from(10_u128.pow(48)).ok().unwrap())
                / <T::Balance as TryFrom<u128>>::try_from(10_u128.pow(42)).ok().unwrap()
            } else {
                (T::Balance::from(20) * utilization_rate_x + T::Balance::from(1_0000_0000)) / T::Balance::from(100)
            };
        }
        loan_interest_rate_current
    }

    // Obtain current annualized saving interest rate
    fn current_saving_interest_rate() -> T::Balance {
        let collection_asset_id = Self::collection_asset_id();
        let collection_account_id = Self::collection_account_id();
        let total_deposit =
            <generic_asset::Module<T>>::free_balance(&collection_asset_id, &collection_account_id)
                + Self::total_loan();

        // Calculate deposit interest: deposit interest rate = borrowing interest * total borrowing / total deposit
        let current_saving_interest_rate =
            Self::current_loan_interest_rate() * Self::total_loan() / total_deposit;
        current_saving_interest_rate
    }

    fn get_next_loan_id() -> LoanId {
        NextLoanId::mutate(|v| {
            let org = *v;
            *v += 1;
            org
        })
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

decl_error! {
    pub enum Error for Module<T: Trait> {
        Paused,
        NotEnoughBalance,
        SavingTypeNotAllowed,
        UnknowAssetId,
        TradingPairPriceMissing,
        MinCollateralAmount,
        UnknownLoanId,
        NotLoanOwner,
        LoanInLiquidation,
        LoanNotInLiquidation,
        TotalCollateralUnderflow,
        ReachLoanCap,
        InvalidCollateralLoanAmounts,
        OverLTVLimit,
        SavingIsZero,
    }
}

decl_event!(
    #[rustfmt::skip]
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        Balance = <T as generic_asset::Trait>::Balance,
        Loan = Loan<<T as frame_system::Trait>::AccountId, <T as generic_asset::Trait>::Balance>,
        CollateralBalanceOriginal = <T as generic_asset::Trait>::Balance,
        CollateralBalanceAvailable = <T as generic_asset::Trait>::Balance,
        AuctionBalance = <T as generic_asset::Trait>::Balance,
        TotalLoanBalance = <T as generic_asset::Trait>::Balance,
        LineNumber = u32,
        BlockNumber = <T as frame_system::Trait>::BlockNumber,
        ExtrinsicIndex = u32,
    {
        LoanCreated(Loan),
        LoanDrawn(LoanId, Balance),
        LoanRepaid(LoanId, Balance, Balance),
        // Expired(LoanId, AccountId, Balance, Balance),
        // Extended(LoanId, AccountId),
        Warning(LoanId, LTV),
        Paused(LineNumber, BlockNumber, ExtrinsicIndex),

        Liquidating(LoanId, AccountId, CollateralBalanceAvailable, TotalLoanBalance),
        Liquidated(
            LoanId,
            CollateralBalanceOriginal,
            CollateralBalanceAvailable,
            AuctionBalance,
            TotalLoanBalance
        ),

        AddCollateral(LoanId, Balance),
    }
);
