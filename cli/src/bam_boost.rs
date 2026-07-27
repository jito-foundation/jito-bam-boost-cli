use clap::{Subcommand, ValueEnum};
use solana_pubkey::Pubkey;

/// Network type for subsidy schedule
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum NetworkArg {
    Mainnet,
    Testnet,
}

/// The CLI handler for the bam_boost program
#[derive(Subcommand)]
pub enum BamBoostCommands {
    /// MerkleDistributor operations
    MerkleDistributor {
        #[command(subcommand)]
        action: MerkleDistributorActions,
    },

    /// ClaimStatus operations
    ClaimStatus {
        #[command(subcommand)]
        action: ClaimStatusActions,
    },
}

/// The actions that can be performed on the bam_boost MerkleDistributor
#[derive(Subcommand)]
pub enum MerkleDistributorActions {
    /// Claim
    Claim {
        /// Network type (mainnet or testnet)
        #[arg(long, value_enum)]
        network: NetworkArg,

        /// Epoch number
        #[arg(long)]
        epoch: u64,
    },

    /// Show subsidy status for every published epoch
    Status {
        /// Network type (mainnet or testnet)
        #[arg(long, value_enum)]
        network: NetworkArg,

        /// Claimant pubkey (validator identity); no keypair needed
        #[arg(long)]
        claimant: Pubkey,
    },

    /// Claim every unclaimed epoch for the signer
    ClaimAll {
        /// Network type (mainnet or testnet)
        #[arg(long, value_enum)]
        network: NetworkArg,

        /// Skip the interactive confirmation
        #[arg(long, default_value = "false")]
        yes: bool,
    },
}

/// The actions that can be performed on the bam_boost ClaimStatus
#[derive(Subcommand)]
pub enum ClaimStatusActions {
    /// Get ClaimStatus
    Get {
        /// Epoch number
        #[arg(long)]
        epoch: u64,

        /// Claimant
        #[arg(long)]
        claimant: Pubkey,
    },
}
