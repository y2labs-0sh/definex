use std::sync::Arc;

use codec::Codec;
use jsonrpc_core::{Error as RPCError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
// use sp_core::{Bytes, H256};
use sp_rpc::number;
use sp_runtime::{
    generic::BlockId,
    traits::{AtLeast32Bit, Block as BlockT},
};
use std::convert::{TryFrom, TryInto};

pub use self::gen_client::Client as P2PClient;
pub use p2p_rpc_runtime_api::{self as runtime_api, P2PApi as P2PRuntimeApi};

use p2p_primitives::*;

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

/// P2P RPC methods
#[rpc]
pub trait P2PApi<BlockHash, AccountId, Moment, BorrowsResult, LoansResult> {
    #[rpc(name = "pToP_borrows")]
    fn borrows(
        &self,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<BlockHash>,
    ) -> Result<BorrowsResult>;

    #[rpc(name = "pToP_userBorrows")]
    fn user_borrows(
        &self,
        who: AccountId,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<BlockHash>,
    ) -> Result<BorrowsResult>;

    #[rpc(name = "pToP_aliveBorrows")]
    fn alive_borrows(
        &self,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<BlockHash>,
    ) -> Result<BorrowsResult>;

    #[rpc(name = "pToP_loans")]
    fn loans(
        &self,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<BlockHash>,
    ) -> Result<LoansResult>;

    #[rpc(name = "pToP_userLoans")]
    fn user_loans(
        &self,
        who: AccountId,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<BlockHash>,
    ) -> Result<LoansResult>;

    #[rpc(name = "pToP_aliveLoans")]
    fn alive_loans(
        &self,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<BlockHash>,
    ) -> Result<LoansResult>;
}

pub struct P2P<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}
impl<C, B> P2P<C, B> {
    pub fn new(client: Arc<C>) -> Self {
        P2P {
            client,
            _marker: Default::default(),
        }
    }
}
impl<C, Block, AssetId, Balance, BlockNumber, AccountId, Moment>
    P2PApi<
        <Block as BlockT>::Hash,
        AccountId,
        Moment,
        Vec<P2PBorrow<AssetId, Balance, BlockNumber, AccountId>>,
        Vec<P2PLoanRPC<AssetId, Balance, AccountId>>,
    > for P2P<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: P2PRuntimeApi<Block, AssetId, Balance, BlockNumber, AccountId, Moment>,
    AssetId: Codec + Copy + Clone,
    Balance: Codec + Copy + Clone,
    BlockNumber: Codec + Copy + Clone + AtLeast32Bit,
    AccountId: Codec + Clone,
    Moment: Codec + Copy + Clone + AtLeast32Bit,
{
    fn borrows(
        &self,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<P2PBorrow<AssetId, Balance, BlockNumber, AccountId>>> {
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
        Ok(list)
    }

    fn user_borrows(
        &self,
        who: AccountId,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<P2PBorrow<AssetId, Balance, BlockNumber, AccountId>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let list = api
            .get_user_borrows(&at, who, size, offset)
            .map_err(|e| RPCError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: Error::RuntimeError.into(),
                data: Some(format!("{:?}", e).into()),
            })
            .unwrap();
        Ok(list)
    }

    fn alive_borrows(
        &self,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<P2PBorrow<AssetId, Balance, BlockNumber, AccountId>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let list = api
            .get_alive_borrows(&at, size, offset)
            .map_err(|e| RPCError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: Error::RuntimeError.into(),
                data: Some(format!("{:?}", e).into()),
            })
            .unwrap();
        Ok(list)
    }

    fn loans(
        &self,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<P2PLoanRPC<AssetId, Balance, AccountId>>> {
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

        self.p2p_loan_2_rpc_loan::<AssetId, Balance, BlockNumber, AccountId, Moment>(api, at, list)
    }

    fn user_loans(
        &self,
        who: AccountId,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<P2PLoanRPC<AssetId, Balance, AccountId>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let list = api
            .get_user_loans(&at, who, size, offset)
            .map_err(|e| RPCError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: Error::RuntimeError.into(),
                data: Some(format!("{:?}", e).into()),
            })
            .unwrap();

        self.p2p_loan_2_rpc_loan::<AssetId, Balance, BlockNumber, AccountId, Moment>(api, at, list)
    }

    fn alive_loans(
        &self,
        size: Option<u64>,
        offset: Option<u64>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<P2PLoanRPC<AssetId, Balance, AccountId>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let list = api
            .get_alive_loans(&at, size, offset)
            .map_err(|e| RPCError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: Error::RuntimeError.into(),
                data: Some(format!("{:?}", e).into()),
            })
            .unwrap();

        self.p2p_loan_2_rpc_loan::<AssetId, Balance, BlockNumber, AccountId, Moment>(api, at, list)
    }
}

