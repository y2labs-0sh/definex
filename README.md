## most of the P2P logic resides in pallets/p2p

This module is meant for Web3 grant.

In this module, definex implemented a DeFi model which follows a 'maker-taker'.

Basically, there are 3 major roles:

    1. maker: those who want to borrow money. they can publish their needs (collateral amount, borrow amount, how long they will repay, a specific interest rate, etc.) on the platform.

    2. taker: those who bring liquidity to the platform. they select the borrows that most profitable, and lend the money to the borrower. By doing this, they earn the negotiated interest.

    3. liquidator: those who keep monitoring if there is any loan with a ltv lower than the 'LTVLiquidate'. By doing this, they would be rewarded.

## price is fed through offchain worker

pallets/new-oracle

you can customize your crypto price sources by "add_source".

## assets are based on pallets/generic-asset

this is a modified version of frame/pallet-generic-asset.

we need every asset dynamically created can be reserved with a lock on the balance. but the default frame/pallet-generic-asset implementation doesn't support that.

## JS types

```javascript
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
    "PriceInUSDT": "u64",
    "LoanHealth": {
        "_enum": {
            "Well": null,
            "Warning": "LTV",
            "Liquidating": "LTV"
        }
    },
    "LoanPackageStatus": {
        "_enum": [
            "Active",
            "Inactive"
        ]
    },
    "Loan": {
        "id": "LoanId",
        "who": "AccountId",
        "collateral_balance_original": "Balance",
        "collateral_balance_available": "Balance",
        "loan_balance_total": "Balance",
        "status": "LoanHealth"
    },
    "ReleaseTrigger": {
        "_enum": {
            "PhaseChange": null,
            "BlockNumber": "BlockNumber"
        }
    },
    "LoanId": "u64",
    "LoanPackageId": "u64",
    "PhaseId": "u32"
}
```

## RPC types

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
