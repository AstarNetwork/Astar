(function() {var implementors = {
"astar_primitives":[["impl&lt;T: Config, OldCurrency&gt; MigrationStep for <a class=\"struct\" href=\"astar_primitives/migrations/contract_v12/struct.Migration.html\" title=\"struct astar_primitives::migrations::contract_v12::Migration\">Migration</a>&lt;T, OldCurrency&gt;<span class=\"where fmt-newline\">where\n    OldCurrency: ReservableCurrency&lt;&lt;T as Config&gt;::AccountId&gt; + 'static,\n    OldCurrency::Balance: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.70.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;&lt;&lt;T as Config&gt;::Currency as Inspect&lt;&lt;T as Config&gt;::AccountId&gt;&gt;::Balance&gt;,</span>"],["impl&lt;T, OldCurrency&gt; MigrationStep for <a class=\"struct\" href=\"astar_primitives/migrations/contract_v14/struct.Migration.html\" title=\"struct astar_primitives::migrations::contract_v14::Migration\">Migration</a>&lt;T, OldCurrency&gt;<span class=\"where fmt-newline\">where\n    T: Config,\n    OldCurrency: 'static + ReservableCurrency&lt;&lt;T as Config&gt;::AccountId&gt;,\n    &lt;&lt;T as Config&gt;::Currency as Inspect&lt;&lt;T as Config&gt;::AccountId&gt;&gt;::Balance: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.70.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;OldCurrency::Balance&gt;,</span>"],["impl&lt;T: Config&gt; MigrationStep for <a class=\"struct\" href=\"astar_primitives/migrations/contract_v12_fix/struct.Migration.html\" title=\"struct astar_primitives::migrations::contract_v12_fix::Migration\">Migration</a>&lt;T&gt;"]]
};if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()