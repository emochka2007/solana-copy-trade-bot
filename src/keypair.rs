use solana_sdk::signature::Keypair;
use std::env;

pub fn get_keypair(private_key: &str) -> Keypair {
    Keypair::from_base58_string(private_key)
}

// todo move to config
pub fn from_bytes_to_key_pair() -> Keypair {
    let pk_bytes = env::var("PK_SOLANA").expect("PK bytes is not found");
    let bytes: Vec<u8> = pk_bytes
        .trim_matches(&['[', ']'][..])
        .split(',')
        .map(|s| s.trim().parse::<u8>().expect("Error converting to bytes"))
        .collect();
    Keypair::from_bytes(&bytes).unwrap()
}
