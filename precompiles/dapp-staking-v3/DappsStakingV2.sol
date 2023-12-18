// SPDX-License-Identifier: BSD-3-Clause

pragma solidity >=0.8.0;

/// Predeployed at the address 0x0000000000000000000000000000000000005001
/// For better understanding check the source code:
/// repo: https://github.com/AstarNetwork/Astar
/// code: pallets/dapp-staking-v3
interface DAppStaking {

    // Types

    /// Describes the subperiod in which the protocol currently is.
    enum Subperiod {Voting, BuildAndEarn}

    /// Describes current smart contract types supported by the network.
    enum SmartContractType {EVM, WASM}

    /// @notice Describes protocol state.
    /// @param era: Ongoing era number.
    /// @param period: Ongoing period number.
    /// @param subperiod: Ongoing subperiod type.
    struct ProtocolState {
        uint256 era;
        uint256 period;
        Subperiod subperiod;
    }

    /// @notice Used to describe smart contract. Astar supports both EVM & WASM smart contracts
    ///         so it's important to differentiate between the two. This approach also allows
    ///         easy extensibility in the future.
    /// @param contract_type: Type of the smart contract to be used
    struct SmartContract {
        SmartContractType contract_type;
        bytes contract_address;
    }

    // Storage getters

    /// @notice Get the current protocol state.
    /// @return (current era, current period number, current subperiod type).
    function protocol_state() external view returns (ProtocolState memory);

    /// @notice Get the unlocking period expressed in the number of blocks.
    /// @return period: The unlocking period expressed in the number of blocks.
    function unlocking_period() external view returns (uint256);


    // Extrinsic calls

    /// @notice Lock the given amount of tokens into dApp staking protocol.
    /// @param amount: The amount of tokens to be locked.
    function lock(uint128 amount) external returns (bool);

    /// @notice Start the unlocking process for the given amount of tokens.
    /// @param amount: The amount of tokens to be unlocked.
    function unlock(uint128 amount) external returns (bool);

    /// @notice Claims unlocked tokens, if there are any
    function claim_unlocked() external returns (bool);

    /// @notice Stake the given amount of tokens on the specified smart contract.
    ///         The amount specified must be precise, otherwise the call will fail.
    /// @param smart_contract: The smart contract to be staked on.
    /// @param amount: The amount of tokens to be staked.
    function stake(SmartContract calldata smart_contract, uint128 amount) external returns (bool);

    /// @notice Unstake the given amount of tokens from the specified smart contract.
    ///         The amount specified must be precise, otherwise the call will fail.
    /// @param smart_contract: The smart contract to be unstaked from.
    /// @param amount: The amount of tokens to be unstaked.
    function unstake(SmartContract calldata smart_contract, uint128 amount) external returns (bool);

    /// @notice Claims one or more pending staker rewards.
    function claim_staker_rewards() external returns (bool);

    /// @notice Claim the bonus reward for the specified smart contract.
    /// @param smart_contract: The smart contract for which the bonus reward should be claimed.
    function claim_bonus_reward(SmartContract calldata smart_contract) external returns (bool);

    /// @notice Claim dApp reward for the specified smart contract & era.
    /// @param smart_contract: The smart contract for which the dApp reward should be claimed.
    /// @param era: The era for which the dApp reward should be claimed.
    function claim_dapp_reward(SmartContract calldata smart_contract, uint256 era) external returns (bool);

    /// @notice Unstake all funds from the unregistered smart contract.
    /// @param smart_contract: The smart contract which was unregistered and from which all funds should be unstaked.
    function unstake_from_unregistered(SmartContract calldata smart_contract) external returns (bool);

    /// @notice Used to cleanup all expired contract stake entries from the caller.
    function cleanup_expired_entries() external returns (bool);
}
