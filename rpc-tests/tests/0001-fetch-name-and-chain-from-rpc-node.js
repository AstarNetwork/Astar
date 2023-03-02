async function run(nodeName, networkInfo, args) {
    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    const name = await api.rpc.system.name();
    console.log(`The current system name is: ${name.toString()}`);

    const nameResult = (name.toString() === 'Astar Collator') ? 1 : 0

    const chain = await api.rpc.system.chain();
    console.log(`The current network is: ${chain.toString()}`);

    const capitalize = str => str.split(' ').map(sub => sub.charAt(0).toUpperCase() + sub.slice(1)).join(' ');
    const chainResult = (chain.toString() === `${capitalize(nodeName)} Testnet`) ? 1 : 0

    return nameResult && chainResult ? 1 : 0 ;
}

module.exports = { run }
