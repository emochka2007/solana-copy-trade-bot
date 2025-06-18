use crate::keypair::from_bytes_to_key_pair;
use crate::trade_info::TradeInfoFromToken;
use borsh::BorshDeserialize;
use log::info;
use reqwest::Client;
use reqwest::blocking::get;
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use std::str::FromStr;

// Constants (assumed to be defined in config.rs)
const RAYDIUM_AMM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const RAYDIUM_AUTHORITY_V4: &str = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1";
const WSOL: &str = "So11111111111111111111111111111111111111112";
const SERUM_PROGRAM_ID: &str = "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX";

// Assume these are defined in your project
struct Config {
    rpc_link: String,
    private_key: String,
}

impl Config {
    fn new() -> anyhow::Result<Self> {
        // Implementation to load config
        Ok(Config {
            rpc_link: String::from("https://api.devnet.solana.com"),
            private_key: String::from("YOUR_PRIVATE_KEY"),
        })
    }
}

// Pool state struct for deserialization
#[derive(BorshDeserialize, Debug)]
struct AmmInfo {
    token_0_mint: Pubkey,
    token_1_mint: Pubkey,
    token_0_vault: Pubkey,
    token_1_vault: Pubkey,
    open_orders: Pubkey,
    market: Pubkey,
}

// API response struct
#[derive(Deserialize)]
struct PoolInfo {
    id: String,
    baseMint: String,
    quoteMint: String,
    baseVault: String,
    quoteVault: String,
    openOrders: String,
    marketId: String,
    marketProgramId: String,
    marketBids: String,
    marketAsks: String,
    marketEventQueue: String,
    marketBaseVault: String,
    marketQuoteVault: String,
    marketVaultSigner: String,
}

// Pool keys struct
#[derive(Clone, Debug)]
struct PoolKeys {
    id: Pubkey,
    authority: Pubkey,
    open_orders: Pubkey,
    base_vault: Pubkey,
    quote_vault: Pubkey,
    market_id: Pubkey,
    market_program_id: Pubkey,
    market_bids: Pubkey,
    market_asks: Pubkey,
    market_event_queue: Pubkey,
    market_base_vault: Pubkey,
    market_quote_vault: Pubkey,
    market_vault_signer: Pubkey,
}

// Fetch pool keys from API or on-chain
// curl -X 'GET' \
// 'https://api-v3.raydium.io/pools/info/mint?mint1=So11111111111111111111111111111111111111112&mint2=43PkURedPPaTv6NRXC17fvR5E4Zcn3URsEpZ4bzSMjW4&poolType=all&poolSortField=default&sortType=desc&pageSize=1000&page=1' \
// -H 'accept: application/json'
async fn get_pool_keys(client: &RpcClient, pool_id: &Pubkey) -> anyhow::Result<PoolKeys> {
    // Use async reqwest client
    let reqwest_client = Client::new();
    let url = "https://api.raydium.io/v2/ammV3/ammPools";
    let response = reqwest_client
        .get(url)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    info!("{:?}", response);
    let response = reqwest_client
        .get(url)
        .send()
        .await
        .unwrap()
        .json::<Vec<PoolInfo>>()
        .await
        .unwrap();
    let pool = response
        .into_iter()
        .find(|p| p.id == pool_id.to_string())
        .ok_or_else(|| anyhow::anyhow!("Pool not found"))?;

    let pool_keys = PoolKeys {
        id: Pubkey::from_str(&pool.id)?,
        authority: Pubkey::from_str(RAYDIUM_AUTHORITY_V4)?,
        open_orders: Pubkey::from_str(&pool.openOrders)?,
        base_vault: Pubkey::from_str(&pool.baseVault)?,
        quote_vault: Pubkey::from_str(&pool.quoteVault)?,
        market_id: Pubkey::from_str(&pool.marketId)?,
        market_program_id: Pubkey::from_str(&pool.marketProgramId)?,
        market_bids: Pubkey::from_str(&pool.marketBids)?,
        market_asks: Pubkey::from_str(&pool.marketAsks)?,
        market_event_queue: Pubkey::from_str(&pool.marketEventQueue)?,
        market_base_vault: Pubkey::from_str(&pool.marketBaseVault)?,
        market_quote_vault: Pubkey::from_str(&pool.marketQuoteVault)?,
        market_vault_signer: Pubkey::from_str(&pool.marketVaultSigner)?,
    };

    // Option 2: Fetch on-chain (alternative, uncomment to use)
    /*
    let account = client.get_account(pool_id)?;
    let pool_data = account.data;
    let amm_info = AmmInfo::try_from_slice(&pool_data[8..])?; // Skip discriminator
    let pool_keys = PoolKeys {
        id: *pool_id,
        authority: Pubkey::from_str(RAYDIUM_AUTHORITY_V4)?,
        open_orders: amm_info.open_orders,
        base_vault: amm_info.token_0_vault,
        quote_vault: amm_info.token_1_vault,
        market_id: amm_info.market,
        market_program_id: Pubkey::from_str(SERUM_PROGRAM_ID)?,
        // Market accounts would need to be fetched from market state
        market_bids: Pubkey::default(),
        market_asks: Pubkey::default(),
        market_event_queue: Pubkey::default(),
        market_base_vault: Pubkey::default(),
        market_quote_vault: Pubkey::default(),
        market_vault_signer: Pubkey::default(),
    };
    */

    Ok(pool_keys)
}

