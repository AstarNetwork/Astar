/**
 * End to end test for council elections on Astar
 * Test steps:
 * - Build the chain source code by following the instructions in README.md
 * - Run the collator locally:
 * - $ ./target/release/astar-collator --port 30333 --ws-port 9944 --rpc-port 9933 --rpc-cors all --alice --dev
 * - Navigate to the JavaScript interface at https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/js
 * - Run the following code in this file up to the "Post-election tests" line into the console
 * - Once that code has run, wait for the current council election to run (see the Council tab).
 * - Run the code after "Post-election tests" to assert that the council members have been elected.
 */

function makeTx(api, tx, signer) {
  console.log(`Making tx: ${tx.method.section}::${tx.method.method}`);
  return new Promise((resolve, reject) => {
    tx.signAndSend(signer, async (status) => {
      if (status.isFinalized) {
        for (const e of status.events) {
          const { data, method, section } = e.event;
          if (section === 'system') {
            if (method === 'ExtrinsicSuccess') {
              resolve();
            } else if (method === 'ExtrinsicFailed') {
              const errorData = data[0];
              let errorInfo;
              if (errorData.isModule) {
                const details = api.registry.findMetaError(errorData.asModule.toU8a());
                errorInfo = `${details.section}::${details.name}: ${details.documentation[0]}`;
              } else if (errorData.isBadOrigin) {
                errorInfo = 'TX Error: invalid sender origin';
              } else if (errorData.isCannotLookup) {
                errorInfo = 'TX Error: cannot lookup call';
              } else {
                errorInfo = 'TX Error: unknown';
              }
              reject(new Error(errorInfo));
            }
          }
        }
      } else if (status.isError) {
        reject(new Error(`Failed to submit tx '${tx.method.method}'`));
      }
    });
  });
}

const ALICE = 'ajYMsCKsEAhEvHpeA4XqsfiA9v1CdzZPrCfS6pEfeGHW9j8';
const BOB = 'ZAP5o2BjWAo5uoKDE6b6Xkk4Ju7k6bDu24LNjgZbfM3iyiR';
const CHARLIE = 'ZD39yAE4W4RiXCyk1gv6CD2tSaVjQU5KoKfujyft4Xa2GAz';
const DAVE = 'X2mE9hCGX771c3zzV6tPa8U2cDz4U4zkqUdmBrQn83M3cm7';
const EVE = 'b9KwTcKzttJjDuZji5cWgXA22jeraTdwtJkUGp4bT8i4VYZ';
const FERDIE = 'WayqAccATqhyZY34Poum2y2wjFrL4U4oqRB69zWd8SGcJcM';


const sameMembers = (a1, a2) => {
  if (a1.length != a2.length) {return false;}
  for (var i = 0;i < a1.length;i++) {
    if (a1[i] != a2[i]) {return false}
  }
  return true;
}

const sameMembersUnsorted = (values, expectedValues) => {
  const values_ = [...values];
  const expectedValues_ = [...expectedValues];
  values_.sort();
  expectedValues_.sort();
  return sameMembers(values_, expectedValues_);
}

// initially expect no members
const members = await api.query.elections.members();
const memberIds = members.map((elem) => elem.who);
console.log('initial members == []:', sameMembersUnsorted(memberIds, []));

// get balances of accounts
const aliceInitialAccountData = await api.query.system.account(ALICE);
console.log('alice initial free balance == 1,000,000,000,000,000,000,000', aliceInitialAccountData.data.free == 1000000000000000000000);

const eveInitialAccountData = await api.query.system.account(EVE);
console.log('eve initial free balance == 1,000,000,000,000,000,000,000', eveInitialAccountData.data.free == 1000000000000000000000);

// the council has 3 seats

// submit some candidates

const submitCandidacyPromises = [
  makeTx(api, api.tx.elections.submitCandidacy(1), keyring.getPair(ALICE)),
  makeTx(api, api.tx.elections.submitCandidacy(2), keyring.getPair(BOB)),
  makeTx(api, api.tx.elections.submitCandidacy(3), keyring.getPair(CHARLIE)),
  makeTx(api, api.tx.elections.submitCandidacy(4), keyring.getPair(DAVE)),
  makeTx(api, api.tx.elections.submitCandidacy(5), keyring.getPair(EVE)),
];

console.log('submitting candidacies...');
await Promise.all(submitCandidacyPromises);
console.log('candidacies submitted');

const candidates = await api.query.elections.candidates();
const candidateIds = candidates.map((elem) => elem[0]);
console.log('initial candidates == [ALICE, BOB, CHARLIE, DAVE, EVE]:', sameMembersUnsorted(candidateIds, [ALICE, BOB, CHARLIE, DAVE, EVE]));

const bond = api.consts.elections.candidacyBond;
// cast some votes
const votePromises = [
  makeTx(api, api.tx.elections.vote([BOB, CHARLIE, DAVE], bond), keyring.getPair(ALICE)),
  makeTx(api, api.tx.elections.vote([ALICE, CHARLIE], bond), keyring.getPair(BOB)),
  makeTx(api, api.tx.elections.vote([ALICE, BOB], bond), keyring.getPair(CHARLIE)),
  makeTx(api, api.tx.elections.vote([ALICE], bond), keyring.getPair(DAVE)),
  // don't make a vote for Eve
];

console.log('casting votes...');
await Promise.all(votePromises);
console.log('votes have been cast');

// Post-election tests
// WAIT for the election cycle to end then run the following code

const electedMembers = await api.query.elections.members();
const electedMemberIds = electedMembers.map((elem) => elem.who);
console.log('elected members == [ALICE, BOB, CHARLIE]:', sameMembersUnsorted(electedMemberIds, [ALICE, BOB, CHARLIE]));

const runnersUp = await api.query.elections.runnersUp();
const runnersUpIds = runnersUp.map((elem) => elem.who);
console.log('runners up == [DAVE]', sameMembersUnsorted(runnersUpIds, [DAVE]));

// check that the council has been updated
const councilMemberIds = await api.query.council.members();
console.log('council members == [ALICE, BOB, CHARLIE]:', sameMembersUnsorted(councilMemberIds, [ALICE, BOB, CHARLIE]));

// assert balances of accounts
