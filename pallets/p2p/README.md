## RPCs

api.rpc.pToP.borrows() : []P2PBorrow

api.rpc.pToP.loans() : []P2PLoan

```json
{
  "genericAsset": {
    "symbolsList": {
      "params": [],
      "type": "Vec<(AssetId, String)>"
    },
    "userAssets": {
      "params": [
        {
          "name": "who",
          "type": "AccountId"
        }
      ],
      "type": "Vec<UserAssets<AssetId, Balance>>"
    }
  },
  "pToP": {
    "borrows": {
      "params": [
        {
          "name": "size",
          "type": "Option<u64>"
        },
        {
          "name": "offset",
          "type": "Option<u64>"
        }
      ],
      "type": "Vec<P2PBorrow>"
    },
    "loans": {
      "params": [
        {
          "name": "size",
          "type": "Option<u64>"
        },
        {
          "name": "offset",
          "type": "Option<u64>"
        }
      ],
      "type": "Vec<P2PLoan>"
    }
  }
}
```

## query

\*\*\* important query apis

api.query.pToP.money_pool() : AccountId

api.query.pToP.platform() : AccountId

api.query.pToP.borrows(BorrowId) : P2PBorrow

api.query.pToP.borrow_ids_by_account_id(AccountId) : []P2PBorrowId

api.query.pToP.alive_borrow_ids() : []P2PBorrowId

api.query.pToP.loans(P2PLoanId) : P2PLoan

api.query.pToP.loan_ids_by_account_id(AccountId) : []P2PLoanId

api.query.pToP.alive_loan_ids_by_account_id(AccountId) : []P2PLoanId

api.query.pToP.account_ids_with_loans() : []AccountId

\*\*

api.query.pToP.trading_pairs() : []TradingPair

api.query.pToP.safe_ltv() : u32

api.query.pToP.liquidate_ltv() : u32

api.query.pToP.min_borrow_terms() : u64

api.query.pToP.min_borrow_interest_rate() : u64

api.query.pToP.next_borrow_id() : P2PBorrowId

api.query.pToP.next_loan_id() : P2PLoanId

## extrinsics

#### publish a new borrow with the borrow options

api.tx.pToP.make(collateral_balance:Balance, trading_pair:TradingPair, borrow_options:P2PBorrowOptions)

#### remove a borrow specified by borrow_id

api.tx.pToP.cancel(borrow_id:P2PBorrowId)

#### take a borrow specified by borrow_id

api.tx.pToP.take(borrow_id:P2PBorrowId)

#### liquidate a loan specified by loan_id

api.tx.pToP.liquidate(loan_id:P2PLoanId)

#### add more collateral to an pre-existed loan

api.tx.pToP.add(loan_id:P2PLoanId, amount:Balance)

#### repay a healthy loan

api.tx.pToP.repay(borrow_id:P2PBorrowId)

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
    "_enum": ["All", "Deposit", "Withdraw", "Refund", "Mark"]
  },
  "BlackOrWhite": {
    "_enum": ["Black", "White"]
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
  "P2PLoanHealth": {
    "_enum": ["Well", "ToBeLiquidated", "Liquidated", "Dead", "Completed"]
  },
  "P2PLoan": {
    "id": "P2PLoanId",
    "borrow_id": "P2PBorrowId",
    "borrower_id": "AccountId",
    "loaner_id": "AccountId",
    "due": "BlockNumber",
    "collateral_asset_id": "AssetId",
    "collateral_balance": "Balance",
    "loan_balance": "Balance",
    "loan_asset_id": "AssetId",
    "status": "P2PLoanHealth",
    "interest_rate": "u64",
    "liquidation_type": "LiquidationType"
  },
  "P2PBorrow": {
    "id": "P2PBorrowId",
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
    "loan_id": "Option<P2PLoanId>"
  },
  "LTV": "u64",
  "P2PBorrowId": "u128",
  "P2PLoanId": "u128",
  "LiquidationType": {
    "_enum": ["JustCollateral", "SellCollateral"]
  },
  "P2PBorrowStatus": {
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
  "P2PBorrowOptions": {
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