impl<C, Block> P2P<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
{
    fn p2p_loan_2_rpc_loan<AssetId, Balance, BlockNumber, AccountId, Moment>(
        &self,
        api: ApiRef<C::Api>,
        at: BlockId<Block>,
        list: Vec<P2PLoan<AssetId, Balance, BlockNumber, AccountId>>,
    ) -> Result<Vec<P2PLoanRPC<AssetId, Balance, AccountId>>>
    where
        C::Api: P2PRuntimeApi<Block, AssetId, Balance, BlockNumber, AccountId, Moment>,
        AssetId: Codec + Copy + Clone,
        Balance: Codec + Copy + Clone,
        BlockNumber: Codec + Copy + Clone + AtLeast32Bit,
        AccountId: Codec + Clone,
        Moment: Codec + Copy + Clone + AtLeast32Bit,
    {
        let at_hash = match at {
            BlockId::Hash(h) => h,
            _ => unreachable!("we are assure at is BlockId::Hash"),
        };
        let block_number: u64 = TryInto::<u64>::try_into(
            self.client
                .number(at_hash)
                .map_err(|e| RPCError {
                    code: ErrorCode::ServerError(Error::RuntimeError.into()),
                    message: Error::RuntimeError.into(),
                    data: Some(format!("{:?}", e).into()),
                })?
                .ok_or(RPCError {
                    code: ErrorCode::ServerError(Error::RuntimeError.into()),
                    message: Error::RuntimeError.into(),
                    data: None,
                })?,
        )
        .ok()
        .unwrap();
        let secs_per_block: u64 = TryInto::<u64>::try_into(api.get_secs_per_block(&at).unwrap())
            .ok()
            .unwrap();

        Ok(list
            .iter()
            .map(|v| -> P2PLoanRPC<AssetId, Balance, AccountId> {
                let due = TryInto::<u64>::try_into(v.due).ok().unwrap();
                let blocks_left = if due <= block_number {
                    0u64
                } else {
                    due - block_number
                };

                P2PLoanRPC {
                    id: v.id,
                    borrow_id: v.borrow_id,
                    borrower_id: v.borrower_id.clone(),
                    loaner_id: v.loaner_id.clone(),
                    secs_left: blocks_left * secs_per_block,
                    collateral_asset_id: v.collateral_asset_id,
                    collateral_balance: v.collateral_balance,
                    loan_asset_id: v.loan_asset_id,
                    loan_balance: v.loan_balance,
                    status: v.status,
                    interest_rate: v.interest_rate,
                    liquidation_type: v.liquidation_type,
                    can_be_liquidate: if v.status == P2PLoanHealth::ToBeLiquidated
                        || v.status == P2PLoanHealth::Overdue
                    {
                        true
                    } else {
                        false
                    },
                }
            })
            .collect::<_>())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct P2PLoanRPC<AssetId, Balance, AccountId> {
    #[serde(bound(serialize = "Balance: std::fmt::Display"))]
    #[serde(serialize_with = "serialize_as_string")]
    #[serde(bound(deserialize = "Balance: std::str::FromStr"))]
    #[serde(deserialize_with = "deserialize_from_string")]
    pub id: P2PLoanId,

    #[serde(bound(serialize = "Balance: std::fmt::Display"))]
    #[serde(serialize_with = "serialize_as_string")]
    #[serde(bound(deserialize = "Balance: std::str::FromStr"))]
    #[serde(deserialize_with = "deserialize_from_string")]
    pub borrow_id: P2PBorrowId,

    pub borrower_id: AccountId,
    pub loaner_id: AccountId,
    pub secs_left: u64,
    pub collateral_asset_id: AssetId,

    #[serde(bound(serialize = "Balance: std::fmt::Display"))]
    #[serde(serialize_with = "serialize_as_string")]
    #[serde(bound(deserialize = "Balance: std::str::FromStr"))]
    #[serde(deserialize_with = "deserialize_from_string")]
    pub collateral_balance: Balance,

    #[serde(bound(serialize = "Balance: std::fmt::Display"))]
    #[serde(serialize_with = "serialize_as_string")]
    #[serde(bound(deserialize = "Balance: std::str::FromStr"))]
    #[serde(deserialize_with = "deserialize_from_string")]
    pub loan_balance: Balance,

    pub loan_asset_id: AssetId,
    pub status: P2PLoanHealth,
    pub interest_rate: u64,
    pub liquidation_type: LiquidationType,
    pub can_be_liquidate: bool,
}
