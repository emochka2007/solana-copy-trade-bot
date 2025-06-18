use std::env;
use std::env::VarError;
pub const WSOL: &str = "So11111111111111111111111111111111111111112";

pub struct Config {
    pub rpc_link: String,
    pub ws_link: String,
    pub grpc_link: String,
    pub private_key: String,
    pub target_wallet: String,
}
impl Config {
    pub fn new() -> Result<Self, VarError> {
        Ok(Self {
            rpc_link: env::var("RPC_SOLANA")?,
            ws_link: env::var("WS_SOLANA")?,
            grpc_link: env::var("GRPC_SOLANA")?,
            private_key: env::var("PK_SOLANA")?,
            target_wallet: env::var("TARGET_WALLET")?,
        })
    }
}
