use std::sync::Arc;

use codec::Codec;
use jsonrpc_core::{Error as RPCError, ErrorCode, Result as RPCResult};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

pub use self::gen_client::Client as GenericAssetClient;
pub use generic_asset_rpc_runtime_api::{
    self as runtime_api, GenericAssetApi as GenericAssetRuntimeApi,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Eq, PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct UserAssets<AssetId, Balance> {
    pub asset_id: AssetId,
    pub symbol: String,

    #[serde(bound(serialize = "Balance: std::fmt::Display"))]
    #[serde(serialize_with = "serialize_as_string")]
    #[serde(bound(deserialize = "Balance: std::str::FromStr"))]
    #[serde(deserialize_with = "deserialize_from_string")]
    pub balance: Balance,
}

fn serialize_as_string<S: Serializer, T: std::fmt::Display>(
    t: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&t.to_string())
}

fn deserialize_from_string<'de, D: Deserializer<'de>, T: std::str::FromStr>(
    deserializer: D,
) -> Result<T, D::Error> {
    let s = String::deserialize(deserializer)?;
    s.parse::<T>()
        .map_err(|_| serde::de::Error::custom("Parse from string failed"))
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
pub trait GenericAssetApi<BlockHash, AccountId, SymbolsResponse, AssetsResponse> {
    #[rpc(name = "genericAsset_symbolsList")]
    fn get_symbols_list(&self, at: Option<BlockHash>) -> RPCResult<Vec<SymbolsResponse>>;

    #[rpc(name = "genericAsset_userAssets")]
    fn get_user_assets(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> RPCResult<Vec<AssetsResponse>>;
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
    GenericAssetApi<
        <Block as BlockT>::Hash,
        AccountId,
        (AssetId, String),
        UserAssets<AssetId, Balance>,
    > for GenericAsset<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: GenericAssetRuntimeApi<Block, AssetId, Balance, AccountId>,
    AssetId: Codec + Copy + Clone + std::str::FromStr + std::fmt::Display,
    Balance: Codec + Copy + Clone + std::str::FromStr + std::fmt::Display,
    AccountId: Codec + Clone + std::fmt::Display,
{
    fn get_symbols_list(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RPCResult<Vec<(AssetId, String)>> {
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
    ) -> RPCResult<Vec<UserAssets<AssetId, Balance>>> {
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
