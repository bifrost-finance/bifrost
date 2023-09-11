use crate::InterestRateModel;
use frame_support::pallet_prelude::*;
use primitives::{CurrencyId, Rate, Ratio};
use scale_info::TypeInfo;

/// Container for borrow balance information
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, Default, TypeInfo)]
pub struct BorrowSnapshot<Balance> {
    /// Principal Total balance (with accrued interest), after applying the most recent balance-changing action
    pub principal: Balance,
    /// InterestIndex Global borrowIndex as of the most recent balance-changing action
    pub borrow_index: Rate,
}

/// Container for earned amount information
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, Default, TypeInfo)]
pub struct EarnedSnapshot<Balance> {
    /// Total deposit interest, after applying the most recent balance-changing action
    pub total_earned_prior: Balance,
    /// Exchange rate, after applying the most recent balance-changing action
    pub exchange_rate_prior: Rate,
}

/// Deposit information
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, Default, TypeInfo)]
pub struct Deposits<Balance> {
    /// The voucher amount of the deposit
    pub voucher_balance: Balance,
    /// Can this deposit be used as collateral
    pub is_collateral: bool,
}

/// The current state of a market. For more information, see [Market].
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, PartialEq, Eq, codec::Decode, codec::Encode, RuntimeDebug, TypeInfo)]
pub enum MarketState {
    Active,
    Pending,
    Supervision,
}

/// Market.
///
/// A large pool of liquidity where accounts can lend and borrow.
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, PartialEq, Eq, codec::Decode, codec::Encode, RuntimeDebug, TypeInfo)]
pub struct Market<Balance> {
    /// The collateral utilization ratio
    pub collateral_factor: Ratio,
    /// A liquidation_threshold ratio more than collateral_factor to avoid liquidate_borrow too casual
    pub liquidation_threshold: Ratio,
    /// Fraction of interest currently set aside for reserves
    pub reserve_factor: Ratio,
    /// The percent, ranging from 0% to 100%, of a liquidatable account's
    /// borrow that can be repaid in a single liquidate transaction.
    pub close_factor: Ratio,
    /// Liquidation incentive ratio
    pub liquidate_incentive: Rate,
    /// Liquidation incentive reserved ratio
    pub liquidate_incentive_reserved_factor: Ratio,
    /// Current interest rate model being used
    pub rate_model: InterestRateModel,
    /// Current market state
    pub state: MarketState,
    /// Upper bound of supplying
    pub supply_cap: Balance,
    /// Upper bound of borrowing
    pub borrow_cap: Balance,
    /// Ptoken asset id
    pub ptoken_id: CurrencyId,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
pub struct RewardMarketState<BlockNumber, Balance> {
    pub index: Balance,
    /// total amount of staking asset user deposited
    pub block: BlockNumber,
}
