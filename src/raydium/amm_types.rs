use crate::impl_loadable;
use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::Zeroable;
use bytemuck::{Pod, from_bytes, from_bytes_mut};
use safe_transmute::TriviallyTransmutable;
use solana_sdk::account_info::AccountInfo;
use solana_sdk::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use std::cell::{Ref, RefMut};

pub trait Loadable: Pod {
    fn load_mut<'a>(account: &'a AccountInfo) -> Result<RefMut<'a, Self>, ProgramError> {
        Ok(RefMut::map(account.try_borrow_mut_data()?, |data| {
            from_bytes_mut(data)
        }))
    }
    fn load<'a>(account: &'a AccountInfo) -> Result<Ref<'a, Self>, ProgramError> {
        Ok(Ref::map(account.try_borrow_data()?, |data| {
            from_bytes(data)
        }))
    }

    fn load_from_bytes(data: &[u8]) -> Result<&Self, ProgramError> {
        Ok(from_bytes(data))
    }
}
pub const TEN_THOUSAND: u64 = 10000;
pub const MAX_ORDER_LIMIT: usize = 10;
#[derive(Clone, Copy, Default)]
struct TargetOrder {
    pub price: u64,
    pub vol: u64,
}

#[derive(Clone, Copy)]
pub struct RaydiumTargetOrders {
    pub owner: [u64; 4],
    pub buy_orders: [TargetOrder; 50],
    pub padding1: [u64; 8],
    pub target_x: u128,
    pub target_y: u128,
    pub plan_x_buy: u128,
    pub plan_y_buy: u128,
    pub plan_x_sell: u128,
    pub plan_y_sell: u128,
    pub placed_x: u128,
    pub placed_y: u128,
    pub calc_pnl_x: u128,
    pub calc_pnl_y: u128,
    pub sell_orders: [TargetOrder; 50],
    pub padding2: [u64; 6],
    pub replace_buy_client_id: [u64; MAX_ORDER_LIMIT],
    pub replace_sell_client_id: [u64; MAX_ORDER_LIMIT],
    pub last_order_numerator: u64,
    pub last_order_denominator: u64,

    pub plan_orders_cur: u64,
    pub place_orders_cur: u64,

    pub valid_buy_order_num: u64,
    pub valid_sell_order_num: u64,

    pub padding3: [u64; 10],

    pub free_slot_bits: u128,
}
#[cfg(target_endian = "little")]
unsafe impl Zeroable for MarketState {}

impl_loadable!(RaydiumTargetOrders);
#[derive(Clone, Copy, Default, PartialEq, Debug, BorshDeserialize)]
pub struct RaydiumAmmInfo {
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
    pub state_data: RaydiumStateData,
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
impl_loadable!(RaydiumAmmInfo);

#[derive(Clone, Copy, Debug, Default, PartialEq, BorshDeserialize)]
pub struct RaydiumFees {
    /// numerator of the min_separate
    pub min_separate_numerator: u64,
    /// denominator of the min_separate
    pub min_separate_denominator: u64,

    /// numerator of the fee
    pub trade_fee_numerator: u64,
    /// denominator of the fee
    /// and 'trade_fee_denominator' must be equal to 'min_separate_denominator'
    pub trade_fee_denominator: u64,

    /// numerator of the pnl
    pub pnl_numerator: u64,
    /// denominator of the pnl
    pub pnl_denominator: u64,

    /// numerator of the swap_fee
    pub swap_fee_numerator: u64,
    /// denominator of the swap_fee
    pub swap_fee_denominator: u64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, BorshDeserialize)]
pub struct RaydiumStateData {
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
    pub swap_coin_in_amount: u128,
    /// swap pc out amount
    pub swap_pc_out_amount: u128,
    /// charge pc as swap fee while swap pc to coin
    pub swap_acc_pc_fee: u64,

    /// swap pc in amount
    pub swap_pc_in_amount: u128,
    /// swap coin out amount
    pub swap_coin_out_amount: u128,
    /// charge coin as swap fee while swap coin to pc
    pub swap_acc_coin_fee: u64,
}

#[repr(u64)]
pub enum RaydiumStatus {
    Uninitialized = 0u64,
    Initialized = 1u64,
    Disabled = 2u64,
    WithdrawOnly = 3u64,
    // pool only can add or remove liquidity, can't swap and plan orders
    LiquidityOnly = 4u64,
    // pool only can add or remove liquidity and plan orders, can't swap
    OrderBookOnly = 5u64,
    // pool only can add or remove liquidity and swap, can't plan orders
    SwapOnly = 6u64,
    // pool status after created and will auto update to SwapOnly during swap after open_time
    WaitingTrade = 7u64,
}

