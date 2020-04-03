**deposit-loan** is an implementation of Financial market protocol that provides both liquid money markets for cross-chain assets and capital markets for longer-term cryptocurrency  loans. 

## How it works

+ It will automatically adjust the interest rates based on the amount saved and the amount borrowed.

+ We are working on a three-level interest rate based on cash utilization rate that is partially influenced by the economic pricing for scarce resources and our belief that the demand for stable coin is relatively inelastic in different utilization rate intervals.  The exact loan interest rate is yet to be determined but it would look like this : 

  `f(x) = 0.1x + 0.05 （0≤x＜0.4）|| 0.2x + 0.01 (0.4≤x<0.8) || 0.3x^6 + 0.1x^3 + 0.06 (0.8≤x≤1)`
  
  In which, Utilization rate X = Total borrows / (Total deposits + Total Borrows)


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
