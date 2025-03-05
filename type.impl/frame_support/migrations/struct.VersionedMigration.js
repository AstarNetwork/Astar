(function() {var type_impls = {
"pallet_dapp_staking":[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-OnRuntimeUpgrade-for-VersionedMigration%3CFROM,+TO,+Inner,+Pallet,+DbWeight%3E\" class=\"impl\"><a href=\"#impl-OnRuntimeUpgrade-for-VersionedMigration%3CFROM,+TO,+Inner,+Pallet,+DbWeight%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;const FROM: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.77.0/std/primitive.u16.html\">u16</a>, const TO: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.77.0/std/primitive.u16.html\">u16</a>, Inner, Pallet, DbWeight&gt; OnRuntimeUpgrade for VersionedMigration&lt;FROM, TO, Inner, Pallet, DbWeight&gt;<div class=\"where\">where\n    Inner: UncheckedOnRuntimeUpgrade,\n    Pallet: GetStorageVersion&lt;InCodeStorageVersion = StorageVersion&gt; + PalletInfoAccess,\n    DbWeight: Get&lt;RuntimeDbWeight&gt;,</div></h3></section></summary><div class=\"docblock\"><p>Implementation of the <code>OnRuntimeUpgrade</code> trait for <code>VersionedMigration</code>.</p>\n<p>Its main function is to perform the runtime upgrade in <code>on_runtime_upgrade</code> only if the on-chain\nversion of the pallets storage matches <code>From</code>, and after the upgrade set the on-chain storage to\n<code>To</code>. If the versions do not match, it writes a log notifying the developer that the migration\nis a noop.</p>\n</div><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.on_runtime_upgrade\" class=\"method trait-impl\"><a href=\"#method.on_runtime_upgrade\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">on_runtime_upgrade</a>() -&gt; Weight</h4></section></summary><div class=\"docblock\"><p>Executes the versioned runtime upgrade.</p>\n<p>First checks if the pallets on-chain storage version matches the version of this upgrade. If\nit matches, it calls <code>Inner::on_runtime_upgrade</code>, updates the on-chain version, and returns\nthe weight. If it does not match, it writes a log notifying the developer that the migration\nis a noop.</p>\n</div></details></div></details>","OnRuntimeUpgrade","pallet_dapp_staking::migration::versioned_migrations::V8ToV9"]],
"pallet_xc_asset_config":[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-OnRuntimeUpgrade-for-VersionedMigration%3CFROM,+TO,+Inner,+Pallet,+DbWeight%3E\" class=\"impl\"><a href=\"#impl-OnRuntimeUpgrade-for-VersionedMigration%3CFROM,+TO,+Inner,+Pallet,+DbWeight%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;const FROM: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.77.0/std/primitive.u16.html\">u16</a>, const TO: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.77.0/std/primitive.u16.html\">u16</a>, Inner, Pallet, DbWeight&gt; OnRuntimeUpgrade for VersionedMigration&lt;FROM, TO, Inner, Pallet, DbWeight&gt;<div class=\"where\">where\n    Inner: UncheckedOnRuntimeUpgrade,\n    Pallet: GetStorageVersion&lt;InCodeStorageVersion = StorageVersion&gt; + PalletInfoAccess,\n    DbWeight: Get&lt;RuntimeDbWeight&gt;,</div></h3></section></summary><div class=\"docblock\"><p>Implementation of the <code>OnRuntimeUpgrade</code> trait for <code>VersionedMigration</code>.</p>\n<p>Its main function is to perform the runtime upgrade in <code>on_runtime_upgrade</code> only if the on-chain\nversion of the pallets storage matches <code>From</code>, and after the upgrade set the on-chain storage to\n<code>To</code>. If the versions do not match, it writes a log notifying the developer that the migration\nis a noop.</p>\n</div><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.on_runtime_upgrade\" class=\"method trait-impl\"><a href=\"#method.on_runtime_upgrade\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">on_runtime_upgrade</a>() -&gt; Weight</h4></section></summary><div class=\"docblock\"><p>Executes the versioned runtime upgrade.</p>\n<p>First checks if the pallets on-chain storage version matches the version of this upgrade. If\nit matches, it calls <code>Inner::on_runtime_upgrade</code>, updates the on-chain version, and returns\nthe weight. If it does not match, it writes a log notifying the developer that the migration\nis a noop.</p>\n</div></details></div></details>","OnRuntimeUpgrade","pallet_xc_asset_config::migrations::versioned::V2ToV3"]]
};if (window.register_type_impls) {window.register_type_impls(type_impls);} else {window.pending_type_impls = type_impls;}})()