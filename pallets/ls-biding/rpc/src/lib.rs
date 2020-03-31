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

pub use self::gen_client::Client as LSBidingClient;
pub use ls_biding_rpc_runtime_api::{self as runtime_api, LSBidingApi as LSBidingRuntimeApi};

use ls_biding_primitives::{Borrow, Loan};

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

/// LSBiding RPC methods
#[rpc]
pub trait LSBidingApi<BlockHash, AssetId, Balance, BlockNumber, AccountId> {
    #[rpc(name = "lsBiding_borrows")]
    fn borrows(
        &self,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<BlockHash>,
    ) -> Result<Vec<Borrow<AssetId, Balance, BlockNumber, AccountId>>>;

    #[rpc(name = "lsBiding_loans")]
    fn loans(
        &self,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<BlockHash>,
    ) -> Result<Vec<Loan<AssetId, Balance, BlockNumber, AccountId>>>;
}

pub struct LSBiding<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}
impl<C, B> LSBiding<C, B> {
    pub fn new(client: Arc<C>) -> Self {
        LSBiding {
            client,
            _marker: Default::default(),
        }
    }
}
impl<C, Block, AssetId, Balance, BlockNumber, AccountId>
    LSBidingApi<<Block as BlockT>::Hash, AssetId, Balance, BlockNumber, AccountId>
    for LSBiding<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: LSBidingRuntimeApi<Block, AssetId, Balance, BlockNumber, AccountId>,
    AssetId: Codec + Copy + Clone,
    Balance: Codec + Copy + Clone,
    BlockNumber: Codec + Copy + Clone,
    AccountId: Codec + Clone,
{
    fn borrows(
        &self,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<Borrow<AssetId, Balance, BlockNumber, AccountId>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let list = api
            .get_borrows(&at, size, offset)
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

    fn loans(
        &self,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<Loan<AssetId, Balance, BlockNumber, AccountId>>> {
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
