# zenlink-protocol-rpc(v0.4.0)

#### 1. introduction

- 1.`zenlinkProtocol_getAllAssets`:

   Get all AssetIds in the Zenlink Module, include `foreign` and `liquidity` assets.
  - {"chain_id":200,"asset_type":0,"asset_index":0}: ParaId=200, Native Currency
  - {"chain_id":200,"asset_type":1,"asset_index":0}: ParaId=300, Liquidity Asset

  ```
  curl -H "Content-Type: application/json" http://localhost:11111 -d \
    '{
      "jsonrpc":"2.0",
      "id":1,
      "method":"zenlinkProtocol_getAllAssets",
      "params": [null]
    }'
  ```
  
  **Response:**
  
  ```json
  {
    "jsonrpc": "2.0",
    "result": [
      {
        "asset_index": 0,
        "asset_type": 0,
        "chain_id": 200
      },
      {
        "asset_index": 0,
        "asset_type": 1,
        "chain_id": 300
      }
    ],
    "id": 1
  }
  ```
   
- 2.`zenlinkProtocol_getBalance`:

  Get the balance of the AssetId and account
    
  ```bash
  curl -H "Content-Type: application/json" http://localhost:11111 -d \
  '{
     "jsonrpc":"2.0",
     "id":1,
     "method":"zenlinkProtocol_getBalance",
     "params": [{"chain_id": 200,"asset_type": 0, "asset_index": 0 }, "5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL"]
   }'  
  ```
  
  **Response:**
  
  ```json
  {
    "jsonrpc": "2.0",
    "result": "0x3fffffffffc1ed7f40",
    "id": 1
  }
  ```
  
- 3.`zenlinkProtocol_getSovereignsInfo`：
  Get the origin info about cross-transfer assets
  Return <(paraid, sovereign_account, balance)>
  
  ```bash
    curl -H "Content-Type: application/json" http://localhost:11111 -d \
    '{
       "jsonrpc":"2.0",
       "id":1,
       "method":"zenlinkProtocol_getSovereignsInfo",
       "params": [{"chain_id": 200,"asset_type": 0, "asset_index":0}]
     }'  
    ```
  
  **Response:**
  
  ```json
  {
    "jsonrpc": "2.0",
    "result": [
      [
        200,
        "5Eg2fntGQpyQv5X5d8N5qxG4sX5UBMLG77xEBPjZ9DTxxtt7",
        "0x0"
      ],
      [
        300,
        "5Eg2fnsj9u3qQZcwEtTDxFqWFHsUcYqupaS8MtEPoeHKAXA4",
        "0x0"
      ]
    ],
    "id": 1
  }
  ```
  
- 4.`zenlinkProtocol_getAllPairs`：

  Get all the swap pairs of Zenlink Module.

    ```bash
  curl -H "Content-Type: application/json" http://localhost:11111 -d \
  '{
        "jsonrpc":"2.0",
        "id":1,
        "method":"zenlinkProtocol_getAllPairs",
        "params": [null]
  }'  
    ```
  
  **Response:**
  - account: the account of swap pair
  - holdingLiquidity: the liquidity of user holding
  - reserve0: the amount of asset0 in swap pair
  - reserve1: the amount of asset1 in swap pair
  - asset0 & asset1: the AssetId of asset0 and asset1
  - totalLiquidity：lptoken total supply
  - lpAssetId: the AssetId of lptoken
  
  ```json
  {
    "jsonrpc": "2.0",
    "result": [
      {
        "account": "5EYCAe5ViNAoHnU1ZZVit8ymcR39EP5fyU6Zv3GV7HD5MN9d",
        "asset0": {
          "asset_index": 0,
          "asset_type": 0,
          "chain_id": 200
        },
        "asset1": {
          "asset_index": 0,
          "asset_type": 0,
          "chain_id": 300
        },
        "holdingLiquidity": "0x0",
        "lpAssetId": {
          "asset_index": 0,
          "asset_type": 1,
          "chain_id": 300
        },
        "reserve0": "0x1d91d9f5",
        "reserve1": "0x29d7f22d",
        "totalLiquidity": "0x232aaf80"
      }
    ],
    "id": 1
  }
  ```

