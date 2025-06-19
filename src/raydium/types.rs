use crate::raydium::amm_types::Loadable;
use crate::raydium::amm_types::{RaydiumAmmInfo, RaydiumFees, RaydiumStateData};
use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use safe_transmute::trivial::TriviallyTransmutable;
use solana_sdk::pubkey::Pubkey;
#[macro_export]
macro_rules! impl_loadable {
    ($type_name:ident) => {
        unsafe impl Zeroable for $type_name {}
        unsafe impl Pod for $type_name {}
        unsafe impl TriviallyTransmutable for $type_name {}
        impl Loadable for $type_name {}
    };
}

#[derive(Copy, Clone, Debug, Default)]
pub enum ComputeUnitLimits {
    #[default]
    Dynamic,
    Fixed(u64),
}

#[derive(Copy, Clone, Debug)]
pub enum PriorityFeeConfig {
    DynamicMultiplier(u64),
    FixedCuPrice(u64),
    JitoTip(u64),
}

#[derive(Copy, Clone, Debug)]
pub struct SwapConfig {
    pub priority_fee: Option<PriorityFeeConfig>,
    pub cu_limits: Option<ComputeUnitLimits>,
    pub wrap_and_unwrap_sol: Option<bool>,
    pub as_legacy_transaction: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct SwapConfigOverrides {
    pub priority_fee: Option<PriorityFeeConfig>,
    pub cu_limits: Option<ComputeUnitLimits>,
    pub wrap_and_unwrap_sol: Option<bool>,
    pub destination_token_account: Option<Pubkey>,
    pub as_legacy_transaction: Option<bool>,
}

#[derive(Copy, Clone, Debug)]
pub struct SwapInput {
    pub input_token_mint: Pubkey,
    pub output_token_mint: Pubkey,
    pub slippage_bps: u16,
    pub amount: u64,
    pub mode: SwapExecutionMode,
    pub market: Option<Pubkey>,
}

#[derive(Copy, Clone, Debug)]
pub enum SwapExecutionMode {
    ExactIn,
    ExactOut,
}
impl SwapExecutionMode {
    pub fn amount_specified_is_input(&self) -> bool {
        matches!(self, SwapExecutionMode::ExactIn)
    }
}

#[derive(Default)]
pub struct RaydiumAmmExecutorOpts {
    pub priority_fee: Option<PriorityFeeConfig>,
    pub cu_limits: Option<ComputeUnitLimits>,
    pub wrap_and_unwrap_sol: Option<bool>,
    pub load_keys_by_api: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
pub struct AmmKeys {
    pub amm_pool: Pubkey,
    pub amm_coin_mint: Pubkey,
    pub amm_pc_mint: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_target: Pubkey,
    pub amm_coin_vault: Pubkey,
    pub amm_pc_vault: Pubkey,
    pub amm_lp_mint: Pubkey,
    pub amm_open_order: Pubkey,
    pub market_program: Pubkey,
    pub market: Pubkey,
    pub nonce: u8,
}
#[derive(Debug, Clone, Copy)]
pub struct MarketKeys {
    pub event_queue: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub coin_vault: Pubkey,
    pub pc_vault: Pubkey,
    pub vault_signer_key: Pubkey,
}
impl From<&crate::raydium::api_v3::response::pools::standard::MarketKeys> for MarketKeys {
    fn from(keys: &crate::raydium::api_v3::response::pools::standard::MarketKeys) -> Self {
        MarketKeys {
            event_queue: keys.market_event_queue,
            bids: keys.market_bids,
            asks: keys.market_asks,
            coin_vault: keys.market_base_vault,
            pc_vault: keys.market_quote_vault,
            vault_signer_key: keys.market_authority,
        }
    }
}
impl TryFrom<&crate::raydium::api_v3::response::ApiV3StandardPoolKeys> for MarketKeys {
    type Error = anyhow::Error;

    fn try_from(
        keys: &crate::raydium::api_v3::response::ApiV3StandardPoolKeys,
    ) -> Result<Self, Self::Error> {
        let keys = keys
            .keys
            .market
            .as_ref()
            .context("market keys should be present for amm")?;
        Ok(MarketKeys::from(keys))
    }
}

impl TryFrom<&crate::raydium::api_v3::response::ApiV3StandardPoolKeys> for AmmKeys {
    type Error = anyhow::Error;

