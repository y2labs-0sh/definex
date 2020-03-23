## query

*** important query apis

api.query.lsBiding.money_pool() : AccountId

api.query.lsBiding.platform() : AccountId

api.query.lsBiding.borrows(BorrowId) : Borrow

api.query.lsBiding.borrow_ids_by_account_id(AccountId) : []BorrowId

api.query.lsBiding.alive_borrow_ids() : []BorrowId

api.query.lsBiding.loans(LoanId) : Loan

api.query.lsBiding.loan_ids_by_account_id(AccountId) : []LoanId

api.query.lsBiding.alive_loan_ids_by_account_id(AccountId) : []LoanId

api.query.lsBiding.account_ids_with_loans() : []AccountId

**

api.query.lsBiding.trading_pairs() : []TradingPair

api.query.lsBiding.safe_ltv() : u32

api.query.lsBiding.liquidate_ltv() : u32

api.query.lsBiding.min_borrow_terms() : u64

api.query.lsBiding.min_borrow_interest_rate() : u64

api.query.lsBiding.next_borrow_id() : BorrowId

api.query.lsBiding.next_loan_id() : LoanId


## extrinsics

#### publish a new borrow with the borrow options
api.tx.lsBiding.make(collateral_balance:Balance, trading_pair:TradingPair, borrow_options:BorrowOptions)

#### remove a borrow specified by borrow_id
api.tx.lsBiding.cancel(borrow_id:BorrowId)

#### take a borrow specified by borrow_id
api.tx.lsBiding.take(borrow_id:BorrowId)

#### liquidate a loan specified by loan_id
api.tx.lsBiding.liquidate(loan_id:LoanId)

#### add more collateral to an pre-existed loan
api.tx.lsBiding.add(loan_id:LoanId, amount:Balance)

#### repay a healthy loan
api.tx.lsBiding.repay(borrow_id:BorrowId)


## types
```json
{
    "TxHash": "H256",
    "Deposit": {
        "account_id": "AccountId",
        "tx_hash": "Option<TxHash>",
        "amount": "Balance"
    },
    "Auth": {
        "_enum": [
            "All",
            "Deposit",
            "Withdraw",
            "Refund",
            "Mark"
        ]
    },
    "BlackOrWhite": {
        "_enum": [
            "Black",
            "White"
        ]
    },
    "ExtrinsicIndex": "u32",
    "LineNumber": "u32",
    "AuctionBalance": "Balance",
    "TotalLoanBalance": "Balance",
    "CollateralBalanceAvailable": "Balance",
    "CollateralBalanceOriginal": "Balance",
    "Price": "u128",
    "PriceReport": {
        "reporter": "AccountId",
        "price": "Price"
    },
    "LoanHealth": {
        "_enum": ["Well", "ToBeLiquidated", "Liquidated", "Dead", "Completed"]
    },
    "Loan": {
        "id": "LoanId",
        "borrow_id": "BorrowId",
        "borrower_id": "AccountId",
        "loaner_id": "AccountId",
        "due": "BlockNumber",
        "collateral_asset_id": "AssetId",
        "collateral_balance": "Balance",
        "loan_balance": "Balance",
        "loan_asset_id": "AssetId",
        "status": "LoanHealth",
        "interest_rate": "u64",
        "liquidation_type": "LiquidationType"
    },
    "Borrow": {
        "id": "BorrowId",
        "lock_id": "u128",
        "who": "AccountId",
        "status": "BorrowStatus",
        "borrow_asset_id": "AssetId",
        "collateral_asset_id": "AssetId",
        "borrow_balance": "Balance",
        "collateral_balance": "Balance",
        "terms": "u64",
        "interest_rate": "u64",
        "dead_after": "Option<BlockNumber>",
        "loan_id": "Option<LoanId>"
    },
    "LTV": "u64",
    "BorrowId": "u128",
    "LoanId": "u128",
    "LiquidationType": {
        "_enum": ["JustCollateral", "SellCollateral"]
    },
    "BorrowStatus": {
        "_enum": ["Alive", "Taken", "Completed", "Dead", "Liquidated"]
    },
    "TradingPair": {
        "collateral": "u32",
        "borrow": "u32"
    },
    "TradingPairPrices": {
        "borrow_asset_price": "u64",
        "collateral_asset_price": "u64"
    },
    "BorrowOptions": {
        "amount": "Balance",
        "terms": "u64",
        "interest_rate": "u64",
        "warranty": "Option<BlockNumber>"
    },
    "StrBytes": "Vec<u8>",
    "BalanceLock": {
        "id": "u128",
        "asset_id": "AssetId",
        "amount": "Balance",
        "reasons": "WithdrawReasons"
    },
    "PriceInUSDT": "u64"
}
```

## Errors
```rust
            Error::Paused => 0,
            Error::MinBorrowTerms => 1,
            Error::MinBorrowInterestRate => 2,
            Error::CanNotReserve => 3,
            Error::MultipleAliveBorrows => 4,
            Error::BorrowNotAlive => 5,
            Error::TradingPairNotAllowed => 6,
            Error::NotOwnerOfBorrow => 7,
            Error::UnknownBorrowId => 8,
            Error::UnknownLoanId => 9,
            Error::NoLockedBalance => 10,
            Error::InitialCollateralRateFail => 11,
            Error::NotEnoughBalance => 12,
            Error::TradingPairPriceMissing => 13,
            Error::BorrowNotLoaned => 14,
            Error::LTVNotMeet => 15,
            Error::ShouldNotBeLiquidated => 16,
            Error::ShouldBeLiquidated => 17,
            Error::LoanNotWell => 18,
```
