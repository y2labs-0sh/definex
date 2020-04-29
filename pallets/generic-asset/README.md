## Config
#### For test purpose only

``` rust
        generic_asset: Some(GenericAssetConfig {
            next_asset_id: 2,
            symbols: vec![
                (0, "DUSD".as_bytes().to_vec()),
                (1, "BTC".as_bytes().to_vec()),
            ],
        })
```

Currently we don't pre-mine any generic assets. And in generic asset pallet, we don't allow average user creating assets.

We have predefined 2 assets: 'DUSD' with asset id 0, 'BTC' with asset id 1.

All assets predefined are owned by `sudo::Root`. Since `sudo::Root` is set to 'Alice', you can just mint 'DUSD' and 'BTC' to any testing account by using 'Alice' in the "Polkadotjs/App".

In the near future, we will integrate 'XCMP' to bridge real Bitcoin.

## RPCs

api.rpc.genericAsset.symbolList()

api.rpc.genericAsset.userAssets()

``` json
  "genericAsset": {
    "symbolsList": {
      "params": [],
      "type": "Vec<(AssetId, String)>"
    },
    "userAssets": {
      "params": [
        {
          "name": "who",
          "type": "AccountId"
        }
      ],
      "type": "Vec<UserAssets<AssetId, Balance>>"
    }
  },
```

## types

``` json
{
  "UserAssets": {
    "asset_id": "AssetId",
    "symbol": "String",
    "balance": "String"
  }
}
```
