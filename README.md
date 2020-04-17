This project is meant for our Web3 grant application.

## How to 
1. Run a local test node:
- `git clone https://github.com/y2labs-0sh/definex.git && cd definex`
- `cargo build -p node-cli --release` 
- `./target/release/substrate purge-chain --dev`
- `./target/release/substrate --dev` (since We are expecting to follow substrate's development releases, we are just leaving the binary name unchanged.)

2. Run a local web wallet:
- `git clone https://github.com/y2labs-0sh/definex-wallet.git && cd definex-wallet`
- `yarn install`
- `yarn run start` this will automatically open localhost:8000 in your browser trying to connecting 'ws://localhost:9944'

3. Connect https://polkadot.js.org/apps/ to your local node
4. Use Alice to mint some DUSD and BTC to some accounts in the genericAsset as a faucet
5. Then you can do 'borrow&lend' in pallets/p2p and 'saving&loan' in pallets/deposit-loan

## P2P module is at pallets/p2p

In this DeFi pallet, there are 3 major roles:

    1. maker: those who want to borrow money. they can publish their needs (collateral amount, borrow amount, how long they will repay, a specific interest rate, etc.) on the platform.
    
    2. taker: those who bring liquidity to the platform. they select the borrows that most profitable, and lend the money to the borrower. By doing this, they earn the negotiated interest.
    
    3. liquidator: those who keep monitoring if there is any loan with a ltv lower than the 'LTVLiquidate'. By doing this, they would be rewarded.

## Money Market Module is at pallets/deposit-loan

**deposit-loan** is an implementation of Financial market protocol that provides both liquid money markets for cross-chain assets and capital markets for longer-term cryptocurrency loans.

    - It will automatically adjust the interest rates based on the amount saved and the amount borrowed.
    
    - We are working on a three-level interest rate based on cash utilization rate that is partially influenced by the economic pricing for scarce resources and our belief that the demand for stable coin is relatively inelastic in different utilization rate intervals. The exact loan interest rate is yet to be determined but it would look like this :
    
    `f(x) = 0.1x + 0.05 （0≤x＜0.4）|| 0.2x + 0.01 (0.4≤x<0.8) || 0.3x^6 + 0.1x^3 + 0.06 (0.8≤x≤1)`
    
    In which, Utilization rate X = Total borrows / (Total deposits + Total Borrows)

## price is fed through offchain worker

pallets/new-oracle

You can customize your crypto price sources by "add_source".

And by default, DUSD(USDT) and BTC are provided.

## assets are based on pallets/generic-asset

This is a modified version of frame/pallet-generic-asset.

We need every asset dynamically created can be reserved with a lock on the balance. But the default frame/pallet-generic-asset implementation doesn't support that.
so we removed those complicated "\*\*Currency", and make all assets lockable with
respective lock id design.

## JS types

This is just for frontend developer

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
        "_enum": ["Well", "ToBeLiquidated", "Overdue", "Liquidated", "Dead", "Completed"]
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
        "status": "P2PBorrowStatus",
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
        "_enum": ["Alive", "Taken", "Canceled", "Completed", "Dead", "Liquidated"]
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
            "Well",
            "Liquidating"
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
    "PhaseId": "u32",
    "UserAssets": {
      "asset_id": "AssetId",
      "symbol": "String",
      "balance": "String"
    },
    "P2PLoanRPC": {
      "id": "P2PLoanId",
      "borrow_id": "P2PBorrowId",
      "borrower_id": "AccountId",
      "loaner_id": "AccountId",
      "secs_left": "u64",
      "collateral_asset_id": "AssetId",
      "collateral_balance": "Balance",
      "loan_balance": "Balance",
      "loan_asset_id": "AssetId",
      "status": "P2PLoanHealth",
      "interest_rate": "u64",
      "liquidation_type": "LiquidationType",
      "can_be_liquidate": "bool",
      "due_height": "BlockNumber"
    }
}
```

## RPC types

Since alpha5, substrate runtime storage seems no longer provides "linked_map".
So we provide some 'list' functions by default to offer some basic support for
our web wallet.

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
          "type": "Option<u64>",
        },
        {
          "name": "offset",
          "type": "Option<u64>",
        },
      ],
      "type": "Vec<P2PBorrow>",
    },
    "userBorrows": {
      "params": [
        {
          "name": "who",
          "type": "AccountId",
        },
        {
          "name": "size",
          "type": "Option<u64>",
        },
        {
          "name": "offset",
          "type": "Option<u64>",
        },
      ],
      "type": "Vec<P2PBorrow>",
    },
    "aliveBorrows": {
      "params": [
        {
          "name": "size",
          "type": "Option<u64>",
        },
        {
          "name": "offset",
          "type": "Option<u64>",
        },
      ],
      "type": "Vec<P2PBorrow>",
    },
    "loans": {
      "params": [
        {
          "name": "size",
          "type": "Option<u64>",
        },
        {
          "name": "offset",
          "type": "Option<u64>",
        },
      ],
      "type": "Vec<P2PLoanRPC>",
    },
    "userLoans": {
      "params": [
        {
          "name": "who",
          "type": "AccountId",
        },
        {
          "name": "size",
          "type": "Option<u64>",
        },
        {
          "name": "offset",
          "type": "Option<u64>",
        },
      ],
      "type": "Vec<P2PLoanRPC>",
    },
    "aliveLoans": {
      "params": [
        {
          "name": "size",
          "type": "Option<u64>",
        },
        {
          "name": "offset",
          "type": "Option<u64>",
        },
      ],
      "type": "Vec<P2PLoanRPC>",
    }
  },
  "depositLoan": {
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
      "type": "Vec<Loan>"
    },
    "userLoans": {
      "params": [
        {
            "name": "who",
            "type": "AccountId"
        },
        {
            "name": "size",
            "type": "Option<u64>"
        },
        {
            "name": "offset",
            "type": "Option<u64>"
        }
      ],
      "type": "Vec<Loan>"
    }
  }
}
```
