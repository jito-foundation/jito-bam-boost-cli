use solana_pubkey::{pubkey, Pubkey};

pub mod bam_boost;
pub mod bam_boost_handler;
pub mod cli_args;
pub mod cli_config;

pub const JITOSOL_MINT: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");
pub const JITOSOL_POOL_ADDRESS: Pubkey = pubkey!("Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb");
pub const JITOSOL_POOL_MANAGER: Pubkey = pubkey!("5eosrve6LktMZgVNszYzebgmmC7BjLK8NoWyRQtcmGTF");
pub const JITOSOL_POOL_FEE: Pubkey = pubkey!("5eosrve6LktMZgVNszYzebgmmC7BjLK8NoWyRQtcmGTF");
pub const JITOSOL_RESERVE_STAKE: Pubkey = pubkey!("BgKUXdS29YcHCFrPm5M8oLHiTzZaMDjsebggjoaQ6KFL");
pub const JITOSOL_STAKE_POOL_WITHDRAW_AUTHORITY: Pubkey =
    pubkey!("6iQKfEyhr3bZMotVkW6beNZz5CPAkiwvgV2CTje9pVSS");
