// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use primitives::{Rate, Ratio};
use scale_info::TypeInfo;
use sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedSub, Saturating};

use crate::*;

/// Parallel interest rate model
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo)]
pub enum InterestRateModel {
    Jump(JumpModel),
    Curve(CurveModel),
}

impl Default for InterestRateModel {
    fn default() -> Self {
        Self::new_jump_model(
            Rate::saturating_from_rational(2, 100),
            Rate::saturating_from_rational(10, 100),
            Rate::saturating_from_rational(32, 100),
            Ratio::from_percent(80),
        )
    }
}

impl InterestRateModel {
    pub fn new_jump_model(
        base_rate: Rate,
        jump_rate: Rate,
        full_rate: Rate,
        jump_utilization: Ratio,
    ) -> Self {
        Self::Jump(JumpModel::new_model(
            base_rate,
            jump_rate,
            full_rate,
            jump_utilization,
        ))
    }

    pub fn new_curve_model(base_rate: Rate) -> Self {
        Self::Curve(CurveModel::new_model(base_rate))
    }

    pub fn check_model(&self) -> bool {
        match self {
            Self::Jump(jump) => jump.check_model(),
            Self::Curve(curve) => curve.check_model(),
        }
    }

    /// Calculates the current borrow interest rate
    pub fn get_borrow_rate(&self, utilization: Ratio) -> Option<Rate> {
        match self {
            Self::Jump(jump) => jump.get_borrow_rate(utilization),
            Self::Curve(curve) => curve.get_borrow_rate(utilization),
        }
    }

    /// Calculates the current supply interest rate
    pub fn get_supply_rate(borrow_rate: Rate, util: Ratio, reserve_factor: Ratio) -> Rate {
        // ((1 - reserve_factor) * borrow_rate) * utilization
        let one_minus_reserve_factor = Ratio::one().saturating_sub(reserve_factor);
        let rate_to_pool = borrow_rate.saturating_mul(one_minus_reserve_factor.into());

        rate_to_pool.saturating_mul(util.into())
    }
}

/// The jump interest rate model
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, Default, TypeInfo)]
pub struct JumpModel {
    /// The base interest rate when utilization rate is 0
    pub base_rate: Rate,
    /// The interest rate on jump utilization point
    pub jump_rate: Rate,
    /// The max interest rate when utilization rate is 100%
    pub full_rate: Rate,
    /// The utilization point at which the jump_rate is applied
    pub jump_utilization: Ratio,
}

impl JumpModel {
    pub const MAX_BASE_RATE: Rate = Rate::from_inner(100_000_000_000_000_000); // 10%
    pub const MAX_JUMP_RATE: Rate = Rate::from_inner(300_000_000_000_000_000); // 30%
    pub const MAX_FULL_RATE: Rate = Rate::from_inner(500_000_000_000_000_000); // 50%

    /// Create a new rate model
    pub fn new_model(
        base_rate: Rate,
        jump_rate: Rate,
        full_rate: Rate,
        jump_utilization: Ratio,
    ) -> JumpModel {
        Self {
            base_rate,
            jump_rate,
            full_rate,
            jump_utilization,
        }
    }

    /// Check the jump model for sanity
    pub fn check_model(&self) -> bool {
        if self.base_rate > Self::MAX_BASE_RATE
            || self.jump_rate > Self::MAX_JUMP_RATE
            || self.full_rate > Self::MAX_FULL_RATE
        {
            return false;
        }
        if self.base_rate > self.jump_rate || self.jump_rate > self.full_rate {
            return false;
        }

        true
    }

    /// Calculates the borrow interest rate of jump model
    pub fn get_borrow_rate(&self, utilization: Ratio) -> Option<Rate> {
        if utilization <= self.jump_utilization {
            // utilization * (jump_rate - zero_rate) / jump_utilization + zero_rate
            let result = self
                .jump_rate
                .checked_sub(&self.base_rate)?
                .saturating_mul(utilization.into())
                .checked_div(&self.jump_utilization.into())?
                .checked_add(&self.base_rate)?;

            Some(result)
        } else {
            // (utilization - jump_utilization)*(full_rate - jump_rate) / ( 1 - jump_utilization) + jump_rate
            let excess_util = utilization.saturating_sub(self.jump_utilization);
            let result = self
                .full_rate
                .checked_sub(&self.jump_rate)?
                .saturating_mul(excess_util.into())
                .checked_div(&(Ratio::one().saturating_sub(self.jump_utilization).into()))?
                .checked_add(&self.jump_rate)?;

            Some(result)
        }
    }
}