impl RaydiumStatus {
    pub fn from_u64(status: u64) -> Self {
        match status {
            0u64 => RaydiumStatus::Uninitialized,
            1u64 => RaydiumStatus::Initialized,
            2u64 => RaydiumStatus::Disabled,
            3u64 => RaydiumStatus::WithdrawOnly,
            4u64 => RaydiumStatus::LiquidityOnly,
            5u64 => RaydiumStatus::OrderBookOnly,
            6u64 => RaydiumStatus::SwapOnly,
            7u64 => RaydiumStatus::WaitingTrade,
            _ => unreachable!(),
        }
    }

    pub fn into_u64(&self) -> u64 {
        match self {
            RaydiumStatus::Uninitialized => 0u64,
            RaydiumStatus::Initialized => 1u64,
            RaydiumStatus::Disabled => 2u64,
            RaydiumStatus::WithdrawOnly => 3u64,
            RaydiumStatus::LiquidityOnly => 4u64,
            RaydiumStatus::OrderBookOnly => 5u64,
            RaydiumStatus::SwapOnly => 6u64,
            RaydiumStatus::WaitingTrade => 7u64,
        }
    }
    pub fn valid_status(status: u64) -> bool {
        match status {
            1u64 | 2u64 | 3u64 | 4u64 | 5u64 | 6u64 | 7u64 => return true,
            _ => return false,
        }
    }

    pub fn deposit_permission(&self) -> bool {
        match self {
            RaydiumStatus::Uninitialized => false,
            RaydiumStatus::Initialized => true,
            RaydiumStatus::Disabled => false,
            RaydiumStatus::WithdrawOnly => false,
            RaydiumStatus::LiquidityOnly => true,
            RaydiumStatus::OrderBookOnly => true,
            RaydiumStatus::SwapOnly => true,
            RaydiumStatus::WaitingTrade => true,
        }
    }

    pub fn withdraw_permission(&self) -> bool {
        match self {
            RaydiumStatus::Uninitialized => false,
            RaydiumStatus::Initialized => true,
            RaydiumStatus::Disabled => false,
            RaydiumStatus::WithdrawOnly => true,
            RaydiumStatus::LiquidityOnly => true,
            RaydiumStatus::OrderBookOnly => true,
            RaydiumStatus::SwapOnly => true,
            RaydiumStatus::WaitingTrade => true,
        }
    }

    pub fn swap_permission(&self) -> bool {
        match self {
            RaydiumStatus::Uninitialized => false,
            RaydiumStatus::Initialized => true,
            RaydiumStatus::Disabled => false,
            RaydiumStatus::WithdrawOnly => false,
            RaydiumStatus::LiquidityOnly => false,
            RaydiumStatus::OrderBookOnly => false,
            RaydiumStatus::SwapOnly => true,
            RaydiumStatus::WaitingTrade => true,
        }
    }

    pub fn orderbook_permission(&self) -> bool {
        match self {
            RaydiumStatus::Uninitialized => false,
            RaydiumStatus::Initialized => true,
            RaydiumStatus::Disabled => false,
            RaydiumStatus::WithdrawOnly => false,
            RaydiumStatus::LiquidityOnly => false,
            RaydiumStatus::OrderBookOnly => true,
            RaydiumStatus::SwapOnly => false,
            RaydiumStatus::WaitingTrade => false,
        }
    }
}

use crate::raydium::serum_types::MarketState;
use crate::raydium::types::{AmmInfo, AmmKeys, MarketKeys};
use num_derive::FromPrimitive;
use solana_program::{decode_error::DecodeError, msg, program_error::PrintProgramError};
use thiserror::Error;

