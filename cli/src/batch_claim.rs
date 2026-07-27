use std::sync::Arc;

use jito_bam_boost_client::instructions::ClaimBuilder;
use jito_bam_boost_merkle_tree::bam_boost_merkle_tree::BamBoostMerkleTree;
use solana_keypair::Signer as _;
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;
use solana_transaction::{Instruction, Transaction};
use spl_associated_token_account_interface::{
    address::get_associated_token_address_with_program_id,
    instruction::create_associated_token_account_idempotent,
};

use crate::{cli_config::CliConfig, pda, scanner::Scanner, JITOSOL_MINT};

/// Builds the [create ATA, claim] instruction pair for one epoch.
pub fn build_claim_ixs(
    program_id: &Pubkey,
    claimant: &Pubkey,
    epoch: u64,
    amount: u64,
    proof: Vec<[u8; 32]>,
) -> Vec<Instruction> {
    let distributor_pda = pda::merkle_distributor_address(program_id, &JITOSOL_MINT, epoch);
    let distributor_token_address = get_associated_token_address_with_program_id(
        &distributor_pda,
        &JITOSOL_MINT,
        &spl_token_interface::id(),
    );
    let claim_status_pda = pda::claim_status_address(program_id, claimant, &distributor_pda);
    let claimant_token_address = get_associated_token_address_with_program_id(
        claimant,
        &JITOSOL_MINT,
        &spl_token_interface::id(),
    );

    let mut ix_builder = ClaimBuilder::new();
    ix_builder
        .distributor(distributor_pda)
        .claim_status(claim_status_pda)
        .from(distributor_token_address)
        .to(claimant_token_address)
        .claimant(*claimant)
        .token_program(spl_token_interface::id())
        .amount(amount)
        .proof(proof);
    let mut claim_ix = ix_builder.instruction();
    claim_ix.program_id = *program_id;

    vec![
        create_associated_token_account_idempotent(
            claimant,
            claimant,
            &JITOSOL_MINT,
            &spl_token_interface::id(),
        ),
        claim_ix,
    ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimState {
    Started,
    Success(String),
    Failed(String),
    Skipped(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimEvent {
    pub epoch: u64,
    pub state: ClaimState,
}

/// Claims each epoch sequentially; one failure does not stop the rest.
pub async fn claim_epochs(
    scanner: &Scanner,
    cli_config: &CliConfig,
    program_id: &Pubkey,
    network: &str,
    epochs: &[u64],
    progress: &mut (dyn FnMut(ClaimEvent) + Send),
) -> anyhow::Result<Vec<ClaimEvent>> {
    let signer = cli_config
        .signer
        .clone()
        .ok_or_else(|| anyhow::anyhow!("signer is required"))?;
    let rpc = RpcClient::new_with_commitment(cli_config.rpc_url.clone(), cli_config.commitment);
    let mut results = Vec::with_capacity(epochs.len());

    for &epoch in epochs {
        progress(ClaimEvent {
            epoch,
            state: ClaimState::Started,
        });
        let state = claim_one(scanner, &rpc, &signer, program_id, network, epoch).await;
        let event = ClaimEvent { epoch, state };
        progress(event.clone());
        results.push(event);
    }
    Ok(results)
}

async fn claim_one(
    scanner: &Scanner,
    rpc: &RpcClient,
    signer: &Arc<solana_keypair::Keypair>,
    program_id: &Pubkey,
    network: &str,
    epoch: u64,
) -> ClaimState {
    let entries = match scanner.fetch_entries(network, epoch).await {
        Ok(Some(entries)) => entries,
        Ok(None) => return ClaimState::Skipped("no distribution for epoch".into()),
        Err(e) => return ClaimState::Failed(format!("fetch merkle tree: {e}")),
    };

    let tree = match BamBoostMerkleTree::new_from_entries(entries) {
        Ok(tree) => tree,
        Err(e) => return ClaimState::Failed(format!("build merkle tree: {e}")),
    };

    let claimant = signer.pubkey();
    let Some(node) = tree
        .tree_nodes
        .iter()
        .find(|n| n.claimant.to_bytes() == claimant.to_bytes())
        .cloned()
    else {
        return ClaimState::Skipped("claimant not in tree".into());
    };
    let Some(proof) = node.proof.clone() else {
        return ClaimState::Failed("merkle proof missing".into());
    };

    let distributor = pda::merkle_distributor_address(program_id, &JITOSOL_MINT, epoch);
    let claim_status = pda::claim_status_address(program_id, &claimant, &distributor);
    if rpc.get_account(&claim_status).is_ok() {
        return ClaimState::Skipped("already claimed".into());
    }

    let ixs = build_claim_ixs(program_id, &claimant, epoch, node.amount, proof);
    let blockhash = match rpc.get_latest_blockhash() {
        Ok(b) => b,
        Err(e) => return ClaimState::Failed(format!("blockhash: {e}")),
    };
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&claimant),
        std::slice::from_ref(signer),
        blockhash,
    );
    match rpc.send_and_confirm_transaction(&tx) {
        Ok(sig) => ClaimState::Success(sig.to_string()),
        Err(e) => ClaimState::Failed(format!("send: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_pubkey::Pubkey;
    use std::str::FromStr;

    #[test]
    fn builds_ata_then_claim_instruction() {
        let program_id = Pubkey::from_str("BoostxbPp2ENYHGcTLYt1obpcY13HE4NojdqNWdzqSSb").unwrap();
        let claimant = Pubkey::new_unique();
        let proof = vec![[7u8; 32]];

        let ixs = build_claim_ixs(&program_id, &claimant, 42, 1000, proof);

        assert_eq!(ixs.len(), 2);
        // First: ATA creation for the claimant's JitoSOL account.
        assert_eq!(
            ixs[0].program_id.to_string(),
            "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        );
        // Second: claim instruction owned by the BAM Boost program.
        assert_eq!(ixs[1].program_id, program_id);
        // Claimant must be a signer of the claim instruction.
        assert!(ixs[1]
            .accounts
            .iter()
            .any(|m| m.pubkey == claimant && m.is_signer));
    }
}
