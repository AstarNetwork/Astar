async function run(nodeName, networkInfo, args) {
    networkInfo.nodesByName["astar"].rpcPort = 8545
    networkInfo.nodesByName["shiden"].rpcPort = 8546

    const {rpcPort} = networkInfo.nodesByName[nodeName];
    const solc = require("solc");

    const source = `
    // SPDX-License-Identifier: MIT
    pragma solidity ^0.8.13;
    
    contract Hello {
        string public message;
    
        constructor() {
            message = "Hello World";
        }
    
        function sayMessage() public view returns (string memory) {
            return message;
        }
    
        function setMessage(string memory newMessage) public {
            message = newMessage;
        }
    }
    `;
    
    const input = {
        language: 'Solidity',
        sources: {
          'hello.sol': {
            content: source
          }
        },
        settings: {
          outputSelection: {
            '*': {
              '*': ['*']
            }
          }
        }
      };
    
    const contractFile = JSON.parse(solc.compile(JSON.stringify(input))).contracts['hello.sol']['Hello'];

    // Add the Web3 provider logic here:
    const Web3 = require("web3");
    
    // Create Web3 instance
    const web3 = new Web3('http://localhost:' + rpcPort);
    
    // Get the bytecode and API
    const bytecode = contractFile.evm.bytecode.object;
    const abi = contractFile.abi;
    
    // Create deploy function
    const deploy = async (accountFrom) => {
      console.log(`Attempting to deploy from account ${accountFrom.address}`);
    
      // Create contract instance
      const hello = new web3.eth.Contract(abi);
    
      // Create constructor tx
      const helloTx = hello.deploy({
        data: bytecode,
        arguments: [],
      });
    
      // Sign transacation and send
      const createTransaction = await web3.eth.accounts.signTransaction(
        {
          data: helloTx.encodeABI(),
          gas: await helloTx.estimateGas(),
        },
        accountFrom.privateKey
      );
    
      // Send tx and wait for receipt
      const createReceipt = await web3.eth.sendSignedTransaction(createTransaction.rawTransaction);
      console.log(`Contract deployed at address: ${createReceipt.contractAddress}`);
    
      return createReceipt;
    };

    const evmAccount = {
      privateKey: '0x01ab6e801c06e59ca97a14fc0a1978b27fa366fc87450e0b65459dd3515b7391',
      address: '0xaaafB3972B05630fCceE866eC69CdADd9baC2771',
    };

    const deployed = await deploy(evmAccount);
    console.log('deployed', deployed);

    const result = (deployed.contractAddress === '0x687528e4BC4040DC9ADBA05C1f00aE3633faa731') ? 1 : 0;
    return result;
}

module.exports = { run }
