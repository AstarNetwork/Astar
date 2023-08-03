async function run(nodeName, networkInfo, args) {
    networkInfo.nodesByName["astar"].rpcPort = 9944
    networkInfo.nodesByName["shiden"].rpcPort = 9945

    const {rpcPort} = networkInfo.nodesByName[nodeName];
    const { compiled } = await import("./compile.mjs");

    // Add the Web3 provider logic here:
    const Web3 = require("web3");

    // Create Web3 instance
    const web3 = new Web3('http://localhost:' + rpcPort);

    // Get the bytecode and API
    const bytecode = compiled.evm.bytecode.object;
    const abi = compiled.abi;

    const evmAccount = {
      privateKey: '0x01ab6e801c06e59ca97a14fc0a1978b27fa366fc87450e0b65459dd3515b7391',
      address: '0xaaafB3972B05630fCceE866eC69CdADd9baC2771',
    };

    const deployedContract = '0x687528e4BC4040DC9ADBA05C1f00aE3633faa731';

    const sayMessage = async (contractAddress) => {
      // 4. Create contract instance
      const hello = new web3.eth.Contract(abi, contractAddress);
      console.log(`Making a call to contract at address: ${contractAddress}`);

      // 6. Call contract
      const data = await hello.methods.sayMessage().call();

      console.log(`The current message is: ${data}`);

      return data;
    };

    const message = await sayMessage(deployedContract);
    const result = (message === 'Hello World') ? 1 : 0;
    return result;
}

module.exports = { run }
