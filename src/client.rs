use crate::decoder;
use crate::target_list::Targetlist;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use yellowstone_grpc_proto::geyser::{
    SubscribeRequestFilterTransactions, SubscribeUpdateTransaction,
};
use yellowstone_grpc_proto::prost::bytes::Bytes;
use {
    futures::{sink::SinkExt, stream::StreamExt},
    log::info,
    tokio::time::{Duration, interval},
    tonic::transport::channel::ClientTlsConfig,
    yellowstone_grpc_client::GeyserGrpcClient,
    yellowstone_grpc_proto::prelude::{
        CommitmentLevel, SubscribeRequest, SubscribeRequestFilterSlots, SubscribeRequestPing,
        SubscribeUpdatePong, SubscribeUpdateSlot, subscribe_update::UpdateOneof,
    },
};

pub struct SolGrpcClient {
    endpoint: String,
}
impl SolGrpcClient {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }
    pub async fn connect(&self) -> anyhow::Result<()> {
        let endpoint = self.endpoint.clone();
        let mut client = GeyserGrpcClient::build_from_shared(endpoint)?
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .connect()
            .await?;
        let (mut subscribe_tx, mut stream) = client.subscribe().await?;

        futures::try_join!(
            async move {
                let raydium_account = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string();
                subscribe_tx
                    .send(SubscribeRequest {
                        transactions: maplit::hashmap! {
                            "".to_owned() => SubscribeRequestFilterTransactions {
                                vote: None,failed: None,signature: None,account_include: vec![raydium_account] ,account_exclude: vec![],
                            account_required: vec![],}
                        },
                        commitment: Some(CommitmentLevel::Processed as i32),
                        ..Default::default()
                    })
                    .await?;

                let mut timer = interval(Duration::from_secs(3));
                let mut id = 0;
                loop {
                    timer.tick().await;
                    id += 1;
                    subscribe_tx
                        .send(SubscribeRequest {
                            ping: Some(SubscribeRequestPing { id }),
                            ..Default::default()
                        })
                        .await?;
                }
                #[allow(unreachable_code)]
                Ok::<(), anyhow::Error>(())
            },
            async move {
                let target_list = Targetlist::new("target_list.txt")?;
                let token_list = Targetlist::new("tokens_list.txt")?;

                while let Some(message) = stream.next().await {
                    match message?.update_oneof.expect("valid message") {
                        UpdateOneof::Transaction(transaction) => {
                            // info!("slot received: {slot}");
                            match decoder::decode_instruction(
                                target_list.clone(),
                                token_list.clone(),
                                transaction,
                            ) {
                                Ok(_) => {}
                                Err(_) => {}
                            }
                        }
                        UpdateOneof::Slot(SubscribeUpdateSlot { slot, .. }) => {
                            info!("slot received: {slot}");
                        }
                        UpdateOneof::Ping(_msg) => {
                            info!("ping received");
                        }
                        UpdateOneof::Pong(SubscribeUpdatePong { id }) => {
                            info!("pong received: id#{id}");
                        }
                        msg => anyhow::bail!("received unexpected message: {msg:?}"),
                    }
                }
                Ok::<(), anyhow::Error>(())
            }
        )?;

        Ok(())
    }
}
