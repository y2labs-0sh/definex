#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use sp_runtime::traits::{MaybeDisplay, MaybeFromStr};
use sp_std::vec::Vec;

use deposit_loan_primitives::*;

sp_api::decl_runtime_apis! {
    pub trait DepositLoanApi<AccountId, Balance> where
        Balance: Codec,
        AccountId: Codec,
    {
        fn get_loans(size: Option<u64>, offset: Option<u64>) -> Option<Vec<Loan<AccountId, Balance>>>;

    }
}
