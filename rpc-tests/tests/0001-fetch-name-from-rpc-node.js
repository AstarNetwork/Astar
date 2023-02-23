async function run(nodeName, networkInfo, args) {
    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);
 
    const name = await api.rpc.system.name();
    console.log(`The current system name is: ${name.toString()}`);

    const result = (name.toString() === 'Astar Collator') ? 1 : 0
    return result;
}

module.exports = { run }
