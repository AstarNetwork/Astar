//! XCM Version Negotiation Test
//!
//! The test requires:
//! 1. `SubscriptionService = PolkadotXcm` in XcmConfig (handles SubscribeVersion)
//! 2. `ResponseHandler = PolkadotXcm` in XcmConfig (handles QueryResponse)
//! 3. `TrailingSetTopicAsId` wrapping the barrier (uses SetTopic as message ID)
//! 4. `WithComputedOrigin` containing `AllowSubscriptionsFrom` (handles sibling origins)
//! 5. `AllowKnownQueryResponses<PolkadotXcm>` allows responses to pending queries

use crate::mocks::msg_queue::mock_msg_queue;
use crate::mocks::{parachain, *};
use frame_support::{assert_ok, weights::Weight};
use sp_core::H256;
use xcm::prelude::*;
use xcm_simulator::TestExt;

/// Test SubscribeVersion from a system parachain succeeds.
/// Verifies message ID matches SetTopic value.
#[test]
fn system_parachain_subscribe_version_succeeds() {
    MockNet::reset();

    let topic_id: [u8; 32] = [0x8B; 32];
    let expected_message_id = H256::from(topic_id);

    // ParaAssetHub (1000) -> ParaA (1)
    ParaAssetHub::execute_with(|| {
        let message = Xcm::<()>(vec![
            SubscribeVersion {
                query_id: 4,
                max_response_weight: Weight::from_parts(0, 0),
            },
            SetTopic(topic_id),
        ]);
        assert_ok!(ParachainPalletXcm::send_xcm(
            Here,
            (Parent, Parachain(1)),
            message
        ));
    });

    ParaA::execute_with(|| {
        use parachain::{RuntimeEvent, System};
        let events = System::events();

        // Verify success with correct message_id (SetTopic value)
        let success = events.iter().find_map(|r| match &r.event {
            RuntimeEvent::MsgQueue(mock_msg_queue::Event::Success { message_id }) => {
                message_id.as_ref()
            }
            _ => None,
        });
        assert_eq!(
            success,
            Some(&expected_message_id),
            "Message ID must be SetTopic value"
        );

        // Verify subscription registered for correct destination
        assert!(
            events.iter().any(|r| matches!(
                &r.event,
                RuntimeEvent::PolkadotXcm(pallet_xcm::Event::VersionNotifyStarted {
                    destination, ..
                }) if *destination == Location::new(1, [Parachain(1000)])
            )),
            "Subscription must be registered for para 1000"
        );
    });
}

/// Test QueryResponse for version negotiation.
///
/// This simulates the full version negotiation flow:
/// 1. ParaA subscribes to ParaAssetHub's version (creates pending query)
/// 2. ParaAssetHub sends QueryResponse back to ParaA
#[test]
fn query_response_version_negotiation_succeeds() {
    MockNet::reset();

    let response_topic: [u8; 32] = [0xBB; 32];
    let expected_response_id = H256::from(response_topic);

    // Step 1: ParaA subscribes to ParaAssetHub's version updates
    // This creates a pending query on ParaA and triggers automatic QueryResponse
    ParaA::execute_with(|| {
        let dest: Location = (Parent, Parachain(1000)).into();
        assert_ok!(ParachainPalletXcm::force_subscribe_version_notify(
            parachain::RuntimeOrigin::root(),
            Box::new(dest.into()),
        ));
    });

    // Step 2: Send additional QueryResponse to verify our SetTopic handling explicitly
    ParaAssetHub::execute_with(|| {
        let message = Xcm::<()>(vec![
            QueryResponse {
                query_id: 0,
                response: Response::Version(5),
                max_weight: Weight::from_parts(0, 0),
                querier: Some(Location::new(1, [Parachain(1)])),
            },
            SetTopic(response_topic),
        ]);
        assert_ok!(ParachainPalletXcm::send_xcm(
            Here,
            (Parent, Parachain(1)),
            message
        ));
    });

    // Verify ParaA processed the QueryResponse successfully
    ParaA::execute_with(|| {
        use parachain::{RuntimeEvent, System};
        let events = System::events();

        println!("=== Events on ParaA ===");
        for (i, event) in events.iter().enumerate() {
            println!("[{}] {:?}", i, event.event);
        }

        // Verify our QueryResponse with specific topic ID was processed successfully
        let has_success_with_our_topic = events.iter().any(|r| {
            matches!(
                &r.event,
                RuntimeEvent::MsgQueue(mock_msg_queue::Event::Success { message_id: Some(id) })
                if *id == expected_response_id
            )
        });
        assert!(
            has_success_with_our_topic,
            "QueryResponse with SetTopic {:?} must succeed",
            expected_response_id
        );

        // Verify 2 SupportedVersionChanged were emitted (the automatic one from force_subscribe_version_notify & our QueryResponse manually sent)
        let version_changed_count = events
            .iter()
            .filter(|r| {
                matches!(
                    &r.event,
                    RuntimeEvent::PolkadotXcm(pallet_xcm::Event::SupportedVersionChanged { .. })
                )
            })
            .count();
        assert_eq!(
            version_changed_count, 2,
            "2 SupportedVersionChanged events must be emitted"
        );
    });
}
