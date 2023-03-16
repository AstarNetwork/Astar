async function run(nodeName, networkInfo, args) {
    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    const signedBlock = await api.rpc.chain.getBlock();
    console.log(signedBlock, JSON.stringify(signedBlock));

    // the information for each of the contained extrinsics
    await signedBlock.block.extrinsics.forEach((ex, index) => {
        // the extrinsics are decoded by the API, human-like view
        // console.log(index, ex.toHuman());
        console.log(JSON.stringify(ex));

        const { isSigned, meta, method: { args, method, section } } = ex;

        // explicit display of name, args & documentation
        // console.log(`${section}.${method}(${args.map((a) => a.toString()).join(', ')})`);
        console.log(JSON.stringify(args));
        // console.log(meta.documentation.map((d) => d.toString()).join('\n'));

        // signer/nonce info
        if (isSigned) {
        console.log(`signer=${ex.signer.toString()}, nonce=${ex.nonce.toString()}`);
        }
    });

    return 1;
}

module.exports = { run }
