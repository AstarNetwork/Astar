// This file is part of Astar.

// Copyright 2019-2022 PureStake Inc.
// Copyright (C) 2022-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later
//
// This file is part of Utils package, originally developed by Purestake Inc.
// Utils package used in Astar Network in terms of GPLv3.
//
// Utils is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Utils is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Utils.  If not, see <http://www.gnu.org/licenses/>.
use super::*;
use assert_matches::assert_matches;
use fp_evm::{
    ExitReason, ExitSucceed, PrecompileOutput, PrecompileResult, PrecompileSet, Transfer,
};
use sp_std::boxed::Box;

pub struct Subcall {
    pub address: H160,
    pub transfer: Option<Transfer>,
    pub input: Vec<u8>,
    pub target_gas: Option<u64>,
    pub is_static: bool,
    pub context: Context,
}

pub struct SubcallOutput {
    pub reason: ExitReason,
    pub output: Vec<u8>,
    pub cost: u64,
    pub logs: Vec<Log>,
}

pub trait SubcallTrait: FnMut(Subcall) -> SubcallOutput + 'static {}

impl<T: FnMut(Subcall) -> SubcallOutput + 'static> SubcallTrait for T {}

pub type SubcallHandle = Box<dyn SubcallTrait>;

/// Mock handle to write tests for precompiles.
pub struct MockHandle {
    pub gas_limit: u64,
    pub gas_used: u64,
    pub logs: Vec<PrettyLog>,
    pub subcall_handle: Option<SubcallHandle>,
    pub code_address: H160,
    pub input: Vec<u8>,
    pub context: Context,
    pub is_static: bool,
}

impl MockHandle {
    pub fn new(code_address: H160, context: Context) -> Self {
        Self {
            gas_limit: u64::MAX,
            gas_used: 0,
            logs: vec![],
            subcall_handle: None,
            code_address,
            input: Vec::new(),
            context,
            is_static: false,
        }
    }
}

// Compute the cost of doing a subcall.
// Some parameters cannot be known in advance, so we estimate the worst possible cost.
pub fn call_cost(value: U256, config: &evm::Config) -> u64 {
    // Copied from EVM code since not public.
    pub const G_CALLVALUE: u64 = 9000;
    pub const G_NEWACCOUNT: u64 = 25000;

    fn address_access_cost(is_cold: bool, regular_value: u64, config: &evm::Config) -> u64 {
        if config.increase_state_access_gas {
            if is_cold {
                config.gas_account_access_cold
            } else {
                config.gas_storage_read_warm
            }
        } else {
            regular_value
        }
    }

    fn xfer_cost(is_call_or_callcode: bool, transfers_value: bool) -> u64 {
        if is_call_or_callcode && transfers_value {
            G_CALLVALUE
        } else {
            0
        }
    }

    fn new_cost(
        is_call_or_staticcall: bool,
        new_account: bool,
        transfers_value: bool,
        config: &evm::Config,
    ) -> u64 {
        let eip161 = !config.empty_considered_exists;
        if is_call_or_staticcall {
            if eip161 {
                if transfers_value && new_account {
                    G_NEWACCOUNT
                } else {
                    0
                }
            } else if new_account {
                G_NEWACCOUNT
            } else {
                0
            }
        } else {
            0
        }
    }

    let transfers_value = value != U256::default();
    let is_cold = true;
    let is_call_or_callcode = true;
    let is_call_or_staticcall = true;
    let new_account = true;

    address_access_cost(is_cold, config.gas_call, config)
        + xfer_cost(is_call_or_callcode, transfers_value)
        + new_cost(is_call_or_staticcall, new_account, transfers_value, config)
}

impl PrecompileHandle for MockHandle {
    /// Perform subcall in provided context.
    /// Precompile specifies in which context the subcall is executed.
    fn call(
        &mut self,
        address: H160,
        transfer: Option<Transfer>,
        input: Vec<u8>,
        target_gas: Option<u64>,
        is_static: bool,
        context: &Context,
    ) -> (ExitReason, Vec<u8>) {
        if self
            .record_cost(call_cost(context.apparent_value, &evm::Config::london()))
            .is_err()
        {
            return (ExitReason::Error(ExitError::OutOfGas), vec![]);
        }

        match &mut self.subcall_handle {
            Some(handle) => {
                let SubcallOutput {
                    reason,
                    output,
                    cost,
                    logs,
                } = handle(Subcall {
                    address,
                    transfer,
                    input,
                    target_gas,
                    is_static,
                    context: context.clone(),
                });

                if self.record_cost(cost).is_err() {
                    return (ExitReason::Error(ExitError::OutOfGas), vec![]);
                }

                for log in logs {
                    self.log(log.address, log.topics, log.data)
                        .expect("cannot fail");
                }

                (reason, output)
            }
            None => panic!("no subcall handle registered"),
        }
    }

    fn record_cost(&mut self, cost: u64) -> Result<(), ExitError> {
        self.gas_used += cost;

        if self.gas_used > self.gas_limit {
            Err(ExitError::OutOfGas)
        } else {
            Ok(())
        }
    }

    fn remaining_gas(&self) -> u64 {
        self.gas_limit - self.gas_used
    }

    fn log(&mut self, address: H160, topics: Vec<H256>, data: Vec<u8>) -> Result<(), ExitError> {
        self.logs.push(PrettyLog(Log {
            address,
            topics,
            data,
        }));
        Ok(())
    }

