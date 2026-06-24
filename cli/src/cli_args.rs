use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::bam_boost::BamBoostCommands;

/// Output format for built unsigned transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum OutputFormat {
    /// One base58 unsigned transaction per line (default).
    #[default]
    Text,
    /// A JSON array of `{unsigned_tx_base58, unsigned_tx_base64}` objects —
    /// only the serialized transaction, no sidecar metadata.
    Json,
}

/// Lighthouse deploy-slot guard configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeploySlotGuard {
    /// Resolve the BAM Boost ProgramData deploy slot from RPC at build time
    /// and assert on it (default).
    Auto,
    /// Assert on an explicit slot number.
    Slot(u64),
    /// Disable the guard entirely.
    Off,
}

/// Parse the `--assert-deploy-slot` value: `auto`, a slot number, or `off`.
pub fn parse_deploy_slot_guard(s: &str) -> Result<DeploySlotGuard, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(DeploySlotGuard::Auto),
        "off" | "none" | "disabled" | "false" => Ok(DeploySlotGuard::Off),
        other => other
            .parse::<u64>()
            .map(DeploySlotGuard::Slot)
            .map_err(|_| format!("invalid --assert-deploy-slot '{s}': expected 'auto', 'off', or a slot number")),
    }
}

#[derive(Parser)]
#[command(author, version, about = "A CLI for managing BAM Boost operations", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<ProgramCommand>,

    #[arg(long, global = true, help = "Path to the configuration file")]
    pub config_file: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        default_value = "https://api.mainnet-beta.solana.com",
        help = "RPC URL to use"
    )]
    pub rpc_url: Option<String>,

    #[arg(long, global = true, help = "Commitment level")]
    pub commitment: Option<String>,

    #[arg(long, global = true, help = "BAM Boost program ID")]
    pub jito_bam_boost_program_id: Option<String>,

    #[arg(long, global = true, help = "Filepath or URL to a keypair", conflicts_with = "address")]
    pub signer: Option<String>,

    #[arg(
        long,
        global = true,
        help = "Claimant pubkey (offline mode, no signing). Mutually exclusive with --signer",
        conflicts_with = "signer"
    )]
    pub address: Option<String>,

    #[arg(long, global = true, help = "Verbose mode")]
    pub verbose: bool,

    #[arg(
        long,
        global = true,
        default_value = "false",
        help = "This will print out the raw TX instead of running it"
    )]
    pub print_tx: bool,

    #[arg(
        long,
        global = true,
        default_value = "false",
        help = "This will print out account information in JSON format"
    )]
    pub print_json: bool,

    #[arg(
        long,
        global = true,
        default_value = "false",
        help = "This will print out account information in JSON format with reserved space"
    )]
    pub print_json_with_reserves: bool,

    #[arg(long, global = true, hide = true)]
    pub markdown_help: bool,

    #[arg(
        long,
        global = true,
        value_enum,
        default_value_t = OutputFormat::Text,
        help = "Output format for built unsigned transactions: 'text' (one base58 tx per line) or 'json' (array of {unsigned_tx_base58, unsigned_tx_base64} — serialized tx only, no metadata)"
    )]
    pub output: OutputFormat,

    #[arg(
        long,
        global = true,
        default_value = "auto",
        value_parser = parse_deploy_slot_guard,
        help = "Lighthouse program-integrity guard: \"auto\" (resolve current ProgramData deploy slot from RPC — default), an explicit slot number, or \"off\" to disable"
    )]
    pub assert_deploy_slot: DeploySlotGuard,
}

#[derive(Debug, Subcommand)]
pub enum ProgramCommand {
    /// BAM Boost program commands
    BamBoost {
        #[command(subcommand)]
        action: BamBoostCommands,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bam_boost::{MerkleDistributorActions, NetworkArg};

    #[test]
    fn test_rpc_default_is_mainnet() {
        let cli = Cli::parse_from(["jito-bam-boost-cli"]);
        assert_eq!(
            cli.rpc_url.as_deref(),
            Some("https://api.mainnet-beta.solana.com")
        );
    }

    #[test]
    fn test_network_default_is_mainnet() {
        let cli = Cli::parse_from([
            "jito-bam-boost-cli",
            "bam-boost",
            "merkle-distributor",
            "claim",
            "--epoch",
            "1",
        ]);
        match cli.command {
            Some(ProgramCommand::BamBoost {
                action:
                    crate::bam_boost::BamBoostCommands::MerkleDistributor {
                        action: MerkleDistributorActions::Claim { network, .. },
                    },
            }) => {
                assert!(
                    matches!(network, NetworkArg::Mainnet),
                    "Expected default network to be Mainnet, got {:?}",
                    network
                );
            }
            other => panic!("Unexpected command variant: {:?}", other),
        }
    }

    #[test]
    fn test_claim_epoch_and_first_epoch_optional() {
        // No --epoch and no --first-epoch => scan-all with auto-discovered start.
        let cli = Cli::parse_from([
            "jito-bam-boost-cli",
            "bam-boost",
            "merkle-distributor",
            "claim",
        ]);
        match cli.command {
            Some(ProgramCommand::BamBoost {
                action:
                    crate::bam_boost::BamBoostCommands::MerkleDistributor {
                        action:
                            MerkleDistributorActions::Claim {
                                epoch, first_epoch, ..
                            },
                    },
            }) => {
                assert_eq!(epoch, None, "Expected epoch to be optional/None when omitted");
                assert_eq!(
                    first_epoch, None,
                    "Expected first_epoch to be None (auto-discover) when omitted"
                );
            }
            other => panic!("Unexpected command variant: {:?}", other),
        }
    }

    #[test]
    fn test_claim_single_epoch_parses() {
        let cli = Cli::parse_from([
            "jito-bam-boost-cli",
            "bam-boost",
            "merkle-distributor",
            "claim",
            "--epoch",
            "950",
            "--first-epoch",
            "900",
        ]);
        match cli.command {
            Some(ProgramCommand::BamBoost {
                action:
                    crate::bam_boost::BamBoostCommands::MerkleDistributor {
                        action:
                            MerkleDistributorActions::Claim {
                                epoch, first_epoch, ..
                            },
                    },
            }) => {
                assert_eq!(epoch, Some(950));
                assert_eq!(first_epoch, Some(900));
            }
            other => panic!("Unexpected command variant: {:?}", other),
        }
    }

    #[test]
    fn test_address_and_signer_mutually_exclusive() {
        let result = Cli::try_parse_from([
            "jito-bam-boost-cli",
            "--signer",
            "/tmp/key.json",
            "--address",
            "11111111111111111111111111111111",
        ]);
        assert!(result.is_err(), "Expected error when both --signer and --address are provided");
    }
}
