import json
import sys
from pathlib import Path

plain_path = Path(sys.argv[1])
wasm_path = Path(sys.argv[2])
out_path = Path(sys.argv[3])
alice = sys.argv[4]

spec = json.loads(plain_path.read_text())

runtime_genesis = spec["genesis"]["runtimeGenesis"]

# detect layout
if "patch" in runtime_genesis:
    key = "patch"
elif "config" in runtime_genesis:
    key = "config"
else:
    raise Exception("unknown runtimeGenesis layout")

runtime_genesis[key].setdefault("sudo", {})
runtime_genesis[key]["sudo"]["key"] = alice

runtime_genesis["code"] = "0x" + wasm_path.read_bytes().hex()

spec["name"] = "Astar Mainnet Runtime Local"
spec["id"] = "astar-mainnet-runtime-local"
spec["chainType"] = "Local"

out_path.write_text(json.dumps(spec))
