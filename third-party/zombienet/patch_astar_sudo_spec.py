"""
Patch an Astar plain chain-spec so it can be driven by zombienet from a raw file:

  1. Overwrite the runtime WASM with a sudo-featured build.
  2. Set the sudo key.
  3. Rewrite `session.keys` and `collator_selection.invulnerables` for the given
     collator names — the collators zombienet spawns derive their aura keys from
     these names, and once we hand zombienet a raw spec it can no longer inject
     them itself. Without this rewrite the collators are unauthorised to author
     and the chain never produces a block.

Usage:
    patch_astar_sudo_spec.py <plain_in> <wasm> <patched_out> \\
        <sudo_ss58> <collator_binary> <collator_name>...
"""

import json
import subprocess
import sys
from pathlib import Path

if len(sys.argv) < 7:
    print(__doc__, file=sys.stderr)
    sys.exit(2)

plain_path = Path(sys.argv[1])
wasm_path = Path(sys.argv[2])
out_path = Path(sys.argv[3])
sudo_ss58 = sys.argv[4]
collator_bin = sys.argv[5]
collator_names = sys.argv[6:]


def inspect(uri: str) -> dict:
    """Return {'ss58': ..., 'public_hex': ...} for a Substrate URI."""
    out = subprocess.check_output(
        [collator_bin, "key", "inspect", "--scheme", "sr25519", "--output-type", "json", uri],
        text=True,
    )
    j = json.loads(out)
    # subkey/collator output field names vary slightly; accept both.
    ss58 = j.get("ss58Address") or j["ss58_address"]
    pub = j.get("publicKey") or j["public_key"]
    if not pub.startswith("0x"):
        pub = "0x" + pub
    return {"ss58": ss58, "public_hex": pub}


# Zombienet derives node keys from "//<CapitalizedName>".
def uri_for(name: str) -> str:
    return "//" + name[:1].upper() + name[1:]


collators = [(name, inspect(uri_for(name))) for name in collator_names]

spec = json.loads(plain_path.read_text())
runtime_genesis = spec["genesis"]["runtimeGenesis"]

if "patch" in runtime_genesis:
    key = "patch"
elif "config" in runtime_genesis:
    key = "config"
else:
    raise Exception("unknown runtimeGenesis layout")

cfg = runtime_genesis[key]

cfg.setdefault("sudo", {})["key"] = sudo_ss58

cfg.setdefault("session", {})["keys"] = [
    [c["ss58"], c["ss58"], {"aura": c["public_hex"]}] for _, c in collators
]

cfg.setdefault("collatorSelection", {})
cfg["collatorSelection"]["invulnerables"] = [c["ss58"] for _, c in collators]

# Aura authorities are populated from session on new_session; leave empty at genesis.
cfg.setdefault("aura", {})["authorities"] = []

runtime_genesis["code"] = "0x" + wasm_path.read_bytes().hex()

spec["name"] = "Astar Mainnet Runtime Local"
spec["id"] = "astar-mainnet-runtime-local"
spec["chainType"] = "Local"

out_path.write_text(json.dumps(spec))

print(f"Patched spec written to {out_path}")
for name, info in collators:
    print(f"  collator {name} ({uri_for(name)}) -> {info['ss58']}")