- 5.`zenlinkProtocol_getOwnerPairs`：

  Get the pair info of the specified account
  
  ```bash
  curl -H "Content-Type: application/json" http://localhost:11111 -d \
  '{
        "jsonrpc":"2.0",
        "id":1,
        "method":"zenlinkProtocol_getOwnerPairs",
        "params": ["5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL", null]
  }'
  ```
  
  **Reponse:**
  
  - account: the account of swap pair
  - holdingLiquidity: the liquidity of user holding
  - reserve0: the amount of asset0 in swap pair
  - reserve1: the amount of asset1 in swap pair
  - asset0 & asset1: the AssetId of asset0 and asset1
  - totalLiquidity：lptoken total supply
  - lpAssetId: the AssetId of lptoken
  
  ```json
  {
    "jsonrpc": "2.0",
    "result": [
      {
        "account": "5EYCAe5ViNAoHnU1ZZVit8ymcR39EP5fyU6Zv3GV7HD5MN9d",
        "asset0": {
          "asset_index": 0,
          "asset_type": 0,
          "chain_id": 200
        },
        "asset1": {
          "asset_index": 0,
          "asset_type": 0,
          "chain_id": 300
        },
        "holdingLiquidity": "0x232aaf80",
        "lpAssetId": {
          "asset_index": 0,
          "asset_type": 1,
          "chain_id": 300
        },
        "reserve0": "0x1d91d9f5",
        "reserve1": "0x29d7f22d",
        "totalLiquidity": "0x232aaf80"
      }
    ],
    "id": 1
  }
  ```
  
- 6.`zenlinkProtocol_getPairByAssetId`：

  Get the pair info of the specified AssetIds
  
  ```bash
  curl -H "Content-Type: application/json" http://localhost:11111 -d \
  '{
     "jsonrpc":"2.0",
     "id":1,
     "method":"zenlinkProtocol_getPairByAssetId",
     "params": [
       {"chain_id": 200,"asset_type": 0, "asset_index":0}, 
       {"chain_id": 300,"asset_type": 0, "asset_index":0},
       null
     ]
   }'
  ```

  **Response**
  - account: the account of swap pair
  - holdingLiquidity: the liquidity of user holding
  - reserve0: the amount of asset0 in swap pair
  - reserve1: the amount of asset1 in swap pair
  - asset0 & asset1: the AssetId of asset0 and asset1
  - totalLiquidity：lptoken total supply
  - lpAssetId: the AssetId of lptoken
    
  ```json
  {
    "jsonrpc": "2.0",
    "result": {
      "account": "5EYCAe5ViNAoHnU1ZZVit8ymcR39EP5fyU6Zv3GV7HD5MN9d",
      "asset0": {
        "asset_index": 0,
        "asset_type": 0,
        "chain_id": 200
      },
      "asset1": {
        "asset_index": 0,
        "asset_type": 0,
        "chain_id": 300
      },
      "holdingLiquidity": "0x0",
      "lpAssetId": {
        "asset_index": 0,
        "asset_type": 1,
        "chain_id": 300
      },
      "reserve0": "0x1d91d9f5",
      "reserve1": "0x29d7f22d",
      "totalLiquidity": "0x232aaf80"
    },
    "id": 1
  }
  ```
  
- 7.`zenlinkProtocol_getAmountInPrice`： 
  
  Query the buying rate (fixed trading pair on the right)
  
  - params[0]: "100": the amount of buy
  - params[1]: swap path。
  
  ```bash
  curl -H "Content-Type: application/json" http://localhost:11111 -d \
  '{
     "jsonrpc":"2.0",
     "id":1,
     "method":"zenlinkProtocol_getAmountInPrice",
     "params": [
       100000000,
       [
         {"chain_id": 200,"asset_type": 0, "asset_index":0},
         {"chain_id": 300,"asset_type": 0, "asset_index":0}
       ],
       null
     ]
   }'  
    ```
  
  **Response:**
  - result: 99226799： it means that 10000000 (200,0,0) are exchanged for 82653754 (300,0,0)
    
  ```json
  {
    "jsonrpc": "2.0",
    "result": "0x4ed323a",
    "id": 1
  }
  ```
  
- 8.`zenlinkProtocol_getAmountOutPrice`：
  
   Query the selling exchange rate (fixed trading pair on the left)
  
  - params[0]: "100000000": the amount of sell
  - params[1]: swap path
  
  ```bash
  curl -H "Content-Type: application/json" http://localhost:11111 -d \
  '{
     "jsonrpc":"2.0",
     "id":1,
     "method":"zenlinkProtocol_getAmountOutPrice",
     "params": [
       100000000,
       [
         {"chain_id": 200,"asset_type": 0, "asset_index":0},
         {"chain_id": 300,"asset_type": 0, "asset_index":0}
       ],
       null
     ]
   }'  
  ```

  **Response:**
  
  ```json
  {
    "jsonrpc": "2.0",
    "result": "0x70085cc",
    "id": 1
  }
  ```
    
