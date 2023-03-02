# js-script known details of zndsl

`[nodeName]: js-script ./[path to script] with "[args]" ([return is greater than x] or [return is equal to x]) [within y seconds]`

See the [Testing DSL spec](https://paritytech.github.io/zombienet/cli/test-dsl-definition-spec.html)

Warning: can't use `"type": "module",` in `package.json`, it will break js-script functionality.

## The run function has 3 parameter

`async function run(nodeName, networkInfo, args) { ... }`

### nodeName

A string of the name of the parachains.collator or relaychain.nodes as defined in the .toml file.

### networkInfo

```json
{
  tmpDir: '/tmp/zombie-761f1a1f6ee74842242e90d09fbdc2c5_-3685709-RuyzsfXB12n9',
  chainSpecPath: '/tmp/zombie-761f1a1f6ee74842242e90d09fbdc2c5_-3685709-RuyzsfXB12n9/rococo-local.json',
  relay: [
    {
      name: 'relay01',
      wsUri: 'ws://127.0.0.1:37839',
      prometheusUri: 'http://127.0.0.1:37667/metrics',
      userDefinedTypes: {}
    },
    {
      name: 'relay02',
      wsUri: 'ws://127.0.0.1:37455',
      prometheusUri: 'http://127.0.0.1:40329/metrics',
      userDefinedTypes: {}
    },
    {
      name: 'relay03',
      wsUri: 'ws://127.0.0.1:35361',
      prometheusUri: 'http://127.0.0.1:34653/metrics',
      userDefinedTypes: {}
    },
    {
      name: 'relay04',
      wsUri: 'ws://127.0.0.1:45777',
      prometheusUri: 'http://127.0.0.1:43713/metrics',
      userDefinedTypes: {}
    }
  ],
  paras: {
    '2006': {
      chainSpecPath: '/tmp/zombie-761f1a1f6ee74842242e90d09fbdc2c5_-3685709-RuyzsfXB12n9/astar-dev-2006-rococo-local.json',
      wasmPath: '/tmp/zombie-761f1a1f6ee74842242e90d09fbdc2c5_-3685709-RuyzsfXB12n9/2006/genesis-wasm',
      statePath: '/tmp/zombie-761f1a1f6ee74842242e90d09fbdc2c5_-3685709-RuyzsfXB12n9/2006/genesis-state',
      nodes: [Array]
    },
    '2007': {
      chainSpecPath: '/tmp/zombie-761f1a1f6ee74842242e90d09fbdc2c5_-3685709-RuyzsfXB12n9/shiden-dev-2007-rococo-local.json',
      wasmPath: '/tmp/zombie-761f1a1f6ee74842242e90d09fbdc2c5_-3685709-RuyzsfXB12n9/2007/genesis-wasm',
      statePath: '/tmp/zombie-761f1a1f6ee74842242e90d09fbdc2c5_-3685709-RuyzsfXB12n9/2007/genesis-state',
      nodes: [Array]
    }
  },
  nodesByName: {
    relay01: {
      name: 'relay01',
      wsUri: 'ws://127.0.0.1:37839',
      prometheusUri: 'http://127.0.0.1:37667/metrics',
      userDefinedTypes: {}
    },
    relay02: {
      name: 'relay02',
      wsUri: 'ws://127.0.0.1:37455',
      prometheusUri: 'http://127.0.0.1:40329/metrics',
      userDefinedTypes: {}
    },
    relay03: {
      name: 'relay03',
      wsUri: 'ws://127.0.0.1:35361',
      prometheusUri: 'http://127.0.0.1:34653/metrics',
      userDefinedTypes: {}
    },
    relay04: {
      name: 'relay04',
      wsUri: 'ws://127.0.0.1:45777',
      prometheusUri: 'http://127.0.0.1:43713/metrics',
      userDefinedTypes: {}
    },
    astar: {
      name: 'astar',
      wsUri: 'ws://127.0.0.1:36527',
      prometheusUri: 'http://127.0.0.1:35863/metrics',
      userDefinedTypes: {},
      parachainId: 2006
    },
    shiden: {
      name: 'shiden',
      wsUri: 'ws://127.0.0.1:38037',
      prometheusUri: 'http://127.0.0.1:33795/metrics',
      userDefinedTypes: {},
      parachainId: 2007
    }
  }
}
```

### args

An array of arguments introduced using with:  `with "Alice,Bob"`

`[ 'Alice','Bob' ]`
