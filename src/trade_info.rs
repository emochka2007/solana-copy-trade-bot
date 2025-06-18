use crate::config::WSOL;
use anyhow::anyhow;
use log::info;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use yellowstone_grpc_proto::geyser::SubscribeUpdateTransaction;

#[derive(Clone, Debug)]
pub struct TokenAmountList {
    pub token_pre_amount: f64,
    pub token_post_amount: f64,
}

#[derive(Clone, Debug)]
pub struct SolAmountList {
    pub sol_pre_amount: f64,
    pub sol_post_amount: f64,
}

#[derive(Clone, Debug)]
pub enum TradeType {
    Buy,
    Sell,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct TradeInfoFromToken {
    pub slot: u64,
    pub recent_blockhash: Hash,
    pub signature: String,
    pub target: String,
    pub mint: String,
    pub token_amount_list: TokenAmountList,
    pub sol_amount_list: SolAmountList,
    pub pool: String,
    pub decimal: u32,
    pub trade_type: TradeType,
}

impl TradeInfoFromToken {
    pub fn from_update(txn: SubscribeUpdateTransaction) -> anyhow::Result<Self> {
        let slot = txn.slot;
        let (
            recent_blockhash,
            signature,
            target,
            mint,
            token_amount_list,
            sol_amount_list,
            bonding_curve,
            mint_decimal,
            trade_type,
        ) = if let Some(transaction) = txn.transaction {
            let signature = match Signature::try_from(transaction.signature.clone()) {
                Ok(signature) => format!("{:?}", signature),
                Err(_) => "".to_string(),
            };
            let recent_blockhash_slice = &transaction
                .transaction
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Failed to get recent blockhash"))?
                .message
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Failed to get recent blockhash"))?
                .recent_blockhash;
            let recent_blockhash = Hash::new(recent_blockhash_slice);

            let mut mint = String::new();
            let mut bonding_curve = String::new();
            let mut sol_pre_amount = 0_f64;
            let mut sol_post_amount = 0_f64;
            let mut token_pre_amount = 0_f64;
            let mut token_post_amount = 0_f64;
            let mut mint_decimal = 6_u32; // Default to 6 decimals if not found

            // Retrieve Target Wallet Pubkey
            let account_keys = &transaction
                .transaction
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Failed to get account keys"))?
                .message
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Failed to get account keys"))?
                .account_keys;

            let target = Pubkey::try_from(account_keys[0].clone())
                .map_err(|_| anyhow::anyhow!("Failed to parse target pubkey"))?
                .to_string();

            if let Some(meta) = transaction.meta.clone() {
                if let Some(error) = meta.err {
                    return Err(anyhow!("Error in transaction"));
                }
                // Identify mint and bonding curve
                for balance in meta.post_token_balances.iter() {
                    let owner = balance.owner.clone();
                    if owner != target && balance.mint != WSOL {
                        bonding_curve = owner.clone(); // Assume non-target owner is the pool
                    }
                    if (owner == target || owner == bonding_curve) && balance.mint != WSOL {
                        mint = balance.mint.clone();
                    }
                }
                if mint.is_empty() {
                    for balance in meta.pre_token_balances.iter() {
                        let owner = balance.owner.clone();
                        if owner != target && balance.mint != WSOL {
                            bonding_curve = owner.clone();
                        }
                        if (owner == target || owner == bonding_curve) && balance.mint != WSOL {
                            mint = balance.mint.clone();
                        }
                    }
                }

                if mint.is_empty() {
                    return Err(anyhow::anyhow!(format!(
                        "signature[{}]: mint is None",
                        signature
                    )));
                }

                // Calculate SOL and token balances for the target wallet
                for balance in meta.pre_token_balances.iter() {
                    if balance.owner == target {
                        if balance.mint == WSOL {
                            sol_pre_amount = balance
                                .ui_token_amount
                                .as_ref()
                                .map(|ui| ui.ui_amount)
                                .unwrap_or(0_f64);
                        } else if balance.mint == mint {
                            token_pre_amount = balance
                                .ui_token_amount
                                .as_ref()
                                .map(|ui| ui.ui_amount)
                                .unwrap_or(0_f64);
                            mint_decimal = balance
                                .ui_token_amount
                                .as_ref()
                                .map(|ui| ui.decimals)
                                .unwrap_or(6_u32);
                        }
                    }
                }

                for balance in meta.post_token_balances.iter() {
                    if balance.owner == target {
                        if balance.mint == WSOL {
                            sol_post_amount = balance
                                .ui_token_amount
                                .as_ref()
                                .map(|ui| ui.ui_amount)
                                .unwrap_or(0_f64);
                        } else if balance.mint == mint {
                            token_post_amount = balance
                                .ui_token_amount
                                .as_ref()
                                .map(|ui| ui.ui_amount)
                                .unwrap_or(0_f64);
                            mint_decimal = balance
                                .ui_token_amount
                                .as_ref()
                                .map(|ui| ui.decimals)
                                .unwrap_or(mint_decimal);
                        }
                    }
                }
            } else {
                return Err(anyhow::anyhow!("Transaction meta is None"));
            }

            let token_amount_list = TokenAmountList {
                token_pre_amount,
                token_post_amount,
            };

            let sol_amount_list = SolAmountList {
                sol_pre_amount,
                sol_post_amount,
            };
            if token_post_amount != 0.0 {
                // info!("TOKEN {:?}", token_amount_list);
                // info!("sig {}", signature);
                // info!("Sol {:?}", transaction.meta);
            }

            // Determine trade type
            let trade_type = if token_post_amount > token_pre_amount {
                TradeType::Buy
            } else if token_pre_amount > token_post_amount {
                TradeType::Sell
            } else {
                TradeType::Unknown
            };

            (
                recent_blockhash,
                signature,
                target,
                mint,
                token_amount_list,
                sol_amount_list,
                bonding_curve,
                mint_decimal,
                trade_type,
            )
        } else {
            return Err(anyhow::anyhow!("Transaction is None"));
        };

        Ok(Self {
            slot,
            recent_blockhash,
            signature,
            target,
            mint,
            token_amount_list,
            sol_amount_list,
            pool: bonding_curve,
            decimal: mint_decimal,
            trade_type,
        })
    }
}
