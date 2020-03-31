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

#[derive(Eq, PartialEq, Default, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct UserAssets<AssetId, Balance> {
    pub asset_id: AssetId,
    pub symbol: String,

    #[cfg_attr(
        feature = "std",
        serde(bound(serialize = "Balance: std::fmt::Display"))
    )]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(
        feature = "std",
        serde(bound(deserialize = "Balance: std::str::FromStr"))
    )]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub balance: Balance,
}

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
pub trait GenericAssetApi<BlockHash, AssetId, Balance, AccountId> {
    #[rpc(name = "genericAsset_symbolsList")]
    fn get_symbols_list(&self, at: Option<BlockHash>) -> Result<Vec<(AssetId, String)>>;

    #[rpc(name = "genericAsset_userAssets")]
    fn get_user_assets(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Result<Vec<UserAssets<AssetId, Balance>>>;
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
impl<C, Block, AssetId, Balance, AccountId>
    GenericAssetApi<<Block as BlockT>::Hash, AssetId, Balance, AccountId> for GenericAsset<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: GenericAssetRuntimeApi<Block, AssetId, Balance, AccountId>,
    AssetId: Codec + MaybeDisplay + MaybeFromStr + Copy + Clone + std::fmt::Debug,
    Balance: Codec + MaybeDisplay + MaybeFromStr + Copy + Clone + std::fmt::Debug,
    AccountId: Codec + Clone + MaybeDisplay,
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
                for (asset_id, name) in list {
                    let s = unsafe { String::from_utf8_unchecked(name.to_vec()) };
                    res.push((asset_id, s));
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

    fn get_user_assets(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<UserAssets<AssetId, Balance>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let list = api.get_user_assets(&at, who).map_err(|e| RPCError {
            code: ErrorCode::ServerError(Error::RuntimeError.into()),
            message: Error::RuntimeError.into(),
            data: Some(format!("{:?}", e).into()),
        })?;
        match list {
            Some(list) => {
                let mut res: Vec<UserAssets<AssetId, Balance>> = Vec::with_capacity(list.len());
                for (asset_id, name_bytes, balance) in list {
                    let s = unsafe { String::from_utf8_unchecked(name_bytes.to_vec()) };
                    res.push(UserAssets {
                        asset_id,
                        balance,
                        symbol: s,
                    });
                }
                dbg!(&res);
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
