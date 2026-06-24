use clap::{Subcommand, ValueEnum};
use solana_pubkey::Pubkey;

/// Network type for subsidy schedule
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum NetworkArg {
    Mainnet,
    Testnet,
}

/// The CLI handler for the bam_boost program
#[derive(Debug, Subcommand)]
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
#[derive(Debug, Subcommand)]
pub enum MerkleDistributorActions {
    /// Claim
    Claim {
        /// Network type (mainnet or testnet)
        #[arg(long, value_enum, default_value = "mainnet")]
        network: NetworkArg,

        /// Epoch number. If omitted, scans every epoch from --first-epoch
        /// through the current epoch and claims each eligible one.
        #[arg(long)]
        epoch: Option<u64>,

        /// First epoch to scan when --epoch is omitted (scan-all mode).
        /// If also omitted, the CLI walks back from the current epoch and
        /// auto-discovers the earliest epoch with a published merkle tree.
        #[arg(long)]
        first_epoch: Option<u64>,
    },
}

/// The actions that can be performed on the bam_boost ClaimStatus
#[derive(Debug, Subcommand)]
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
