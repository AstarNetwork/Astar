import solc from 'solc';

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

export const compiled = JSON.parse(solc.compile(JSON.stringify(input))).contracts['hello.sol']['Hello'];

