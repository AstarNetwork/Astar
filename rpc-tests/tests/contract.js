// 1. Import the contract abi
import compiled from './compile.js';

// 2. Add the Web3 provider logic here:
import Web3 from 'web3';

// Create Web3 instance
const web3 = new Web3('http://localhost:9933');

// 5. Create get function
export const sayMessage = async (contractAddress) => {
    const abi = compiled.abi;
    // 4. Create contract instance
    const hello = new web3.eth.Contract(abi, contractAddress);
    console.log(`Making a call to contract at address: ${contractAddress}`);

    // 6. Call contract
    const data = await hello.methods.sayMessage().call();

    console.log(`The current message is: ${data}`);

    return data;
};

export const setMessage = async (contractAddress, accountFrom, message) => {
    console.log(
        `Calling the setMessage function in contract at address: ${contractAddress}`
    );

    const abi = compiled.abi;

    // Create contract instance
    const hello = new web3.eth.Contract(abi, contractAddress);
    // Build tx
    const helloTx = hello.methods.setMessage(message);

    // Sign Tx with PK
    const createTransaction = await web3.eth.accounts.signTransaction(
        {
            to: contractAddress,
            data: helloTx.encodeABI(),
            gas: await helloTx.estimateGas(),
        },
        accountFrom.privateKey
    );

    // Send Tx and Wait for Receipt
    const createReceipt = await web3.eth.sendSignedTransaction(createTransaction.rawTransaction);
    console.log(`Tx successful with hash: ${createReceipt.transactionHash}`);
};