- 9.`zenlinkProtocol_getEstimateLptoken`:

  ```bash
  curl -H "Content-Type: application/json" http://localhost:11111 -d \
  '{
     "jsonrpc":"2.0",
     "id":1,
     "method":"zenlinkProtocol_getEstimateLptoken",
     "params": [
       {"chain_id": 200,"asset_type": 0, "asset_index":0},
       {"chain_id": 300,"asset_type": 0, "asset_index":0},
       10000000,
       40000000,
       1,
       1,
       null
     ]
   }'  
  ```
  
  **Response:**
  
 ```json
  {
    "jsonrpc": "2.0",
    "result": "0xb5784f",
    "id": 1
  }
  ```

#### 2. rpc calls

```json
{
  "zenlinkProtocol": {
    "getAllAssets": {
      "description": "zenlinkProtocol getAllAssets",
      "params": [
        {
          "name": "at",
          "type": "Hash",
          "isOptional": true
        }
      ],
      "type": "Vec<AssetId>"
    },
    "getBalance": {
      "description": "zenlinkProtocol getBalance",
      "params": [
        {
          "name": "asset_id",
          "type": "AssetId"
        },
        {
          "name": "account",
          "type": "AccountID"
        },
        {
          "name": "at",
          "type": "Hash",
          "isOptional": true
        }
      ],
      "type": "string"
    },
    "getAllPairs": {
      "description": "zenlinkProtocol getAllPairs",
      "params": [
        {
          "name": "at",
          "type": "Hash",
          "isOptional": true
        }
      ],
      "type": "Vec<PairInfo>"
    },
    "getOwnerPairs": {
      "description": "zenlinkProtocol getOwnerPairs",
      "params": [
        {
          "name": "account",
          "type": "AccountID"
        },
        {
          "name": "at",
          "type": "Hash",
          "isOptional": true
        }
      ],
      "type": "Vec<PairInfo>"
    },
    "getPairByAssetId": {
      "description": "zenlinkProtocol getPairByAssetId",
      "params": [
        {
          "name": "asset_0",
          "type": "AssetId"
        },
        {
          "name": "asset_1",
          "type": "AssetId"
        },
        {
          "name": "at",
          "type": "Hash",
          "isOptional": true
        }
      ],
      "type": "PairInfo"
    },
    "getAmountInPrice": {
      "description": "zenlinkProtocol getAmountInPrice",
      "params": [
        {
          "name": "amount_out",
          "type": "AssetBalance"
        },
        {
          "name": "path",
          "type": "Vec<AssetId>"
        },
        {
          "name": "at",
          "type": "Hash",
          "isOptional": true
        }
      ],
      "type": "string"
    },
    "getAmountOutPrice": {
      "description": "zenlinkProtocol getAmountOutPrice",
      "params": [
        {
          "name": "amount_in",
          "type": "AssetBalance"
        },
        {
          "name": "path",
          "type": "Vec<AssetId>"
        },
        {
          "name": "at",
          "type": "Hash",
          "isOptional": true
        }
      ],
      "type": "string"
    },
    "getEstimateLptoken":{
            "description": "zenlinkProtocol getEstimateLptoken",
            "params": [
        {
          "name": "asset_0",
          "type": "AssetId"
        },
        {
          "name": "asset_1",
          "type": "AssetId"
        },
                {
          "name": "amount_0_desired",
          "type": "AssetBalance"
        },
                {
          "name": "amount_1_desired",
          "type": "AssetBalance"
        },
                {
          "name": "amount_0_min",
          "type": "AssetBalance"
        },
                {
          "name": "amount_1_min",
          "type": "AssetBalance"
        },
        {
          "name": "at",
          "type": "Hash",
          "isOptional": true
        }
      ],
      "type": "string"
        }
  }
}
```

#### 3. type

```json
{
  "AssetId": {
    "chain_id": "u32",
    "asset_type": "u8",
    "asset_index": "u32"
  },
  "AssetBalance": "u128",
  "PairInfo": {
    "asset_0": "AssetId",
    "asset_1": "AssetId",
    "account": "AccountId",
    "total_liquidity": "AssetBalance",
    "holding_liquidity": "AssetBalance",
    "reserve_0": "AssetBalance",
    "reserve_1": "AssetBalance",
    "lp_asset_id": "AssetId"
  }
}
```

## License

[GPL-v3](LICENSE)
