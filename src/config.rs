use std::env;
use std::env::VarError;
pub const WSOL: &str = "So11111111111111111111111111111111111111112";
pub const RAYDIUM_AUTHORITY_V4: &str = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1";
pub const RAYDIUM_LIQUIDITY_POOL_V4_PROGRAM_ID: &str =
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

pub struct Config {
    pub rpc_link: String,
    pub ws_link: String,
    pub grpc_link: String,
    pub private_key: String,
}

impl Config {
    pub fn new() -> Result<Self, VarError> {
        Ok(Self {
            rpc_link: env::var("RPC_SOLANA")?,
            ws_link: env::var("WS_SOLANA")?,
            grpc_link: env::var("GRPC_SOLANA")?,
            private_key: env::var("PK_SOLANA")?,
        })
    }
}
