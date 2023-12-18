// SPDX-License-Identifier: BSD-3-Clause

pragma solidity >=0.8.0;

/// Predeployed at the address 0x0000000000000000000000000000000000005001
/// For better understanding check the source code:
/// repo: https://github.com/AstarNetwork/Astar
///
/// **NOTE:** This is a soft-deprecated interface used by the old dApps staking v2.
///           It is still supported by the network, but doesn't reflect how dApp staking v3 should be used.
///           Please refer to the `v2` interface for the latest version of the dApp staking contract.
///
///           It is possible that dApp staking feature will once again evolve in the future so all developers are encouraged
///           to keep their smart contracts which utilize dApp staking precompile interface as upgradable, or implement their logic
///           in such a way it's relatively simple to migrate to the new version of the interface.
interface DappsStaking {

    // Types

    /// Instruction how to handle reward payout for staker.
    /// `FreeBalance` - Reward will be paid out to the staker (free balance).
    /// `StakeBalance` - Reward will be paid out to the staker and is immediately restaked (locked balance)
    enum RewardDestination {FreeBalance, StakeBalance}

    // Storage getters

    /// @notice Read current era.
    /// @return era: The current era
    function read_current_era() external view returns (uint256);

    /// @notice Read the unbonding period (or unlocking period) in the number of eras.
    /// @return period: The unbonding period in eras
    function read_unbonding_period() external view returns (uint256);

    /// @notice Read Total network reward for the given era - sum of staker & dApp rewards.
    /// @return reward: Total network reward for the given era
    function read_era_reward(uint32 era) external view returns (uint128);

    /// @notice Read Total staked amount for the given era
    /// @return staked: Total staked amount for the given era
    function read_era_staked(uint32 era) external view returns (uint128);

    /// @notice Read Staked amount for the staker
    /// @param staker: The staker address in form of 20 or 32 hex bytes
    /// @return amount: Staked amount by the staker
    function read_staked_amount(bytes calldata staker) external view returns (uint128);

    /// @notice Read Staked amount on a given contract for the staker
    /// @param contract_id: The smart contract address used for staking
    /// @param staker: The staker address in form of 20 or 32 hex bytes
    /// @return amount: Staked amount by the staker
    function read_staked_amount_on_contract(address contract_id, bytes calldata staker) external view returns (uint128);

    /// @notice Read the staked amount from the era when the amount was last staked/unstaked
    /// @return total: The most recent total staked amount on contract
    function read_contract_stake(address contract_id) external view returns (uint128);


    // Extrinsic calls

    /// @notice Register is root origin only and not allowed via evm precompile.
    ///         This should always fail.
    function register(address) external returns (bool);

    /// @notice Stake provided amount on the contract.
    function bond_and_stake(address, uint128) external returns (bool);

    /// @notice Start unbonding process and unstake balance from the contract.
    function unbond_and_unstake(address, uint128) external returns (bool);

    /// @notice Withdraw all funds that have completed the unbonding process.
    function withdraw_unbonded() external returns (bool);

    /// @notice Claim earned staker rewards for the oldest unclaimed era.
    ///         In order to claim multiple eras, this call has to be called multiple times.
    ///         Staker account is derived from the caller address.
    /// @param smart_contract: The smart contract address used for staking
    function claim_staker(address smart_contract) external returns (bool);

    /// @notice Claim one era of unclaimed dapp rewards for the specified contract and era.
    /// @param smart_contract: The smart contract address used for staking
    /// @param era: The era to be claimed
    function claim_dapp(address smart_contract, uint128 era) external returns (bool);

    /// @notice Set reward destination for staker rewards
    /// @param reward_destination: The instruction on how the reward payout should be handled
    function set_reward_destination(RewardDestination reward_destination) external returns (bool);

    /// @notice Withdraw staked funds from an unregistered contract.
    /// @param smart_contract: The smart contract address used for staking
    function withdraw_from_unregistered(address smart_contract) external returns (bool);

    /// @notice Transfer part or entire nomination from origin smart contract to target smart contract
    /// @param origin_smart_contract: The origin smart contract address
    /// @param amount: The amount to transfer from origin to target
    /// @param target_smart_contract: The target smart contract address
    function nomination_transfer(address origin_smart_contract, uint128 amount, address target_smart_contract) external returns (bool);
}
