import { expect } from 'chai';
import {
    capitalize,
    describeWithNetwork,
    sendTransaction
} from './util.js';
import deploy from './deploy.js';
import { sayMessage, setMessage } from './contract.js';

const CONTRACT = '0x0000000000000000000000000000000000000001'; //0x01
const ALICE = 'ajYMsCKsEAhEvHpeA4XqsfiA9v1CdzZPrCfS6pEfeGHW9j8';
const BOB = 'ZAP5o2BjWAo5uoKDE6b6Xkk4Ju7k6bDu24LNjgZbfM3iyiR';
const CHARLIE = 'ZD39yAE4W4RiXCyk1gv6CD2tSaVjQU5KoKfujyft4Xa2GAz';
const DAVE = 'X2mE9hCGX771c3zzV6tPa8U2cDz4U4zkqUdmBrQn83M3cm7';

const evmAccount = {
    privateKey: '0x01ab6e801c06e59ca97a14fc0a1978b27fa366fc87450e0b65459dd3515b7391',
    address: '0xaaafB3972B05630fCceE866eC69CdADd9baC2771',
};

export const getAddressEnum = (address) => ({ Evm: address });

const network = process.env.NETWORK;

if (['astar', 'shiden', 'shibuya'].includes(network) === false) {
    throw new Error('Please set valid network in NETWORK env variable');
}

describeWithNetwork(network, `${network} RPC`, function(context) {
    let deployedContract;

	it('should fetch chain from rpc node', async function () {
		const chain = await context.api.rpc.system.chain();

		expect(chain.toString()).to.equal(`${capitalize(network)} Testnet`);
	});

	it('should fetch chain name from rpc node', async () => {
		const name = await context.api.rpc.system.name();

		expect(name.toString()).to.equal('Astar Collator');
	});

	it('should be able to Register contract on H160 address 0x01 using Alice account', async () => {
        const finalised = await sendTransaction(
            context.api.tx.dappsStaking.register(getAddressEnum(CONTRACT)),
            context.alice
        );

        const dappInfoOpt = await context.api.query.dappsStaking.registeredDapps(getAddressEnum(CONTRACT));

        expect(finalised).to.be.true;
        expect(dappInfoOpt.isSome).to.be.true;

        const dappInfo = dappInfoOpt.unwrap();

        expect(dappInfo.developer.toString()).to.equals(ALICE);
        expect(dappInfo.state.toString()).to.equals('Registered');
    });

	it('should be able to transfer tokens from alice to charlie', async () => {
        const originalBalance = await context.api.query.system.account(CHARLIE);
        const finalised = await sendTransaction(
            context.api.tx.balances.transfer({ Id: CHARLIE }, 100),
            context.alice
        );
        const newBalance = await context.api.query.system.account(CHARLIE);

        expect(finalised).to.be.true;
        expect(newBalance.data.free.sub(originalBalance.data.free).toNumber()).to.equal(100);
    });

    it('should be able to transfer tokens from bob to dave', async () => {
        const originalBalance = await context.api.query.system.account(DAVE);
        const finalised = await sendTransaction(
            context.api.tx.balances.transfer({ Id: DAVE }, 200),
            context.bob
        );
        const newBalance = await context.api.query.system.account(DAVE);

        expect(finalised).to.be.true;
        expect(newBalance.data.free.sub(originalBalance.data.free).toNumber()).to.equal(200);
    });

    it('should be able to deploy an evm contract', async () => {
       const deployed = await deploy(evmAccount);

       deployedContract = deployed.contractAddress;

       expect(deployed.contractAddress).to.be.a.string;
    });

    it('should be able to get evm contract data', async () => {
        const message = await sayMessage(deployedContract);

        expect(message).to.equals('Hello World');
    });

    it('should be able to set evm contract data', async () => {
        const setMsg = await setMessage(deployedContract, evmAccount, 'Hi');

        const message = await sayMessage(deployedContract);

        expect(message).to.equals('Hi');
    });
});
