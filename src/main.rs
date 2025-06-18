mod client;
mod config;
pub mod decoder;
pub mod engine;
mod gen_engine;
pub mod keypair;
mod target_list;
mod trade_info;

use crate::client::SolGrpcClient;
use crate::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    env_logger::init();
    let Config {
        rpc_link,
        ws_link,
        grpc_link,
        private_key,
    } = Config::new()?;

    let client = SolGrpcClient::new(grpc_link);
    client.connect().await?;
    Ok(())
}
