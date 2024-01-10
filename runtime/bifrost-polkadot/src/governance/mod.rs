// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

pub mod referenda;

use super::*;
pub use bifrost_runtime_common::dollar;
pub mod fellowship;
mod origins;
pub use origins::{
	custom_origins, CoreAdmin, Fellows, FellowshipAdmin, FellowshipExperts, FellowshipInitiates,
	FellowshipMasters, ReferendumCanceller, ReferendumKiller, SALPAdmin, TechAdmin,
	ValidatorElection, WhitelistedCaller, *,
};
mod tracks;
pub use tracks::TracksInfo;

pub type CoreAdminOrCouncil = EitherOfDiverse<
	CoreAdmin,
	EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>,
>;

pub type TechAdminOrCouncil = EitherOfDiverse<
	TechAdmin,
	EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>,
>;
