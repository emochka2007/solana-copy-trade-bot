pub(crate) mod types;
use crate::config::WSOL;
use crate::raydium::amm::RaydiumAmm;
use crate::raydium::api_v3::ApiV3Client;
use crate::raydium::types::{RaydiumAmmExecutorOpts, SwapExecutionMode, SwapInput};
use crate::trade_info::TradeInfoFromToken;
use log::info;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::VersionedTransaction;
use std::env;
use std::str::FromStr;
use std::sync::Arc;

pub mod amm;
mod amm_math;
mod amm_types;
pub mod api_v3;
mod math;
mod serum;
mod serum_error;
mod serum_types;
mod utils;

pub async fn swap_in(trade_info_from_token: TradeInfoFromToken) {
    let rpc_link = env::var("RPC_SOLANA").unwrap();
    let client = Arc::new(RpcClient::new(rpc_link));
    let executor = RaydiumAmm::new(
        Arc::clone(&client),
        RaydiumAmmExecutorOpts::default(),
        ApiV3Client::new(None),
    );
    let base_token = Pubkey::from_str_const(WSOL);
    let swap_input = SwapInput {
        input_token_mint: base_token,
        output_token_mint: Pubkey::from_str(&trade_info_from_token.mint).unwrap(),
        slippage_bps: 1000, // 10%
        amount: 1_000_000,  // 0.001 SOL
        mode: SwapExecutionMode::ExactIn,
        market: None,
    };

    let quote = executor.quote(&swap_input).await;
    // log::info!("Quote: {:#?}", quote);
    //
    // let keypair = Keypair::new();
    // let mut transaction = executor
    //     .swap_transaction(keypair.pubkey(), quote, None)
    //     .await?;
    // let blockhash = client.get_latest_blockhash()?;
    // transaction.message.set_recent_blockhash(blockhash);
    // let _final_tx = VersionedTransaction::try_new(transaction.message, &[&keypair])?;
    // info!("{:?}", _final_tx);
}