pub struct Engine {}

impl Engine {
    pub async fn buy_token(trade_info: TradeInfoFromToken) -> anyhow::Result<()> {
        let token_amount = trade_info.token_amount_list.token_post_amount
            - trade_info.token_amount_list.token_pre_amount;
        let sol_amount =
            trade_info.sol_amount_list.sol_post_amount - trade_info.sol_amount_list.sol_pre_amount;
        let Config { rpc_link, .. } = Config::new()?;
        let client = RpcClient::new(rpc_link);
        let payer = from_bytes_to_key_pair();
        let token_mint = Pubkey::from_str(&trade_info.mint)?;
        let pool_id = Pubkey::from_str(&trade_info.pool)?;
        let raydium_program_id = Pubkey::from_str(RAYDIUM_AMM_V4)?;
        let wsol_mint = Pubkey::from_str(WSOL)?;

        info!("Token amount: {}, SOL amount: {}", token_amount, sol_amount);

        // Fetch pool keys
        let pool_keys = get_pool_keys(&client, &pool_id).await?;

        info!("pool keys {:?}", pool_keys);
        // Determine base and quote vaults based on pool order
        let (base_vault, quote_vault, base_mint, quote_mint) = {
            let account = client.get_account(&pool_id)?;
            let pool_data = account.data;
            let amm_info = AmmInfo::try_from_slice(&pool_data[8..])?;
            if amm_info.token_0_mint == wsol_mint {
                (
                    pool_keys.base_vault,
                    pool_keys.quote_vault,
                    amm_info.token_0_mint,
                    amm_info.token_1_mint,
                )
            } else {
                (
                    pool_keys.quote_vault,
                    pool_keys.base_vault,
                    amm_info.token_1_mint,
                    amm_info.token_0_mint,
                )
            }
        };

        // Get user token accounts
        let user_source = get_associated_token_address(&payer.pubkey(), &wsol_mint); // SOL
        let user_destination = get_associated_token_address(&payer.pubkey(), &token_mint); // Token

        // Create associated token account if needed
        let create_ata_instruction = if client.get_account(&user_destination).is_err() {
            Some(
                spl_associated_token_account::instruction::create_associated_token_account(
                    &payer.pubkey(),
                    &payer.pubkey(),
                    &token_mint,
                    &spl_token::id(),
                ),
            )
        } else {
            None
        };

        // Swap parameters
        let amount_in: u64 = 1_000_000; // 0.01 SOL in lamports
        let minimum_amount_out: u64 = 0; // Adjust for slippage

        // Construct swap instruction accounts
        let accounts = vec![
            AccountMeta::new_readonly(spl_token::id(), false), // Token program
            AccountMeta::new(pool_id, false),                  // AMM pool
            AccountMeta::new_readonly(pool_keys.authority, false), // AMM authority
            AccountMeta::new(pool_keys.open_orders, false),    // AMM open orders
            AccountMeta::new(base_vault, false),               // AMM base vault (SOL)
            AccountMeta::new(quote_vault, false),              // AMM quote vault (Token)
            AccountMeta::new_readonly(pool_keys.market_program_id, false), // Serum program
            AccountMeta::new(pool_keys.market_id, false),      // Serum market
            AccountMeta::new(pool_keys.market_bids, false),    // Serum bids
            AccountMeta::new(pool_keys.market_asks, false),    // Serum asks
            AccountMeta::new(pool_keys.market_event_queue, false), // Serum event queue
            AccountMeta::new(pool_keys.market_base_vault, false), // Serum base vault
            AccountMeta::new(pool_keys.market_quote_vault, false), // Serum quote vault
            AccountMeta::new_readonly(pool_keys.market_vault_signer, false), // Serum vault signer
            AccountMeta::new(user_source, false),              // User source (SOL)
            AccountMeta::new(user_destination, false),         // User destination (Token)
            AccountMeta::new(payer.pubkey(), true),            // User owner
        ];

        // Swap instruction data
        let data = vec![
            vec![9_u8], // SwapBaseIn instruction index (verify with Raydium program)
            amount_in.to_le_bytes().to_vec(),
            minimum_amount_out.to_le_bytes().to_vec(),
        ]
        .concat();

        let swap_instruction = Instruction {
            program_id: raydium_program_id,
            accounts,
            data,
        };

        // Build transaction
        let mut instructions = vec![];
        if let Some(ata_instruction) = create_ata_instruction {
            instructions.push(ata_instruction);
        }
        instructions.push(swap_instruction);

        let recent_blockhash = client.get_latest_blockhash()?;
        let message = Message::new(&instructions, Some(&payer.pubkey()));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.sign(&[&payer], recent_blockhash);

        // Send transaction
        let signature = client.send_and_confirm_transaction(&transaction)?;
        info!("Buy transaction signature: {}", signature);

        Ok(())
    }

