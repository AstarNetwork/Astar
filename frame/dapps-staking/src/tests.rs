use super::{Event, *};
use mock::*;

use frame_support::{assert_noop, assert_ok};

#[test]
fn assert_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash_id = Origin::signed(1);
        let controller_id = 2u64;

        assert_ok!(DappStaking::bond(
            stash_id,
            controller_id,
            50,
            crate::RewardDestination::Staked
        ));
        // let bond_event = Event::pallet_dapps_staking(RawEvent::Bonded(stash_id, 50));
        println!("{:?}", System::events()[0].event);
        assert_eq!(get_dapp_staking_events()[0], Event::Bonded(1, 50));
    })
}

// pub fn bond(
//     origin: OriginFor<T>,
//     controller: <T::Lookup as StaticLookup>::Source,
//     #[pallet::compact] value: BalanceOf<T>,
//     payee: RewardDestination<T::AccountId>,
// ) -> DispatchResultWithPostInfo {