/// Errors that may be returned by the TokenAmm program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum AmmError {
    // 0
    /// The account cannot be initialized because it is already being used.
    #[error("AlreadyInUse")]
    AlreadyInUse,
    /// The program address provided doesn't match value generated by the program.
    #[error("InvalidProgramAddress")]
    InvalidProgramAddress,
    /// The deserialization of the Token state returned something besides State::Token.
    #[error("ExpectedMint")]
    ExpectedMint,
    /// The deserialization of the Token state returned something besides State::Account.
    #[error("ExpectedAccount")]
    ExpectedAccount,
    /// The coin vault provided doesn't match the coin vault in the AmmInfo.
    #[error("InvalidCoinVault")]
    InvalidCoinVault,

    // 5
    /// The pc vault provided doesn't match the pc vault in the AmmInfo.
    #[error("InvalidPCVault")]
    InvalidPCVault,
    /// The token_lp provided doesn't match the token_lp in the AmmInfo.
    #[error("InvalidTokenLP")]
    InvalidTokenLP,
    /// The dest_token_coin provided doesn't match the dest_token_coin in WithdrawTokenInfo.
    #[error("InvalidDestTokenCoin")]
    InvalidDestTokenCoin,
    /// The dest_token_pc provided doesn't match the dest_token_pc in WithdrawTokenInfo.
    #[error("InvalidDestTokenPC")]
    InvalidDestTokenPC,
    /// The pool_mint provided doesn't match the pool_mint in the AmmInfo.
    #[error("InvalidPoolMint")]
    InvalidPoolMint,

    // 10
    /// The open_orders provided doesn't match the open_orders in in the AmmInfo.
    #[error("InvalidOpenOrders")]
    InvalidOpenOrders,
    /// The market provided doesn't match the market in the AmmInfo.
    #[error("InvalidMarket")]
    InvalidMarket,
    /// The market program provided doesn't match the market program in the AmmInfo.
    #[error("InvalidMarketProgram")]
    InvalidMarketProgram,
    /// The target_orders provided doesn't match the target_orders in the AmmInfo.
    #[error("InvalidTargetOrders")]
    InvalidTargetOrders,
    /// The Account provided must be writeable.
    #[error("AccountNeedWriteable")]
    AccountNeedWriteable,

    // 15
    /// The Account provided must be readonly.
    #[error("AccountNeedReadOnly")]
    AccountNeedReadOnly,
    /// The token_coin's mint provided doesn't match the coin_mint's key.
    #[error("InvalidCoinMint")]
    InvalidCoinMint,
    /// The token_pc's mint provided doesn't match the pc_mint's key.
    #[error("InvalidPCMint")]
    InvalidPCMint,
    /// The owner of the input isn't set to the program address generated by the program.
    #[error("InvalidOwner")]
    InvalidOwner,
    /// The initialized pool had a non zero supply.
    #[error("InvalidSupply")]
    InvalidSupply,

    // 20
    /// The initialized token has a delegate.
    #[error("InvalidDelegate")]
    InvalidDelegate,
    /// Invalid Sign Account
    #[error("Invalid Sign Account")]
    InvalidSignAccount,
    /// The amm status is invalid.
    #[error("InvalidStatus")]
    InvalidStatus,
    /// Invalid instruction number passed in
    #[error("Invalid instruction")]
    InvalidInstruction,
    /// The number of account provided does not match the expectations
    #[error("Wrong accounts number")]
    WrongAccountsNumber,

    // 25
    /// The target account owner is not match with this program
    #[error("The target account owner is not match with this program")]
    InvalidTargetAccountOwner,
    /// The owner saved in target is not match with this amm pool
    #[error("The owner saved in target is not match with this amm pool")]
    InvalidTargetOwner,
    /// The amm account owner is not match with this program"
    #[error("The amm account owner is not match with this program")]
    InvalidAmmAccountOwner,
    /// The params set is invalid
    #[error("Params Set is invalid")]
    InvalidParamsSet,
    /// The params input is invalid.
    #[error("InvalidInput")]
    InvalidInput,

    // 30
    /// instruction exceeds desired slippage limit
    #[error("instruction exceeds desired slippage limit")]
    ExceededSlippage,
    /// The calculation exchange rate failed.
    #[error("CalculationExRateFailure")]
    CalculationExRateFailure,
    /// Checked_Sub Overflow
    #[error("Checked_Sub Overflow")]
    CheckedSubOverflow,
    /// Checked_Add Overflow
    #[error("Checked_Add Overflow")]
    CheckedAddOverflow,
    /// Checked_Mul Overflow
    #[error("Checked_Mul Overflow")]
    CheckedMulOverflow,

    // 35
    /// Checked_Div Overflow
    #[error("Checked_Div Overflow")]
    CheckedDivOverflow,
    /// Empty Funds
    #[error("Empty Funds")]
    CheckedEmptyFunds,
    /// Calc pnl error
    #[error("Calc pnl error")]
    CalcPnlError,
    /// InvalidSplTokenProgram
    #[error("InvalidSplTokenProgram")]
    InvalidSplTokenProgram,
    /// TakePnlError
    #[error("Take Pnl error")]
    TakePnlError,

    // 40
    /// Insufficient funds
    #[error("Insufficient funds")]
    InsufficientFunds,
    /// ConversionFailure
    #[error("Conversion to u64 failed with an overflow or underflow")]
    ConversionFailure,
    /// The user token input does not match amm
    #[error("user token input does not match amm")]
    InvalidUserToken,
    // The srm_token's mint provided doesn't match the pc_mint's key.
    #[error("InvalidSrmMint")]
    InvalidSrmMint,
    /// The srm_token provided doesn't match the srm_token in the program.
    #[error("InvalidSrmToken")]
    InvalidSrmToken,

    // 45
    /// TooManyOpenOrders
    #[error("TooManyOpenOrders")]
    TooManyOpenOrders,
    /// OrderAtSlotIsPlaced
    #[error("OrderAtSlotIsPlaced")]
    OrderAtSlotIsPlaced,
    /// InvalidSysProgramAddress
    #[error("InvalidSysProgramAddress")]
    InvalidSysProgramAddress,
    /// The provided fee does not match the program owner's constraints
    #[error("The provided fee does not match the program owner's constraints")]
    InvalidFee,
    /// Repeat create amm about market
    #[error("Repeat create amm about market")]
    RepeatCreateAmm,

    // 50
    /// Not allow Zero LP
    #[error("Not allow Zero LP")]
    NotAllowZeroLP,
    /// The provided token account has a close authority.
    #[error("Token account has a close authority")]
    InvalidCloseAuthority,
    /// The pool token mint has a freeze authority.
    #[error("Pool token mint has a freeze authority")]
    InvalidFreezeAuthority,
    // The referrer_pc_wallet's mint provided doesn't match the pc_mint's key.
    #[error("InvalidReferPCMint")]
    InvalidReferPCMint,
    /// InvalidConfigAccount
    #[error("InvalidConfigAccount")]
    InvalidConfigAccount,

    // 55
    /// RepeatCreateConfigAccount
    #[error("Repeat create config account")]
    RepeatCreateConfigAccount,
    /// MarketLotSizeIsTooLarge
    #[error("Market lotSize is too large")]
    MarketLotSizeIsTooLarge,
    /// Init lp amount is too less.
    #[error("Init lp amount is too less(Because 10**lp_decimals amount lp will be locked)")]
    InitLpAmountTooLess,
    /// Unknown Amm Error
    #[error("Unknown Amm Error")]
    UnknownAmmError,
}

