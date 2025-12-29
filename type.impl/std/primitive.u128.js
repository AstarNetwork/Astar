(function() {
    var type_impls = Object.fromEntries([["astar_primitives",[]],["astar_runtime",[]],["shibuya_runtime",[]],["shiden_runtime",[]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[23,21,23,22]}