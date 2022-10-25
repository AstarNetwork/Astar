// Import the contract file
import contractFile from './compile.js';

// Add the Web3 provider logic here:
import Web3 from 'web3';

// Create Web3 instance
const web3 = new Web3('http://localhost:9933');

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

export default deploy;