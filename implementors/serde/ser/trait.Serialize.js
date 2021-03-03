(function() {var implementors = {};
implementors["pallet_plasm_rewards"] = [{"text":"impl Serialize for GenesisConfig <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;u32: Serialize,<br>&nbsp;&nbsp;&nbsp;&nbsp;Forcing: Serialize,&nbsp;</span>","synthetic":false,"types":[]}];
implementors["pallet_plasm_validator"] = [{"text":"impl&lt;T:&nbsp;Config&gt; Serialize for GenesisConfig&lt;T&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;Vec&lt;T::AccountId&gt;: Serialize,&nbsp;</span>","synthetic":false,"types":[]}];
implementors["plasm_cli"] = [{"text":"impl Serialize for Extensions","synthetic":false,"types":[]}];
implementors["plasm_runtime"] = [{"text":"impl Serialize for SessionKeys","synthetic":false,"types":[]},{"text":"impl Serialize for GenesisConfig","synthetic":false,"types":[]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()