#!/bin/bash
sh -c "$(curl -k -sSfL https://release.solana.com/v1.18.23/install)"
export PATH="/home/freemell/.cargo/bin:/home/freemell/.local/share/solana/install/active_release/bin:/home/freemell/.avm/bin:$PATH"
solana --version
anchor build
