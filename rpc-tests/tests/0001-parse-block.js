async function run(nodeName, networkInfo, args) {
    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    const signedBlock = await api.rpc.chain.getBlock();
    console.log(signedBlock, JSON.stringify(signedBlock));

    // This test is a work in progress, it is supposed to test a fix for a unparsable block
    // Shiden bad block: 0x53f61ccccff565b91fbd2a01b46fdf958d92486cb998134627e7c992f56dcadd
    // Unable to retrieve the specified block details. createType(SignedBlock):: Struct: failed on block: {"header":"Header","extrinsics":"Vec<Extrinsic>"}:: Struct: failed on extrinsics: Vec<Extrinsic>:: createType(ExtrinsicV4):: createType(Call):: Call: failed decoding ethereum.transact:: Struct: failed on args: {"transaction":"Lookup146"}:: decodeU8aStruct: failed at 0x44000000000000000000000000000000… on transaction (index 1/1): {"hash":"H256","nonce":"u256","blockHash":"Option<H256>","blockNumber":"Option<U256>","transactionIndex":"Option<U256>","from":"H160","to":"Option<H160>","value":"u256","gasPrice":"Option<U256>","maxFeePerGas":"Option<U256>","maxPriorityFeePerGas":"Option<U256>","gas":"u256","input":"Bytes","creates":"Option<H160>","raw":"Bytes","publicKey":"Option<H512>","chainId":"Option<U64>","standardV":"u256","v":"u256","r":"u256","s":"u256","accessList":"Option<Vec<EthAccessListItem>>","transactionType":"Option<U256>"}:: decodeU8aStruct: failed at 0xe742f3c5850454000000000000000000… on accessList (index 22/23): Option<Vec<EthAccessListItem>>:: Vec length 561085648 exceeds 65536
    //
    // the information for each of the contained extrinsics
    await signedBlock.block.extrinsics.forEach((ex, index) => {
        // the extrinsics are decoded by the API, human-like view
        // console.log(index, ex.toHuman());
        // console.log(JSON.stringify(ex));

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
