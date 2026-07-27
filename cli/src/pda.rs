use solana_pubkey::Pubkey;

/// PDA of the MerkleDistributor for a given mint and epoch.
pub fn merkle_distributor_address(program_id: &Pubkey, mint: &Pubkey, epoch: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"merkle_distributor",
            mint.to_bytes().as_slice(),
            epoch.to_le_bytes().as_slice(),
        ],
        program_id,
    )
    .0
}

/// PDA of a claimant's ClaimStatus for a given distributor.
pub fn claim_status_address(
    program_id: &Pubkey,
    claimant: &Pubkey,
    distributor: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"claim_status",
            claimant.to_bytes().as_slice(),
            distributor.to_bytes().as_slice(),
        ],
        program_id,
    )
    .0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JITOSOL_MINT;
    use std::str::FromStr;

    fn program_id() -> Pubkey {
        Pubkey::from_str("BoostxbPp2ENYHGcTLYt1obpcY13HE4NojdqNWdzqSSb").unwrap()
    }

    #[test]
    fn distributor_pda_matches_seed_oracle() {
        let epoch = 1000u64;
        let expected = Pubkey::find_program_address(
            &[
                b"merkle_distributor",
                JITOSOL_MINT.to_bytes().as_slice(),
                epoch.to_le_bytes().as_slice(),
            ],
            &program_id(),
        )
        .0;
        assert_eq!(
            merkle_distributor_address(&program_id(), &JITOSOL_MINT, epoch),
            expected
        );
    }

    #[test]
    fn claim_status_pda_matches_seed_oracle() {
        let claimant = Pubkey::new_unique();
        let distributor = merkle_distributor_address(&program_id(), &JITOSOL_MINT, 5);
        let expected = Pubkey::find_program_address(
            &[
                b"claim_status",
                claimant.to_bytes().as_slice(),
                distributor.to_bytes().as_slice(),
            ],
            &program_id(),
        )
        .0;
        assert_eq!(
            claim_status_address(&program_id(), &claimant, &distributor),
            expected
        );
    }
}