    fn try_from(
        keys: &crate::raydium::api_v3::response::ApiV3StandardPoolKeys,
    ) -> Result<Self, Self::Error> {
        let market_keys = keys
            .keys
            .market
            .as_ref()
            .context("market keys should be present for amm")?;
        Ok(AmmKeys {
            amm_pool: keys.id,
            amm_coin_mint: keys.mint_a.address,
            amm_pc_mint: keys.mint_b.address,
            amm_authority: keys.keys.authority,
            amm_target: keys
                .keys
                .target_orders
                .context("target orders should be present for amm")?,
            amm_coin_vault: keys.vault.a,
            amm_pc_vault: keys.vault.b,
            amm_lp_mint: keys.keys.mint_lp.address,
            amm_open_order: keys
                .keys
                .open_orders
                .context("open orders should be present for amm")?,
            market_program: market_keys.market_program_id,
            market: market_keys.market_id,
            nonce: 0, // random
        })
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AmmInfo {
    /// Initialized status.
    pub status: u64,
    /// Nonce used in program address.
    /// The program address is created deterministically with the nonce,
    /// amm program id, and amm account pubkey.  This program address has
    /// authority over the amm's token coin account, token pc account, and pool
    /// token mint.
    pub nonce: u64,
    /// max order count
    pub order_num: u64,
    /// within this range, 5 => 5% range
    pub depth: u64,
    /// coin decimal
    pub coin_decimals: u64,
    /// pc decimal
    pub pc_decimals: u64,
    /// amm machine state
    pub state: u64,
    /// amm reset_flag
    pub reset_flag: u64,
    /// min size 1->0.000001
    pub min_size: u64,
    /// vol_max_cut_ratio numerator, sys_decimal_value as denominator
    pub vol_max_cut_ratio: u64,
    /// amount wave numerator, sys_decimal_value as denominator
    pub amount_wave: u64,
    /// coinLotSize 1 -> 0.000001
    pub coin_lot_size: u64,
    /// pcLotSize 1 -> 0.000001
    pub pc_lot_size: u64,
    /// min_cur_price: (2 * amm.order_num * amm.pc_lot_size) * max_price_multiplier
    pub min_price_multiplier: u64,
    /// max_cur_price: (2 * amm.order_num * amm.pc_lot_size) * max_price_multiplier
    pub max_price_multiplier: u64,
    /// system decimal value, used to normalize the value of coin and pc amount
    pub sys_decimal_value: u64,
    /// All fee information
    pub fees: RaydiumFees,
    /// Statistical data
    pub state_data: StateData,
    /// Coin vault
    pub coin_vault: Pubkey,
    /// Pc vault
    pub pc_vault: Pubkey,
    /// Coin vault mint
    pub coin_vault_mint: Pubkey,
    /// Pc vault mint
    pub pc_vault_mint: Pubkey,
    /// lp mint
    pub lp_mint: Pubkey,
    /// open_orders key
    pub open_orders: Pubkey,
    /// market key
    pub market: Pubkey,
    /// market program key
    pub market_program: Pubkey,
    /// target_orders key
    pub target_orders: Pubkey,
    /// padding
    pub padding1: [u64; 8],
    /// amm owner key
    pub amm_owner: Pubkey,
    /// pool lp amount
    pub lp_amount: u64,
    /// client order id
    pub client_order_id: u64,
    /// padding
    pub padding2: [u64; 2],
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StateData {
    /// delay to take pnl coin
    pub need_take_pnl_coin: u64,
    /// delay to take pnl pc
    pub need_take_pnl_pc: u64,
    /// total pnl pc
    pub total_pnl_pc: u64,
    /// total pnl coin
    pub total_pnl_coin: u64,
    /// ido pool open time
    pub pool_open_time: u64,
    /// padding for future updates
    pub padding: [u64; 2],
    /// switch from orderbookonly to init
    pub orderbook_to_init_time: u64,

    /// swap coin in amount
    pub swap_coin_in_amount: [u8; 16],
    /// swap pc out amount
    pub swap_pc_out_amount: [u8; 16],
    /// charge pc as swap fee while swap pc to coin
    pub swap_acc_pc_fee: u64,

    /// swap pc in amount
    pub swap_pc_in_amount: [u8; 16],
    /// swap coin out amount
    pub swap_coin_out_amount: [u8; 16],
    /// charge coin as swap fee while swap coin to pc
    pub swap_acc_coin_fee: u64,
}
impl From<StateData> for RaydiumStateData {
    fn from(value: StateData) -> Self {
        Self {
            need_take_pnl_coin: value.need_take_pnl_coin,
            need_take_pnl_pc: value.need_take_pnl_pc,
            total_pnl_coin: value.total_pnl_coin,
            total_pnl_pc: value.total_pnl_pc,
            pool_open_time: value.pool_open_time,
            padding: value.padding,
            orderbook_to_init_time: value.orderbook_to_init_time,
            swap_acc_pc_fee: value.swap_acc_pc_fee,
            swap_acc_coin_fee: value.swap_acc_coin_fee,
            swap_coin_in_amount: u128::from_le_bytes(value.swap_coin_in_amount),
            swap_pc_out_amount: u128::from_le_bytes(value.swap_pc_out_amount),
            swap_pc_in_amount: u128::from_le_bytes(value.swap_pc_in_amount),
            swap_coin_out_amount: u128::from_le_bytes(value.swap_coin_out_amount),
        }
    }
}

impl From<AmmInfo> for RaydiumAmmInfo {
    fn from(value: AmmInfo) -> Self {
        RaydiumAmmInfo {
            status: value.status,
            nonce: value.nonce,
            order_num: value.order_num,
            depth: value.depth,
            coin_decimals: value.coin_decimals,
            pc_decimals: value.pc_decimals,
            state: value.state,
            reset_flag: value.reset_flag,
            min_size: value.min_size,
            vol_max_cut_ratio: value.vol_max_cut_ratio,
            amount_wave: value.amount_wave,
            coin_lot_size: value.coin_lot_size,
            pc_lot_size: value.pc_lot_size,
            min_price_multiplier: value.min_price_multiplier,
            max_price_multiplier: value.max_price_multiplier,
            sys_decimal_value: value.sys_decimal_value,
            fees: value.fees,
            state_data: value.state_data.into(),
            coin_vault: value.coin_vault,
            pc_vault: value.pc_vault,
            coin_vault_mint: value.coin_vault_mint,
            pc_vault_mint: value.pc_vault_mint,
            lp_mint: value.lp_mint,
            open_orders: value.open_orders,
            market: value.market,
            market_program: value.market_program,
            target_orders: value.target_orders,
            padding1: value.padding1,
            amm_owner: value.amm_owner,
            lp_amount: value.lp_amount,
            client_order_id: value.client_order_id,
            padding2: value.padding2,
        }
    }
}

impl_loadable!(AmmInfo);
