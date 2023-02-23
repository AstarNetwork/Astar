async function run(nodeName, networkInfo, args) {
    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);
 
    const chain = await api.rpc.system.chain();
    console.log(`The current network is: ${chain.toString()}`);

    const capitalize = str => str.split(' ').map(sub => sub.charAt(0).toUpperCase() + sub.slice(1)).join(' ');
    const result = (chain.toString() === `${capitalize(nodeName)} Testnet`) ? 1 : 0

    return result;
}

module.exports = { run }
