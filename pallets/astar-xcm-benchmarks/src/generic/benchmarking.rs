// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

// Copyright (C) Parity Technologies (UK) Ltd.

use super::*;
use crate::{new_executor, XcmCallOf};
use frame_benchmarking::v2::*;
use frame_support::dispatch::GetDispatchInfo;
use parity_scale_codec::Encode;
use sp_std::vec;
use xcm::{
    latest::{prelude::*, MaxDispatchErrorLen, MaybeErrorCode, Weight},
    DoubleEncoded,
};
use xcm_executor::{ExecutorError, FeesMode};

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn report_holding() -> Result<(), BenchmarkError> {
        let holding = T::worst_case_holding(0);

        let mut executor = new_executor::<T>(Default::default());
        executor.set_holding(holding.clone().into());

        let instruction = Instruction::<XcmCallOf<T>>::ReportHolding {
            response_info: QueryResponseInfo {
                destination: T::valid_destination()?,
                query_id: Default::default(),
                max_weight: Weight::MAX,
            },
            // Worst case is looking through all holdings for every asset explicitly.
            assets: Definite(holding),
        };
        let xcm = Xcm(vec![instruction]);

        #[block]
        {
            executor.bench_process(xcm)?;
        }
        // The completion of execution above is enough to validate this is completed.
        Ok(())
    }

    #[benchmark]
    fn buy_execution() -> Result<(), BenchmarkError> {
        let holding = T::worst_case_holding(0).into();

        let mut executor = new_executor::<T>(Default::default());
        executor.set_holding(holding);

        let fee_asset = Concrete(Here.into());

        let instruction = Instruction::<XcmCallOf<T>>::BuyExecution {
            fees: (fee_asset, 100_000_000_000u128).into(), // should be something inside of holding
            weight_limit: WeightLimit::Limited(Weight::from_parts(1u64, 64 * 1024)),
        };

        let xcm = Xcm(vec![instruction]);

        #[block]
        {
            executor.bench_process(xcm)?;
        }
        Ok(())
    }

    #[benchmark]
    fn query_response() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());
        let (query_id, response) = T::worst_case_response();
        let max_weight = Weight::MAX;
        let querier: Option<MultiLocation> = Some(Here.into());
        let instruction = Instruction::QueryResponse {
            query_id,
            response,
            max_weight,
            querier,
        };
        let xcm = Xcm(vec![instruction]);

        #[block]
        {
            executor.bench_process(xcm)?;
        }
        Ok(())
    }

    // We don't care about the call itself, since that is accounted for in the weight parameter
    // and included in the final weight calculation. So this is just the overhead of submitting
    // a noop call.
    #[benchmark]
    fn transact() -> Result<(), BenchmarkError> {
        let (origin, noop_call) = T::transact_origin_and_runtime_call()?;
        let mut executor = new_executor::<T>(origin);
        let double_encoded_noop_call: DoubleEncoded<_> = noop_call.encode().into();

        let instruction = Instruction::Transact {
            origin_kind: OriginKind::SovereignAccount,
            require_weight_at_most: noop_call.get_dispatch_info().weight,
            call: double_encoded_noop_call,
        };
        let xcm = Xcm(vec![instruction]);

        #[block]
        {
            executor.bench_process(xcm)?;
        }
        // The assert above is enough to show this XCM succeeded
        Ok(())
    }

    #[benchmark]
    fn refund_surplus() -> Result<(), BenchmarkError> {
        let holding = T::worst_case_holding(0).into();
        let mut executor = new_executor::<T>(Default::default());
        executor.set_holding(holding);
        executor.set_total_surplus(Weight::from_parts(1337, 1337));
        executor.set_total_refunded(Weight::zero());

        let instruction = Instruction::<XcmCallOf<T>>::RefundSurplus;
        let xcm = Xcm(vec![instruction]);

        #[block]
        {
            executor.bench_process(xcm)?;
        }

        assert_eq!(executor.total_surplus(), &Weight::from_parts(1337, 1337));
        assert_eq!(executor.total_refunded(), &Weight::from_parts(1337, 1337));
        Ok(())
    }

    #[benchmark]
    fn set_error_handler() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());
        let instruction = Instruction::<XcmCallOf<T>>::SetErrorHandler(Xcm(vec![]));
        let xcm = Xcm(vec![instruction]);

        #[block]
        {
            executor.bench_process(xcm)?;
        }
        assert_eq!(executor.error_handler(), &Xcm(vec![]));
        Ok(())
    }

    #[benchmark]
    fn set_appendix() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());
        let appendix = Xcm(vec![]);
        let instruction = Instruction::<XcmCallOf<T>>::SetAppendix(appendix);
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        assert_eq!(executor.appendix(), &Xcm(vec![]));
        Ok(())
    }

    #[benchmark]
    fn clear_error() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());
        executor.set_error(Some((5u32, XcmError::Overflow)));
        let instruction = Instruction::<XcmCallOf<T>>::ClearError;
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        assert!(executor.error().is_none());
        Ok(())
    }

    #[benchmark]
    fn descend_origin() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());
        let who = X2(OnlyChild, OnlyChild);
        let instruction = Instruction::DescendOrigin(who.clone());
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        assert_eq!(
            executor.origin(),
            &Some(MultiLocation {
                parents: 0,
                interior: who,
            }),
        );
        Ok(())
    }

    #[benchmark]
    fn clear_origin() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());
        let instruction = Instruction::ClearOrigin;
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        assert_eq!(executor.origin(), &None);
        Ok(())
    }

    #[benchmark]
    fn report_error() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());
        executor.set_error(Some((0u32, XcmError::Unimplemented)));
        let query_id = Default::default();
        let destination = T::valid_destination().map_err(|_| BenchmarkError::Skip)?;
        let max_weight = Default::default();

        let instruction = Instruction::ReportError(QueryResponseInfo {
            query_id,
            destination,
            max_weight,
        });
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        // the execution succeeding is all we need to verify this xcm was successful
        Ok(())
    }

    #[benchmark]
    fn claim_asset() -> Result<(), BenchmarkError> {
        use xcm_executor::traits::DropAssets;

        let (origin, ticket, assets) = T::claimable_asset()?;

        // We place some items into the asset trap to claim.
        <T::XcmConfig as xcm_executor::Config>::AssetTrap::drop_assets(
            &origin,
            assets.clone().into(),
            &XcmContext {
                origin: Some(origin.clone()),
                message_hash: [0; 32],
                topic: None,
            },
        );

        // Assets should be in the trap now.

        let mut executor = new_executor::<T>(origin);
        let instruction = Instruction::ClaimAsset {
            assets: assets.clone(),
            ticket,
        };
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        assert!(executor.holding().ensure_contains(&assets).is_ok());
        Ok(())
    }

    #[benchmark]
    fn trap() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());
        let instruction = Instruction::Trap(10);
        let xcm = Xcm(vec![instruction]);
        // In order to access result in the verification below, it needs to be defined here.
        let mut _result = Ok(());

        #[block]
        {
            _result = executor.bench_process(xcm);
        }
        assert!(matches!(
            _result,
            Err(ExecutorError {
                xcm_error: XcmError::Trap(10),
                ..
            })
        ));
        Ok(())
    }

    #[benchmark]
    fn subscribe_version() -> Result<(), BenchmarkError> {
        use xcm_executor::traits::VersionChangeNotifier;
        let origin = T::subscribe_origin()?;
        let query_id = Default::default();
        let max_response_weight = Default::default();
        let mut executor = new_executor::<T>(origin.clone());
        let instruction = Instruction::SubscribeVersion {
            query_id,
            max_response_weight,
        };
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        assert!(
            <T::XcmConfig as xcm_executor::Config>::SubscriptionService::is_subscribed(&origin)
        );
        Ok(())
    }

    #[benchmark]
    fn unsubscribe_version() -> Result<(), BenchmarkError> {
        use xcm_executor::traits::VersionChangeNotifier;
        // First we need to subscribe to notifications.
        let origin = T::subscribe_origin()?;
        let query_id = Default::default();
        let max_response_weight = Default::default();
        <T::XcmConfig as xcm_executor::Config>::SubscriptionService::start(
            &origin,
            query_id,
            max_response_weight,
            &XcmContext {
                origin: Some(origin.clone()),
                message_hash: [0; 32],
                topic: None,
            },
        )
        .map_err(|_| "Could not start subscription")?;
        assert!(
            <T::XcmConfig as xcm_executor::Config>::SubscriptionService::is_subscribed(&origin)
        );

        let mut executor = new_executor::<T>(origin.clone());
        let instruction = Instruction::UnsubscribeVersion;
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        assert!(
            !<T::XcmConfig as xcm_executor::Config>::SubscriptionService::is_subscribed(&origin)
        );
        Ok(())
    }

    #[benchmark]
    fn initiate_reserve_withdraw() -> Result<(), BenchmarkError> {
        let holding = T::worst_case_holding(1);
        let assets_filter = MultiAssetFilter::Definite(holding.clone());
        let reserve = T::valid_destination().map_err(|_| BenchmarkError::Skip)?;
        let mut executor = new_executor::<T>(Default::default());
        executor.set_holding(holding.into());
        let instruction = Instruction::InitiateReserveWithdraw {
            assets: assets_filter,
            reserve,
            xcm: Xcm(vec![]),
        };
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        // The execute completing successfully is as good as we can check.
        Ok(())
    }

    #[benchmark]
    fn burn_asset() -> Result<(), BenchmarkError> {
        let holding = T::worst_case_holding(0);
        let assets = holding.clone();

        let mut executor = new_executor::<T>(Default::default());
        executor.set_holding(holding.into());

        let instruction = Instruction::BurnAsset(assets.into());
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        assert!(executor.holding().is_empty());
        Ok(())
    }

    #[benchmark]
    fn expect_asset() -> Result<(), BenchmarkError> {
        let holding = T::worst_case_holding(0);
        let assets = holding.clone();

        let mut executor = new_executor::<T>(Default::default());
        executor.set_holding(holding.into());

        let instruction = Instruction::ExpectAsset(assets.into());
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        // `execute` completing successfully is as good as we can check.
        Ok(())
    }

    #[benchmark]
    fn expect_origin() -> Result<(), BenchmarkError> {
        let expected_origin = Parent.into();
        let mut executor = new_executor::<T>(Default::default());

        let instruction = Instruction::ExpectOrigin(Some(expected_origin));
        let xcm = Xcm(vec![instruction]);
        let mut _result = Ok(());
        #[block]
        {
            _result = executor.bench_process(xcm);
        }
        assert!(matches!(
            _result,
            Err(ExecutorError {
                xcm_error: XcmError::ExpectationFalse,
                ..
            })
        ));
        Ok(())
    }

    #[benchmark]
    fn expect_error() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());
        executor.set_error(Some((3u32, XcmError::Overflow)));

        let instruction = Instruction::ExpectError(None);
        let xcm = Xcm(vec![instruction]);
        let mut _result = Ok(());
        #[block]
        {
            _result = executor.bench_process(xcm);
        }
        assert!(matches!(
            _result,
            Err(ExecutorError {
                xcm_error: XcmError::ExpectationFalse,
                ..
            })
        ));
        Ok(())
    }

    #[benchmark]
    fn expect_transact_status() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());
        let worst_error =
            || -> MaybeErrorCode { vec![0; MaxDispatchErrorLen::get() as usize].into() };
        executor.set_transact_status(worst_error());

        let instruction = Instruction::ExpectTransactStatus(worst_error());
        let xcm = Xcm(vec![instruction]);
        let mut _result = Ok(());
        #[block]
        {
            _result = executor.bench_process(xcm);
        }
        assert!(matches!(_result, Ok(..)));
        Ok(())
    }

    #[benchmark]
    fn query_pallet() -> Result<(), BenchmarkError> {
        let query_id = Default::default();
        let destination = T::valid_destination().map_err(|_| BenchmarkError::Skip)?;
        let max_weight = Default::default();
        let mut executor = new_executor::<T>(Default::default());

        let instruction = Instruction::QueryPallet {
            module_name: b"frame_system".to_vec(),
            response_info: QueryResponseInfo {
                destination,
                query_id,
                max_weight,
            },
        };
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        Ok(())
    }

    #[benchmark]
    fn expect_pallet() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());

        let instruction = Instruction::ExpectPallet {
            index: 10,
            name: b"System".to_vec(),
            module_name: b"frame_system".to_vec(),
            crate_major: 4,
            min_crate_minor: 0,
        };
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        // the execution succeeding is all we need to verify this xcm was successful
        Ok(())
    }

    #[benchmark]
    fn report_transact_status() -> Result<(), BenchmarkError> {
        let query_id = Default::default();
        let destination = T::valid_destination().map_err(|_| BenchmarkError::Skip)?;
        let max_weight = Default::default();

        let mut executor = new_executor::<T>(Default::default());
        executor.set_transact_status(b"MyError".to_vec().into());

        let instruction = Instruction::ReportTransactStatus(QueryResponseInfo {
            query_id,
            destination,
            max_weight,
        });
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        Ok(())
    }

    #[benchmark]
    fn clear_transact_status() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());
        executor.set_transact_status(b"MyError".to_vec().into());

        let instruction = Instruction::ClearTransactStatus;
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        assert_eq!(executor.transact_status(), &MaybeErrorCode::Success);
        Ok(())
    }

    #[benchmark]
    fn set_topic() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());

        let instruction = Instruction::SetTopic([1; 32]);
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        assert_eq!(executor.topic(), &Some([1; 32]));
        Ok(())
    }

    #[benchmark]
    fn clear_topic() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());
        executor.set_topic(Some([2; 32]));

        let instruction = Instruction::ClearTopic;
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        assert_eq!(executor.topic(), &None);
        Ok(())
    }

    #[benchmark]
    fn set_fees_mode() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());
        executor.set_fees_mode(FeesMode {
            jit_withdraw: false,
        });

        let instruction = Instruction::SetFeesMode { jit_withdraw: true };
        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        assert_eq!(executor.fees_mode(), &FeesMode { jit_withdraw: true });
        Ok(())
    }

    #[benchmark]
    fn unpaid_execution() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());
        executor.set_origin(Some(Here.into()));

        let instruction = Instruction::<XcmCallOf<T>>::UnpaidExecution {
            weight_limit: WeightLimit::Unlimited,
            check_origin: Some(Here.into()),
        };

        let xcm = Xcm(vec![instruction]);
        #[block]
        {
            executor.bench_process(xcm)?;
        }
        Ok(())
    }

    #[benchmark]
    fn exchange_asset() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    #[benchmark]
    fn export_message() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    #[benchmark]
    fn lock_asset() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    #[benchmark]
    fn unlock_asset() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    #[benchmark]
    fn note_unlockable() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    #[benchmark]
    fn request_unlock() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    #[benchmark]
    fn universal_origin() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    impl_benchmark_test_suite!(
        Pallet,
        crate::generic::mock::new_test_ext(),
        crate::generic::mock::Test
    );
}