impl From<AmmError> for ProgramError {
    fn from(e: AmmError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for AmmError {
    fn type_of() -> &'static str {
        "Amm Error"
    }
}

impl PrintProgramError for AmmError {
    fn print<E>(&self)
    where
        E: 'static
            + std::error::Error
            + DecodeError<E>
            + PrintProgramError
            + num_traits::FromPrimitive,
    {
        match self {
            AmmError::AlreadyInUse => msg!("Error: AlreadyInUse"),
            AmmError::InvalidProgramAddress => msg!("Error: InvalidProgramAddress"),
            AmmError::ExpectedMint => msg!("Error: ExpectedMint"),
            AmmError::ExpectedAccount => msg!("Error: ExpectedAccount"),
            AmmError::InvalidCoinVault => msg!("Error: InvalidCoinVault"),

            AmmError::InvalidPCVault => msg!("Error: InvalidPCVault"),
            AmmError::InvalidTokenLP => msg!("Error: InvalidTokenLP"),
            AmmError::InvalidDestTokenCoin => msg!("Error: InvalidDestTokenCoin"),
            AmmError::InvalidDestTokenPC => msg!("Error: InvalidDestTokenPC"),
            AmmError::InvalidPoolMint => msg!("Error: InvalidPoolMint"),
            AmmError::InvalidOpenOrders => msg!("Error: InvalidOpenOrders"),
            AmmError::InvalidMarket => msg!("Error: InvalidMarket"),
            AmmError::InvalidMarketProgram => msg!("Error: InvalidMarketProgram"),

            AmmError::InvalidTargetOrders => msg!("Error: InvalidTargetOrders"),
            AmmError::AccountNeedWriteable => msg!("Error: AccountNeedWriteable"),
            AmmError::AccountNeedReadOnly => msg!("Error: AccountNeedReadOnly"),
            AmmError::InvalidCoinMint => msg!("Error: InvalidCoinMint"),
            AmmError::InvalidPCMint => msg!("Error: InvalidPCMint"),

            AmmError::InvalidOwner => msg!("Error: InvalidOwner"),
            AmmError::InvalidSupply => msg!("Error: InvalidSupply"),
            AmmError::InvalidDelegate => msg!("Error: InvalidDelegate"),
            AmmError::InvalidSignAccount => msg!("Error: Invalid Sign Account"),
            AmmError::InvalidStatus => msg!("Error: InvalidStatus"),

            AmmError::InvalidInstruction => msg!("Error: InvalidInstruction"),
            AmmError::WrongAccountsNumber => msg!("Error: WrongAccountsNumber"),
            AmmError::InvalidTargetAccountOwner => {
                msg!("Error: The target account owner is not match with this program")
            }
            AmmError::InvalidTargetOwner => {
                msg!("Error: The owner saved in target is not match with this amm pool")
            }
            AmmError::InvalidAmmAccountOwner => {
                msg!("Error: The amm account owner is not match with this program")
            }

            AmmError::InvalidParamsSet => msg!("Error: Params Set is Invalid"),
            AmmError::InvalidInput => msg!("Error: InvalidInput"),
            AmmError::ExceededSlippage => msg!("Error: exceeds desired slippage limit"),
            AmmError::CalculationExRateFailure => msg!("Error: CalculationExRateFailure"),
            AmmError::CheckedSubOverflow => msg!("Error: Checked_Sub Overflow"),

            AmmError::CheckedAddOverflow => msg!("Error: Checked_Add Overflow"),
            AmmError::CheckedMulOverflow => msg!("Error: Checked_Mul Overflow"),
            AmmError::CheckedDivOverflow => msg!("Error: Checked_Div Overflow"),
            AmmError::CheckedEmptyFunds => msg!("Error: CheckedEmptyFunds"),
            AmmError::CalcPnlError => msg!("Error: CalcPnlError"),

            AmmError::InvalidSplTokenProgram => msg!("Error: InvalidSplTokenProgram"),
            AmmError::TakePnlError => msg!("Error: TakePnlError"),
            AmmError::InsufficientFunds => msg!("Error: insufficient funds"),
            AmmError::ConversionFailure => msg!("Error: Conversion to or from u64 failed."),
            AmmError::InvalidUserToken => msg!("Error: User token input does not match amm"),

            AmmError::InvalidSrmMint => msg!("Error: InvalidSrmMint"),
            AmmError::InvalidSrmToken => msg!("Error: InvalidSrmToken"),
            AmmError::TooManyOpenOrders => msg!("Error: TooManyOpenOrders"),
            AmmError::OrderAtSlotIsPlaced => msg!("Error: OrderAtSlotIsPlaced"),
            AmmError::InvalidSysProgramAddress => msg!("Error: InvalidSysProgramAddress"),

            AmmError::InvalidFee => msg!("Error: InvalidFee"),
            AmmError::RepeatCreateAmm => msg!("Error: RepeatCreateAmm"),
            AmmError::NotAllowZeroLP => msg!("Error: NotAllowZeroLP"),
            AmmError::InvalidCloseAuthority => msg!("Error: Token account has a close authority"),
            AmmError::InvalidFreezeAuthority => {
                msg!("Error: Pool token mint has a freeze authority")
            }
            AmmError::InvalidReferPCMint => msg!("Error: InvalidReferPCMint"),
            AmmError::InvalidConfigAccount => msg!("Error: InvalidConfigAccount"),
            AmmError::RepeatCreateConfigAccount => msg!("Error: RepeatCreateConfigAccount"),
            AmmError::MarketLotSizeIsTooLarge => msg!("Error: Market lotSize is too large"),
            AmmError::InitLpAmountTooLess => msg!(
                "Error: Init lp amount is too less(Because 10**lp_decimals amount lp will be locked)"
            ),
            AmmError::UnknownAmmError => msg!("Error: UnknownAmmError"),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy)]
pub struct LiquidityStateV4 {
    pub status: u64,
    pub nonce: u64,
    pub max_order: u64,
    pub depth: u64,
    pub base_decimal: u64,
    pub quote_decimal: u64,
    pub state: u64,
    pub reset_flag: u64,
    pub min_size: u64,
    pub vol_max_cut_ratio: u64,
    pub amount_wave_ratio: u64,
    pub base_lot_size: u64,
    pub quote_lot_size: u64,
    pub min_price_multiplier: u64,
    pub max_price_multiplier: u64,
    pub system_decimal_value: u64,
    pub min_separate_numerator: u64,
    pub min_separate_denominator: u64,
    pub trade_fee_numerator: u64,
    pub trade_fee_denominator: u64,
    pub pnl_numerator: u64,
    pub pnl_denominator: u64,
    pub swap_fee_numerator: u64,
    pub swap_fee_denominator: u64,
    pub base_need_take_pnl: u64,
    pub quote_need_take_pnl: u64,
    pub quote_total_pnl: u64,
    pub base_total_pnl: u64,
    pub pool_open_time: u64,
    pub punish_pc_amount: u64,
    pub punish_coin_amount: u64,
    pub orderbook_to_init_time: u64,
    // if you ever need these, uncomment:
    // pub pool_total_deposit_pc: u128,
    // pub pool_total_deposit_coin: u128,
    pub swap_base_in_amount: u128,
    pub swap_quote_out_amount: u128,
    pub swap_base2quote_fee: u64,
    pub swap_quote_in_amount: u128,
    pub swap_base_out_amount: u128,
    pub swap_quote2base_fee: u64,
    // AMM vaults
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    // mints
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    // market accounts
    pub open_orders: Pubkey,
    pub market_id: Pubkey,
    pub market_program_id: Pubkey,
    pub target_orders: Pubkey,
    pub withdraw_queue: Pubkey,
    pub lp_vault: Pubkey,
    pub owner: Pubkey,
    // true circulating supply without lock up
    pub lp_reserve: u64,
    // padding to keep the size aligned
    pub padding: [u64; 3],
}
impl From<LiquidityStateV4> for RaydiumAmmInfo {
    fn from(value: LiquidityStateV4) -> Self {
        RaydiumAmmInfo {
            status: value.status,
            nonce: value.nonce,
            order_num: value.max_order,
            depth: value.depth,
            coin_decimals: value.quote_decimal,
            pc_decimals: value.base_decimal,
            state: value.state,
            reset_flag: value.reset_flag,
            min_size: value.min_size,
            vol_max_cut_ratio: value.vol_max_cut_ratio,
            amount_wave: value.amount_wave_ratio,
            coin_lot_size: value.quote_lot_size,
            pc_lot_size: value.base_lot_size,
            min_price_multiplier: value.min_price_multiplier,
            max_price_multiplier: value.max_price_multiplier,
            sys_decimal_value: value.system_decimal_value,
            fees: RaydiumFees {
                min_separate_numerator: 0,
                min_separate_denominator: 0,
                trade_fee_numerator: 0,
                trade_fee_denominator: 0,
                pnl_numerator: 0,
                pnl_denominator: 0,
                swap_fee_numerator: 0,
                swap_fee_denominator: value.swap_fee_denominator,
            },
            state_data: RaydiumStateData {
                need_take_pnl_coin: 0,
                need_take_pnl_pc: 0,
                total_pnl_pc: 0,
                total_pnl_coin: 0,
                pool_open_time: 0,
                padding: [0, 0],
                orderbook_to_init_time: 0,
                swap_coin_in_amount: 0,
                swap_pc_out_amount: 0,
                swap_acc_pc_fee: 0,
                swap_pc_in_amount: 0,
                swap_coin_out_amount: 0,
                swap_acc_coin_fee: 0,
            },
            coin_vault: value.quote_vault,
            pc_vault: value.base_vault,
            coin_vault_mint: value.quote_mint,
            pc_vault_mint: value.base_mint,
            lp_mint: value.lp_mint,
            open_orders: value.open_orders,
            market: value.market_id,
            market_program: value.market_program_id,
            target_orders: value.target_orders,
            padding1: [0; 8],
            amm_owner: value.owner,
            lp_amount: value.lp_reserve,
            client_order_id: value.max_order,
            padding2: [0; 2],
        }
    }
}

impl_loadable!(LiquidityStateV4);
#[derive(Debug)]
pub struct RaydiumAmmQuote {
    /// The address of the amm pool
    pub market: Pubkey,
    /// The input mint
    pub input_mint: Pubkey,
    /// The output mint,
    pub output_mint: Pubkey,
    /// The amount specified
    pub amount: u64,
    /// The other amount
    pub other_amount: u64,
    /// The other amount with slippage
    pub other_amount_threshold: u64,
    /// Whether the amount specified is in terms of the input token
    pub amount_specified_is_input: bool,
    /// The input mint decimals
    pub input_mint_decimals: u8,
    /// The output mint decimals
    pub output_mint_decimals: u8,
    /// Amm keys
    pub amm_keys: AmmKeys,
    /// Market keys
    pub market_keys: MarketKeys,
}
