// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
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

use crate::AccountId;
use frame_support::traits::EitherOfDiverse;
use frame_system::EnsureRoot;

pub type OracleMembershipInst = pallet_membership::Instance1;
pub type MainCouncilMembershipInst = pallet_membership::Instance2;
pub type TechnicalCommitteeMembershipInst = pallet_membership::Instance3;
pub type CommunityCouncilMembershipInst = pallet_membership::Instance4;

// Leaving instance 1 for potentially having an oracle membership collective instance
pub type MainCouncilCollectiveInst = pallet_collective::Instance2;
pub type TechnicalCommitteeCollectiveInst = pallet_collective::Instance3;
pub type CommunityCouncilCollectiveInst = pallet_collective::Instance4;

pub type MainTreasuryInst = pallet_treasury::Instance1;
pub type CommunityTreasuryInst = pallet_treasury::Instance2;

// Main Council
pub type EnsureRootOrAllMainCouncil = EitherOfDiverse<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionAtLeast<AccountId, MainCouncilCollectiveInst, 1, 1>,
>;

pub type EnsureRootOrTwoThirdsMainCouncil = EitherOfDiverse<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionAtLeast<AccountId, MainCouncilCollectiveInst, 2, 3>,
>;

// Technical Committee
pub type EnsureRootOrAllTechnicalCommittee = EitherOfDiverse<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCommitteeCollectiveInst, 1, 1>,
>;

pub type EnsureRootOrTwoThirdsTechnicalCommittee = EitherOfDiverse<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCommitteeCollectiveInst, 2, 3>,
>;

// Community Council
pub type EnsureRootOrAllCommunityCouncil = EitherOfDiverse<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionAtLeast<AccountId, CommunityCouncilCollectiveInst, 1, 1>,
>;

pub type EnsureRootOrTwoThirdsCommunityCouncil = EitherOfDiverse<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionAtLeast<AccountId, CommunityCouncilCollectiveInst, 2, 3>,
>;
