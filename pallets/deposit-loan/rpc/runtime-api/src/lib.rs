#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_std::vec::Vec;

use deposit_loan_primitives::*;

sp_api::decl_runtime_apis! {
    pub trait DepositLoanApi<AccountId, Balance> where
        Balance: Codec,
        AccountId: Codec,
    {
        fn get_loans(size: Option<u64>, offset: Option<u64>) -> Vec<Loan<AccountId, Balance>>;

        fn get_user_loans(who: AccountId, size: Option<u64>, offset: Option<u64>) -> Vec<Loan<AccountId, Balance>>;
    }
}
