use crate::config::{RAYDIUM_AUTHORITY_V4, RAYDIUM_LIQUIDITY_POOL_V4_PROGRAM_ID};
use crate::raydium::amm_types::{LiquidityStateV4, RaydiumAmmInfo, RaydiumStatus};
use crate::raydium::api_v3::response::{ApiV3PoolsPage, ApiV3StandardPool, ApiV3StandardPoolKeys};
use crate::raydium::api_v3::{ApiV3Client, PoolFetchParams, PoolSort, PoolSortOrder, PoolType};
use crate::raydium::serum::load_serum_market_order;
use crate::raydium::types::{
    AmmInfo, AmmKeys, MarketKeys, RaydiumAmmExecutorOpts, SwapConfig, SwapInput,
};
use anyhow::{Context, anyhow};
use arrayref::array_ref;
use borsh::BorshDeserialize;
use log::info;
use safe_transmute::{transmute_one_pedantic, transmute_to_bytes};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::account_info::{AccountInfo, IntoAccountInfo};
use spl_token::solana_program;
use spl_token::solana_program::program_pack::Pack;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone)]
pub struct RaydiumAmm {
    client: Arc<RpcClient>,
    api: ApiV3Client,
    config: SwapConfig,
    load_keys_by_api: bool,
}
impl RaydiumAmm {
    pub fn new(client: Arc<RpcClient>, config: RaydiumAmmExecutorOpts, api: ApiV3Client) -> Self {
        let RaydiumAmmExecutorOpts {
            priority_fee,
            cu_limits,
            wrap_and_unwrap_sol,
            load_keys_by_api,
        } = config;
        Self {
            client,
            api,
            load_keys_by_api: load_keys_by_api.unwrap_or(true),
            config: SwapConfig {
                priority_fee,
                cu_limits,
                wrap_and_unwrap_sol,
                as_legacy_transaction: Some(true),
            },
        }
    }

