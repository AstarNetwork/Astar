const config = (network) => ({
	"relaychain": {
		"bin": "./bin/polkadot",
		"chain": "rococo-local",
		"nodes": [
			{
				"name": "alice",
				"wsPort": 9944,
				"port": 30444
			},
			{
				"name": "bob",
				"wsPort": 9955,
				"port": 30555
			},
			{
				"name": "charlie",
				"wsPort": 9966,
				"port": 30666
			},
			{
				"name": "dave",
				"wsPort": 9977,
				"port": 30777
			}
		],
		"genesis": {
			"runtime": {
				"runtime_genesis_config": {
					"configuration": {
						"config": {
							"validation_upgrade_frequency": 1,
							"validation_upgrade_delay": 10
						}
					}
				}
			}
		}
	},
	"parachains": [
		{
			"bin": "./bin/astar-collator",
			"id": "2007",
 			"chain": `${network}-dev`,
			"balance": "1000000000000000000000",
			"nodes": [
				{
					"wsPort": 9988,
					"port": 31200,
					"name": "alice",
					"flags": [
						"--unsafe-ws-external",
						"--unsafe-rpc-external",
						"--rpc-port=8545",
						"--rpc-cors=all",
						"--",
						"--execution=wasm"
					]
				},
				{
					"wsPort": 9989,
					"port": 31201,
					"name": "bob",
					"flags": ["--", "--execution=wasm"]
				}
			]
		}
	],
	"simpleParachains": [
	],
	"hrmpChannels": [
	],
	"types": {},
	"finalization": false
});

export default config;
