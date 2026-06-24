use std::sync::Arc;

use solana_commitment_config::CommitmentConfig;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;

pub struct CliConfig {
    /// The RPC endpoint URL
    pub rpc_url: String,

    /// The commitment level
    pub commitment: CommitmentConfig,

    /// Optional signer
    pub signer: Option<Arc<Keypair>>,

    /// Optional claimant address (offline mode, no signing)
    pub address: Option<Pubkey>,
}
