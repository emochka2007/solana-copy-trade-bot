use crate::engine::Engine;
use crate::gen_engine;
use crate::target_list::TargetList;
use crate::trade_info::{TradeInfoFromToken, TradeType};
use log::{debug, info};
use yellowstone_grpc_proto::geyser::SubscribeUpdateTransaction;

pub async fn decode_instruction(
    target_list: TargetList,
    token_list: TargetList,
    transaction: SubscribeUpdateTransaction,
) -> anyhow::Result<()> {
    if let Some(log_messages) = transaction
        .clone()
        .transaction
        .unwrap()
        .meta
        .map(|meta| meta.log_messages)
    {
        let trade_info = TradeInfoFromToken::from_update(transaction.clone())?;
        match trade_info.trade_type {
            TradeType::Buy => {
                info!("Buy transaction detected: {:?}", trade_info.signature);
                gen_engine::Engine::buy_token(trade_info).await.unwrap();
            }
            TradeType::Sell => {
                debug!("Sell transaction detected: {:?}", trade_info.signature)
            }
            TradeType::Unknown => debug!("Unknown trade type: {:?}", trade_info.signature),
        }

        // todo uncomment
        // if target_list.is_listed_on_target(&trade_info.target)
        //     && token_list.is_listed_on_target(&trade_info.mint)
        // {
        //     // todo make it configurable in env for example -> ONLY_BUY or ONLY_SELL
        //     match trade_info.trade_type {
        //         TradeType::Buy => {
        //             info!("Buy transaction detected: {:?}", trade_info.signature);
        //             Engine::buy_token(trade_info)
        //         }
        //         TradeType::Sell => {
        //             debug!("Sell transaction detected: {:?}", trade_info.signature)
        //         }
        //         TradeType::Unknown => debug!("Unknown trade type: {:?}", trade_info.signature),
        //     }
        //     if let Some(_log) = log_messages.into_iter().next() {};
        // }
    }
    Ok(())
}

pub fn parse_logs(logs: Vec<String>) {
    for log in logs {
        if log.contains("swap") {
            info!("")
        }
    }
}
