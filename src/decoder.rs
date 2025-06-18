use crate::target_list::Targetlist;
use crate::trade_info::{TradeInfoFromToken, TradeType};
use log::{debug, info};
use yellowstone_grpc_proto::geyser::SubscribeUpdateTransaction;

pub fn decode_instruction(
    target_list: Targetlist,
    token_list: Targetlist,
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
        if target_list.is_listed_on_target(&trade_info.target)
            && token_list.is_listed_on_target(&trade_info.mint)
        {
            match trade_info.trade_type {
                TradeType::Buy => info!("Buy transaction detected: {:?}", trade_info.signature),
                TradeType::Sell => {
                    info!("Sell transaction detected: {:?}", trade_info.signature)
                }
                TradeType::Unknown => debug!("Unknown trade type: {:?}", trade_info.signature),
            }
            if let Some(_log) = log_messages.into_iter().next() {};
        }
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
