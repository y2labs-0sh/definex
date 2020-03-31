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

pub use self::gen_client::Client as GenericAssetClient;
pub use generic_asset_rpc_runtime_api::{
    self as runtime_api, GenericAssetApi as GenericAssetRuntimeApi,
};

pub enum Error {
    NoAssets,
    RuntimeError,
}
impl From<Error> for i64 {
    fn from(e: Error) -> i64 {
        match e {
            Error::NoAssets => 1,
            Error::RuntimeError => 2,
        }
    }
}
impl From<Error> for String {
    fn from(e: Error) -> String {
        match e {
            Error::NoAssets => "no assets found".to_string(),
            Error::RuntimeError => "runtime trapped".to_string(),
        }
    }
}

/// Generic Asset RPC methods
#[rpc]
pub trait GenericAssetApi<BlockHash, AssetId> {
    #[rpc(name = "genericAsset_symbolsList")]
    fn get_symbols_list(&self, at: Option<BlockHash>) -> Result<Vec<(AssetId, String)>>;
}

pub struct GenericAsset<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}
impl<C, B> GenericAsset<C, B> {
    pub fn new(client: Arc<C>) -> Self {
        GenericAsset {
            client,
            _marker: Default::default(),
        }
    }
}
impl<C, Block, AssetId> GenericAssetApi<<Block as BlockT>::Hash, AssetId> for GenericAsset<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: GenericAssetRuntimeApi<Block, AssetId>,
    AssetId: Codec + MaybeDisplay + MaybeFromStr + Copy + Clone,
{
    fn get_symbols_list(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<(AssetId, String)>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let list = api
            .get_symbols_list(&at)
            .map_err(|e| RPCError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: Error::RuntimeError.into(),
                data: Some(format!("{:?}", e).into()),
            })
            .unwrap();
        match list {
            Some(list) => {
                let mut res: Vec<(AssetId, String)> = Vec::with_capacity(list.len());
                for i in list.iter() {
                    let s = unsafe { String::from_utf8_unchecked(i.1.to_vec()) };
                    res.push((i.0, s));
                }
                return Ok(res);
            }
            None => {
                return Err(RPCError {
                    code: ErrorCode::ServerError(Error::NoAssets.into()),
                    message: Error::NoAssets.into(),
                    data: None,
                });
            }
        }
    }
}
