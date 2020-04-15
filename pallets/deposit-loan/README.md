**deposit-loan** is an implementation of Financial market protocol that provides both liquid money markets for cross-chain assets and capital markets for longer-term cryptocurrency  loans. 

## How it works

+ It will automatically adjust the interest rates based on the amount saved and the amount borrowed.

+ We are working on a three-level interest rate based on cash utilization rate that is partially influenced by the economic pricing for scarce resources and our belief that the demand for stable coin is relatively inelastic in different utilization rate intervals.  The exact loan interest rate is yet to be determined but it would look like this :

  `f(x) = 0.1x + 0.05 （0≤x＜0.4）|| 0.2x + 0.01 (0.4≤x<0.8) || 0.3x^6 + 0.1x^3 + 0.06 (0.8≤x≤1)`

  In which, Utilization rate X = Total borrows / (Total deposits + Total Borrows)

+ Each time when a block is issued, the interest generated in that interval will be calculated

  based on the last time interest was calculated versus the current time interval versus realtime

  interest,  and the interest is transferred to collection_account. At the same time, based on the

  price of the collateralized asset, it is calculated whether any loan has reached the liquidation

  threshold and those loans will be marked as liquidation status.

  Here is a simple way to calculate Compound interest within every block without calculate each account.

  The initial value of token is set as 1. When a user depoist some money, he will get some dtoken:

     `dtoken_user_will_get = deposit_amount / value_of_token`

     `total_deposit += deposit_amount`

  When interest is deposited, the value of token will be calculated as:

    `value_of_token = value_of_token * interest_amount / total_deposit`

    `total_deposit += interest_amount`

+ Simply example will be shown here:

  ​    At the begining User_A deposit 100 usdt, the price of token is 1; so User_A will get 100 dtoken.

  ​    After some time, 3 usdt interest generated, so the price of token will be: (100 + 3)/100 = 1.03.

  ​    That is, if User_A want to redeem all money, he will get: `100 dtoken * 1.03 value_of_dtoken = 103 usdt`

  ​    Then, User_B deposit 50 usdt, he will get `50 usdt / 1.03 value_of_dtoken` dtoken;

  ​    After some time, 10 usdt interest generated, the value of token will be: `1.03 * (1 + 10 / 153)`

  ​    If User_A want to redeem all now, he will get: `100 dtoken * 1.03 * (1 + 10 / 153)` usdt

  ​    User_B will get: `50 usdt / 1.03 * 1.03 * (1 + 10 / 153)` usdt

  ​    As for the 10 usdt interest:

  ​    `User_A get:User_B get == 103:50 == (100 * 1.03 * (1 + 10 / 153) - 103):(50 / 1.03 * 1.03 * (1 + 10 / 153) - 50)`


## query

**important query apis**

api.query.depositLoan.loan_interest_rate_current() : T::Balance

api.query.depositLoan.saving_interest_rate() : T::Balance

api.query.depositLoan.global_ltv_limit() : LTV

api.query.depositLoan.global_liquidation_threshold() : LTV

api.query.depositLoan.get_loan_by_id(LoanId) : Loan

api.query.depositLoan.loans_by_account(AccountId) : []Loan

api.query.depositLoan.total_loan() : T::Balance

api.query.depositLoan.total_collateral() : T::Balance

api.query.depositLoan.liquidation_penalty() : T::Balance

api.query.depositLoan.liquidating_loans() : []Loan

api.query.depositLoan.minimum_collateral() : T::Balance

api.query.depositLoan.value_of_tokens() : T::Balance

api.query.depositLoan.user_dtoken(AccountId) : T::Balance

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

## Errors

```
Error::Paused => 0,
Error::NotEnoughBalance => 1,
Error::SavingTypeNotAllowed => 2,
Error::UnknowAssetId => 3,
Error::TradingPairPriceMissing => 4,
Error::MinCollateralAmount => 5,
Error::UnknownLoanId => 6,
Error::NotLoanOwner => 7,
Error::LoanInLiquidation => 8,
Error::LoanNotInLiquidation => 9,
Error::TotalCollateralUnderflow => 10,
Error::ReachLoanCap => 11,
Error::InvalidCollateralLoanAmounts => 12,
Error::OverLTVLimit => 13,
Error::SavingIsZero => 14,
```

## RPC types

```
{
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

