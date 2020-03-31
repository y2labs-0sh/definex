## query

**important query apis**

api.query.depositLoan.loan_interest_rate_current() : T::Balance

api.query.depositLoan.saving_interest_rate() : T::Balance

api.query.depositLoan.global_ltv_limit() : LTV

api.query.depositLoan.global_liquidation_threshold() : LTV

api.query.depositLoan.global_warning_threshold() : LTV

api.query.depositLoan.get_loan_by_id(LoanId) : Loan

api.query.depositLoan.loans_by_account(AccountId) : []Loan

api.query.depositLoan.total_loan() : T::Balance

api.query.depositLoan.total_collateral() : T::Balance

api.query.depositLoan.liquidation_penalty() : T::Balance

api.query.depositLoan.liquidating_loans() : []Loan

api.query.depositLoan.minimum_collateral() : T::Balance

## extrinsics

**deposit some assets into module**

api.tx.depositLoan.staking(asset_id: T::AssetId, amount: T::Balance)

**redeem some assets from module**

api.tx.depositLoan.redeem(iou_asset_id: T::AssetId, iou_asset_amount: T::Balance)

**apply a loan by collateral some assets**

api.tx.depositLoan.apply_loan(collateral_amount: T::Balance, loan_amount: T::Balance)

**repay a healthy loan**

api.tx.depositLoan.repay_loan(loan_id: LoanId)

**liquidate a loan specified with auction balance by loan_id**

api.tx.depositLoan.mark_liquidated(loan_id: LoanId, auction_balance: T::Balance)

**add more collateral to an pre-existed loan**

api.tx.depositLoan.add_collateral(loan_id: LoanId, amount: T::Balance)

**draw some assets from an existing loan**

api.tx.depositLoan.draw(loan_id: LoanId, amount: T::Balance)

## types

```
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
    "LTV": "u64",
    "LoanId": "u64",
    "LoanPackageId": "u64",
    "PhaseId": "u32",
    "PriceInUSDT": "u64",
    "StrBytes": "Vec<u8>"
}

```
