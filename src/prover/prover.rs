use std::process::{Command, ExitStatus};

use serde::{Deserialize, Serialize};

use crate::poster::poster::Poster;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prover {
    last_proved_block: u64,
    proof_path: String,
    rpc_url: String,
    pub poster: Poster,
    chain_id: String,
}

impl Prover {
    pub fn new(
        last_proved_block: u64,
        proof_path: String,
        rpc_url: String,
        identifier: String,
        aggregator_url: String,
        chain_id: String,
    ) -> Self {
        let output = Command::new("which").args(["rsp"]).output().unwrap().stdout;
        if output.len() == 0 {
            panic!("rsp process not found in PATH")
        }

        let poster = Poster::new(aggregator_url, proof_path.clone(), identifier);

        Self {
            last_proved_block,
            proof_path,
            rpc_url,
            poster,
            chain_id,
        }
    }

    /// calculates the execution proof of the specified block using rsp and returns the file path.
    pub fn prove(&self, block_number: u64) -> ExitStatus {
        let output = Command::new("rsp")
            .args([
                "--block-number",
                &format!("{}", block_number),
                "--rpc-url",
                &self.rpc_url,
                "--chain-id",
                &self.chain_id,
                "--prove",
            ])
            .output()
            .expect("failed to run the process");
        output.status
    }
}
