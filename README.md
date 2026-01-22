# Jito BAM Boost CLI

## How to Claim Subsidy

Validators can claim their allocated JitoSOL rewards by providing a merkle proof. Use the CLI to claim rewards for a specific epoch:

**NOTE: Signer is the validator's identity keypair**

```bash
cargo r -p jito-bam-boost-cli -- \
    bam-boost \
    merkle-distributor \
    claim \
    --network mainnet \
    --epoch <EPOCH> \
    --rpc-url <RPC_URL> \
    --signer <PATH_TO_KEYPAIR> \
    --commitment confirmed \
    --jito-bam-boost-program-id BoostxbPp2ENYHGcTLYt1obpcY13HE4NojdqNWdzqSSb
```