    pub async fn sell_token(trade_info: TradeInfoFromToken) -> anyhow::Result<()> {
        let token_amount = trade_info.token_amount_list.token_post_amount
            - trade_info.token_amount_list.token_pre_amount;
        let sol_amount =
            trade_info.sol_amount_list.sol_post_amount - trade_info.sol_amount_list.sol_pre_amount;
        let Config { rpc_link, .. } = Config::new()?;
        let client = RpcClient::new(rpc_link);
        let payer = from_bytes_to_key_pair();
        let token_mint = Pubkey::from_str(&trade_info.mint)?;
        let pool_id = Pubkey::from_str(&trade_info.pool)?;
        let raydium_program_id = Pubkey::from_str(RAYDIUM_AMM_V4)?;
        let wsol_mint = Pubkey::from_str(WSOL)?;

        info!("Token amount: {}, SOL amount: {}", token_amount, sol_amount);

        // Fetch pool keys
        let pool_keys = get_pool_keys(&client, &pool_id).await?;

        // Determine base and quote vaults based on pool order
        let (base_vault, quote_vault, base_mint, quote_mint) = {
            let account = client.get_account(&pool_id)?;
            let pool_data = account.data;
            let amm_info = AmmInfo::try_from_slice(&pool_data[8..])?;
            if amm_info.token_0_mint == wsol_mint {
                (
                    pool_keys.base_vault,
                    pool_keys.quote_vault,
                    amm_info.token_0_mint,
                    amm_info.token_1_mint,
                )
            } else {
                (
                    pool_keys.quote_vault,
                    pool_keys.base_vault,
                    amm_info.token_1_mint,
                    amm_info.token_0_mint,
                )
            }
        };

        // Get user token accounts
        let user_source = get_associated_token_address(&payer.pubkey(), &token_mint); // Token
        let user_destination = get_associated_token_address(&payer.pubkey(), &wsol_mint); // SOL

        // Create associated token account if needed
        let create_ata_instruction = if client.get_account(&user_destination).is_err() {
            Some(
                spl_associated_token_account::instruction::create_associated_token_account(
                    &payer.pubkey(),
                    &payer.pubkey(),
                    &wsol_mint,
                    &spl_token::id(),
                ),
            )
        } else {
            None
        };

        // Swap parameters
        let amount_in: u64 = 100_000_000; // Example: 100 tokens (adjust for decimals)
        let minimum_amount_out: u64 = 0; // Adjust for slippage

        // Construct swap instruction accounts
        let accounts = vec![
            AccountMeta::new_readonly(spl_token::id(), false), // Token program
            AccountMeta::new(pool_id, false),                  // AMM pool
            AccountMeta::new_readonly(pool_keys.authority, false), // AMM authority
            AccountMeta::new(pool_keys.open_orders, false),    // AMM open orders
            AccountMeta::new(quote_vault, false),              // AMM base vault (Token)
            AccountMeta::new(base_vault, false),               // AMM quote vault (SOL)
            AccountMeta::new_readonly(pool_keys.market_program_id, false), // Serum program
            AccountMeta::new(pool_keys.market_id, false),      // Serum market
            AccountMeta::new(pool_keys.market_bids, false),    // Serum bids
            AccountMeta::new(pool_keys.market_asks, false),    // Serum asks
            AccountMeta::new(pool_keys.market_event_queue, false), // Serum event queue
            AccountMeta::new(pool_keys.market_quote_vault, false), // Serum base vault
            AccountMeta::new(pool_keys.market_base_vault, false), // Serum quote vault
            AccountMeta::new_readonly(pool_keys.market_vault_signer, false), // Serum vault signer
            AccountMeta::new(user_source, false),              // User source (Token)
            AccountMeta::new(user_destination, false),         // User destination (SOL)
            AccountMeta::new(payer.pubkey(), true),            // User owner
        ];

        // Swap instruction data
        let data = vec![
            vec![9_u8], // SwapBaseIn instruction index (verify with Raydium program)
            amount_in.to_le_bytes().to_vec(),
            minimum_amount_out.to_le_bytes().to_vec(),
        ]
        .concat();

        let swap_instruction = Instruction {
            program_id: raydium_program_id,
            accounts,
            data,
        };

        // Build transaction
        let mut instructions = vec![];
        if let Some(ata_instruction) = create_ata_instruction {
            instructions.push(ata_instruction);
        }
        instructions.push(swap_instruction);

        let recent_blockhash = client.get_latest_blockhash()?;
        let message = Message::new(&instructions, Some(&payer.pubkey()));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.sign(&[&payer], recent_blockhash);

        // Send transaction
        let signature = client.send_and_confirm_transaction(&transaction)?;
        info!("Sell transaction signature: {}", signature);

        Ok(())
    }
}