/// The curve interest rate model
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, Default, TypeInfo)]
pub struct CurveModel {
    pub base_rate: Rate,
}

impl CurveModel {
    pub const MAX_BASE_RATE: Rate = Rate::from_inner(Rate::DIV / 100 * 10); // 10%

    /// Create a new curve model
    pub fn new_model(base_rate: Rate) -> CurveModel {
        Self { base_rate }
    }

    /// Check the curve model for sanity
    pub fn check_model(&self) -> bool {
        self.base_rate <= Self::MAX_BASE_RATE
    }

    /// Calculates the borrow interest rate of curve model
    pub fn get_borrow_rate(&self, utilization: Ratio) -> Option<Rate> {
        const NINE: usize = 9;
        let utilization_rate: Rate = utilization.into();
        utilization_rate
            .saturating_pow(NINE)
            .checked_add(&self.base_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_runtime::FixedU128;

    // Test jump model
    #[test]
    fn init_jump_model_works() {
        let base_rate = Rate::saturating_from_rational(2, 100);
        let jump_rate = Rate::saturating_from_rational(10, 100);
        let full_rate = Rate::saturating_from_rational(32, 100);
        let jump_utilization = Ratio::from_percent(80);

        assert_eq!(
            JumpModel::new_model(base_rate, jump_rate, full_rate, jump_utilization),
            JumpModel {
                base_rate: Rate::from_inner(20_000_000_000_000_000).into(),
                jump_rate: Rate::from_inner(100_000_000_000_000_000).into(),
                full_rate: Rate::from_inner(320_000_000_000_000_000).into(),
                jump_utilization: Ratio::from_percent(80),
            }
        );
    }

    #[test]
    fn get_borrow_rate_works() {
        // init
        let base_rate = Rate::saturating_from_rational(2, 100);
        let jump_rate = Rate::saturating_from_rational(10, 100);
        let full_rate = Rate::saturating_from_rational(32, 100);
        let jump_utilization = Ratio::from_percent(80);
        let jump_model = JumpModel::new_model(base_rate, jump_rate, full_rate, jump_utilization);
        assert!(jump_model.check_model());

        // normal rate
        let mut cash: u128 = 500;
        let borrows: u128 = 1000;
        let util = Ratio::from_rational(borrows, cash + borrows);
        let borrow_rate = jump_model.get_borrow_rate(util).unwrap();
        assert_eq!(
            borrow_rate,
            jump_model.jump_rate.saturating_mul(util.into()) + jump_model.base_rate,
        );

        // jump rate
        cash = 100;
        let util = Ratio::from_rational(borrows, cash + borrows);
        let borrow_rate = jump_model.get_borrow_rate(util).unwrap();
        let normal_rate =
            jump_model.jump_rate.saturating_mul(jump_utilization.into()) + jump_model.base_rate;
        let excess_util = util.saturating_sub(jump_utilization);
        assert_eq!(
            borrow_rate,
            (jump_model.full_rate - jump_model.jump_rate).saturating_mul(excess_util.into())
                / FixedU128::saturating_from_rational(20, 100)
                + normal_rate,
        );
    }

    // Test curve model
    // TODO: Add test cases for curve model

    #[test]
    fn get_supply_rate_works() {
        let borrow_rate = Rate::saturating_from_rational(2, 100);
        let util = Ratio::from_percent(50);
        let reserve_factor = Ratio::zero();
        let supply_rate = InterestRateModel::get_supply_rate(borrow_rate, util, reserve_factor);
        assert_eq!(
            supply_rate,
            borrow_rate
                .saturating_mul(((Ratio::one().saturating_sub(reserve_factor)) * util).into()),
        );
    }

    #[test]
    fn curve_model_correctly_calculates_borrow_rate() {
        let model = CurveModel::new_model(Rate::saturating_from_rational(2, 100));
        assert_eq!(
            model.get_borrow_rate(Ratio::from_percent(80)).unwrap(),
            Rate::from_inner(154217728000000000)
        );
    }
}
