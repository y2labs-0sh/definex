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
