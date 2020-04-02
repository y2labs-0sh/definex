use std::sync::Arc;

use codec::Codec;
use jsonrpc_core::{Error as RPCError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::{Bytes, H256};
use sp_rpc::number;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, Header as HeaderT, MaybeDisplay, MaybeFromStr},
};

pub use self::gen_client::Client as DepositLoanClient;

pub use deposit_loan_rpc_runtime_api::{self as runtime_api, DepositLoanApi as DepositLoanRuntimeApi};

// use ls_biding_primitives::{Borrow, Loan};

use deposit_loan_primitives::{Loan};

pub enum Error {
    RuntimeError,
    NoBorrows,
    NoLoans,
}
impl From<Error> for i64 {
    fn from(e: Error) -> i64 {
        match e {
            Error::RuntimeError => 1,
            Error::NoBorrows => 2,
            Error::NoLoans => 3,
        }
    }
}
impl From<Error> for String {
    fn from(e: Error) -> String {
        match e {
            Error::RuntimeError => "runtime trapped".to_string(),
            Error::NoBorrows => "no borrows found".to_string(),
            Error::NoLoans => "no loans found".to_string(),
        }
    }
}

#[rpc]
pub trait DepositLoanApi<BlockHash, AccountId, Balance> {
    #[rpc(name = "depositLoan_loans")]
    fn loans(
        &self,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<BlockHash>,
    ) -> Result<Vec<Loan<AccountId, Balance>>>;
}


pub struct DepositLoan<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}
impl<C, B> DepositLoan<C, B> {
    pub fn new(client: Arc<C>) -> Self {
        DepositLoan {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, AccountId, Balance>
    DepositLoanApi<<Block as BlockT>::Hash, AccountId, Balance>
    for DepositLoan<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: DepositLoanRuntimeApi<Block, AccountId, Balance>,
    Balance: Codec + Copy + Clone,
    AccountId: Codec + Clone,
{
    fn loans(
        &self,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<Loan<AccountId, Balance>>> {

        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let list = api
            .get_loans(&at, size, offset)
            .map_err(|e| RPCError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: Error::RuntimeError.into(),
                data: Some(format!("{:?}", e).into()),
            })
            .unwrap();
        match list {
            None => {
                return Err(RPCError {
                    code: ErrorCode::ServerError(Error::NoBorrows.into()),
                    message: Error::NoBorrows.into(),
                    data: None,
                });
            }
            Some(list) => Ok(list),
        }
    }
}