    /// Retreive the code address (what is the address of the precompile being called).
    fn code_address(&self) -> H160 {
        self.code_address
    }

    /// Retreive the input data the precompile is called with.
    fn input(&self) -> &[u8] {
        &self.input
    }

    /// Retreive the context in which the precompile is executed.
    fn context(&self) -> &Context {
        &self.context
    }

    /// Is the precompile call is done statically.
    fn is_static(&self) -> bool {
        self.is_static
    }

    /// Retreive the gas limit of this call.
    fn gas_limit(&self) -> Option<u64> {
        Some(self.gas_limit)
    }
}

pub struct PrecompilesTester<'p, P> {
    precompiles: &'p P,
    handle: MockHandle,

    target_gas: Option<u64>,
    subcall_handle: Option<SubcallHandle>,

    expected_cost: Option<u64>,
    expected_logs: Option<Vec<PrettyLog>>,
}

impl<'p, P: PrecompileSet> PrecompilesTester<'p, P> {
    pub fn new(
        precompiles: &'p P,
        from: impl Into<H160>,
        to: impl Into<H160>,
        data: Vec<u8>,
    ) -> Self {
        let to = to.into();
        let mut handle = MockHandle::new(
            to,
            Context {
                address: to,
                caller: from.into(),
                apparent_value: U256::zero(),
            },
        );

        handle.input = data;

        Self {
            precompiles,
            handle,

            target_gas: None,
            subcall_handle: None,

            expected_cost: None,
            expected_logs: None,
        }
    }

    pub fn with_value(mut self, value: impl Into<U256>) -> Self {
        self.handle.context.apparent_value = value.into();
        self
    }

    pub fn with_subcall_handle(mut self, subcall_handle: impl SubcallTrait) -> Self {
        self.subcall_handle = Some(Box::new(subcall_handle));
        self
    }

    pub fn with_target_gas(mut self, target_gas: Option<u64>) -> Self {
        self.target_gas = target_gas;
        self
    }

    pub fn expect_cost(mut self, cost: u64) -> Self {
        self.expected_cost = Some(cost);
        self
    }

    pub fn expect_no_logs(mut self) -> Self {
        self.expected_logs = Some(vec![]);
        self
    }

    pub fn expect_log(mut self, log: Log) -> Self {
        self.expected_logs = Some({
            let mut logs = self.expected_logs.unwrap_or_default();
            logs.push(PrettyLog(log));
            logs
        });
        self
    }

    fn assert_optionals(&self) {
        if let Some(cost) = &self.expected_cost {
            assert_eq!(&self.handle.gas_used, cost);
        }

        if let Some(logs) = &self.expected_logs {
            similar_asserts::assert_eq!(&self.handle.logs, logs);
        }
    }

    fn execute(&mut self) -> Option<PrecompileResult> {
        let handle = &mut self.handle;
        handle.subcall_handle = self.subcall_handle.take();

        if let Some(gas_limit) = self.target_gas {
            handle.gas_limit = gas_limit;
        }

        let res = self.precompiles.execute(handle);

        self.subcall_handle = handle.subcall_handle.take();

        res
    }

    /// Execute the precompile set and expect some precompile to have been executed, regardless of the
    /// result.
    pub fn execute_some(mut self) {
        let res = self.execute();
        assert!(res.is_some());
        self.assert_optionals();
    }

    /// Execute the precompile set and expect no precompile to have been executed.
    pub fn execute_none(mut self) {
        let res = self.execute();
        assert!(res.is_none());
        self.assert_optionals();
    }

    /// Execute the precompile set and check it returns provided output.
    pub fn execute_returns(mut self, output: Vec<u8>) {
        let res = self.execute();
        assert_eq!(
            res,
            Some(Ok(PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output
            }))
        );
        self.assert_optionals();
    }

    /// Execute the precompile set and check if it reverts.
    /// Take a closure allowing to perform custom matching on the output.
    pub fn execute_reverts(mut self, check: impl Fn(&[u8]) -> bool) {
        let res = self.execute();
        assert_matches!(
            res,
            Some(Err(PrecompileFailure::Revert { output, ..}))
                if check(&output)
        );
        self.assert_optionals();
    }

    /// Execute the precompile set and check it returns provided output.
    pub fn execute_error(mut self, error: ExitError) {
        let res = self.execute();
        assert_eq!(
            res,
            Some(Err(PrecompileFailure::Error { exit_status: error }))
        );
        self.assert_optionals();
    }
}

pub trait PrecompileTesterExt: PrecompileSet + Sized {
    fn prepare_test(
        &self,
        from: impl Into<H160>,
        to: impl Into<H160>,
        data: Vec<u8>,
    ) -> PrecompilesTester<Self>;
}

impl<T: PrecompileSet> PrecompileTesterExt for T {
    fn prepare_test(
        &self,
        from: impl Into<H160>,
        to: impl Into<H160>,
        data: Vec<u8>,
    ) -> PrecompilesTester<Self> {
        PrecompilesTester::new(self, from, to, data)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct PrettyLog(Log);

impl core::fmt::Debug for PrettyLog {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let bytes = self
            .0
            .data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<String>>()
            .join("");

        let message = String::from_utf8(self.0.data.clone()).ok();

        f.debug_struct("Log")
            .field("address", &self.0.address)
            .field("topics", &self.0.topics)
            .field("data", &bytes)
            .field("data_utf8", &message)
            .finish()
    }
}