    pub async fn quote(&self, swap_input: &SwapInput) -> anyhow::Result<()> {
        if swap_input.input_token_mint == swap_input.output_token_mint {
            return Err(anyhow!(
                "Input token cannot equal output token {}",
                swap_input.input_token_mint
            ));
        }

        let mut pool_id = swap_input.market;
        if pool_id.is_none() {
            let response: ApiV3PoolsPage<ApiV3StandardPool> = self
                .api
                .fetch_pool_by_mints(
                    &swap_input.input_token_mint,
                    Some(&swap_input.output_token_mint),
                    &PoolFetchParams {
                        pool_type: PoolType::Standard,
                        pool_sort: PoolSort::Liquidity,
                        sort_type: PoolSortOrder::Descending,
                        page_size: 10,
                        page: 1,
                    },
                )
                .await?;
            pool_id = response.pools.into_iter().find_map(|pool| {
                if pool.mint_a.address == swap_input.input_token_mint
                    && pool.mint_b.address == swap_input.output_token_mint
                    || pool.mint_a.address == swap_input.output_token_mint
                        && pool.mint_b.address == swap_input.input_token_mint
                        && pool.program_id
                            == Pubkey::from_str_const(RAYDIUM_LIQUIDITY_POOL_V4_PROGRAM_ID)
                {
                    Some(pool.id)
                } else {
                    None
                }
            });
        }

        let Some(pool_id) = pool_id else {
            return Err(anyhow!("Failed to get market for swap"));
        };

        let response = self
            .api
            .fetch_pool_keys_by_ids::<ApiV3StandardPoolKeys>(
                [&pool_id].into_iter().map(|id| id.to_string()).collect(),
            )
            .await?;
        let keys = response.first().context(format!(
            "Failed to get pool keys for raydium standard pool {}",
            pool_id
        ))?;

        let (amm_keys, market_keys) = (AmmKeys::try_from(keys)?, MarketKeys::try_from(keys)?);
        info!("{:?}, {:?}", amm_keys, market_keys);

        // reload accounts data to calculate amm pool vault amount
        // get multiple accounts at the same time to ensure data consistency
        let load_pubkeys = vec![
            pool_id,
            amm_keys.amm_target,
            amm_keys.amm_pc_vault,
            amm_keys.amm_coin_vault,
            amm_keys.amm_open_order,
            amm_keys.market,
            market_keys.event_queue,
        ];
        let rsps =
            crate::raydium::utils::get_multiple_account_data(&self.client, &load_pubkeys).await?;
        info!("{:?}", rsps);
        let accounts = array_ref![rsps, 0, 7];
        let [
            amm_account,
            amm_target_account,
            amm_pc_vault_account,
            amm_coin_vault_account,
            amm_open_orders_account,
            market_account,
            market_event_q_account,
        ] = accounts;
        let data = &amm_account.as_ref().unwrap().data;
        info!("Account data length: {}", data.len());

        info!(
            "Expected AmmInfo size: {}",
            std::mem::size_of::<LiquidityStateV4>()
        );

        let amm: RaydiumAmmInfo =
            transmute_one_pedantic::<LiquidityStateV4>(transmute_to_bytes(data))
                .map_err(|e| e.without_src())
                .unwrap()
                .into();
        info!("AMM {:?}", amm);

        let _amm_target: crate::raydium::amm_types::RaydiumTargetOrders =
            transmute_one_pedantic::<crate::raydium::amm_types::RaydiumTargetOrders>(
                transmute_to_bytes(&amm_target_account.as_ref().unwrap().clone().data),
            )
            .map_err(|e| e.without_src())?;
        let amm_pc_vault =
            spl_token::state::Account::unpack(&amm_pc_vault_account.as_ref().unwrap().clone().data)
                .unwrap();
        let amm_coin_vault = spl_token::state::Account::unpack(
            &amm_coin_vault_account.as_ref().unwrap().clone().data,
        )
        .unwrap();
        let (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount) =
            crate::raydium::math::Calculator::calc_total_without_take_pnl_no_orderbook(
                amm_pc_vault.amount,
                amm_coin_vault.amount,
                &amm,
            )
            .unwrap();
        let (a, b) = (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount);
        info!("{}", a);

        let (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount) =
            if RaydiumStatus::from_u64(amm.status).orderbook_permission() {
                let amm_open_orders_account =
                    &mut amm_open_orders_account.as_ref().unwrap().clone();
                let market_account = &mut market_account.as_ref().unwrap().clone();
                let market_event_q_account = &mut market_event_q_account.as_ref().unwrap().clone();
                let amm_open_orders_info =
                    (&amm.open_orders, amm_open_orders_account).into_account_info();
                let market_account_info = (&amm.market, market_account).into_account_info();
                let market_event_queue_info =
                    (&(market_keys.event_queue), market_event_q_account).into_account_info();
                let liquidity_pool_pub_key =
                    Pubkey::from_str(RAYDIUM_LIQUIDITY_POOL_V4_PROGRAM_ID).unwrap();
                let amm_authority = Pubkey::find_program_address(
                    &[RAYDIUM_AUTHORITY_V4.as_ref()],
                    &liquidity_pool_pub_key,
                )
                .0;
                let lamports = &mut 0;
                let data = &mut [0u8];
                let owner = Pubkey::default();
                let amm_authority_info = AccountInfo::new(
                    &amm_authority,
                    false,
                    false,
                    lamports,
                    data,
                    &owner,
                    false,
                    0,
                );
                let (market_state, open_orders) = load_serum_market_order(
                    &market_account_info,
                    &amm_open_orders_info,
                    &amm_authority_info,
                    &amm,
                    false,
                )?;
                let (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount) =
                    crate::raydium::math::Calculator::calc_total_without_take_pnl(
                        amm_pc_vault.amount,
                        amm_coin_vault.amount,
                        &open_orders,
                        &amm,
                        &market_state,
                        &market_event_queue_info,
                        &amm_open_orders_info,
                    )?;
                (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount)
            } else {
                let (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount) =
                    crate::raydium::math::Calculator::calc_total_without_take_pnl_no_orderbook(
                        amm_pc_vault.amount,
                        amm_coin_vault.amount,
                        &amm,
                    )?;
                (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount)
            };

        let (direction, coin_to_pc) = if swap_input.input_token_mint == amm_keys.amm_coin_mint
            && swap_input.output_token_mint == amm_keys.amm_pc_mint
        {
            (crate::raydium::utils::SwapDirection::Coin2PC, true)
        } else {
            (crate::raydium::utils::SwapDirection::PC2Coin, false)
        };

        info!("Direction {:?}", direction);

        // let amount_specified_is_input = swap_input.mode.amount_specified_is_input();
        // let (other_amount, other_amount_threshold) = raydium_library::amm::swap_with_slippage(
        //     amm_pool_pc_vault_amount,
        //     amm_pool_coin_vault_amount,
        //     amm.fees.swap_fee_numerator,
        //     amm.fees.swap_fee_denominator,
        //     direction,
        //     swap_input.amount,
        //     amount_specified_is_input,
        //     swap_input.slippage_bps as u64,
        // )?;
        // log::debug!(
        //     "raw quote: {}. raw other_amount_threshold: {}",
        //     other_amount,
        //     other_amount_threshold
        // );
        //
        // Ok(RaydiumAmmQuote {
        //     market: pool_id,
        //     input_mint: swap_input.input_token_mint,
        //     output_mint: swap_input.output_token_mint,
        //     amount: swap_input.amount,
        //     other_amount,
        //     other_amount_threshold,
        //     amount_specified_is_input,
        //     input_mint_decimals: if coin_to_pc {
        //         amm.coin_decimals
        //     } else {
        //         amm.pc_decimals
        //     } as u8,
        //     output_mint_decimals: if coin_to_pc {
        //         amm.pc_decimals
        //     } else {
        //         amm.coin_decimals
        //     } as u8,
        //     amm_keys,
        //     market_keys,
        // })
        Ok(())
    }
}
