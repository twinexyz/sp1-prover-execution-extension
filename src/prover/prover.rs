use std::process::{Command, ExitStatus};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prover {
    last_proved_block: u64,
    proof_path: String,
    rpc_url: String,
}

impl Prover {
    pub fn new(last_proved_block: u64, proof_path: String, rpc_url: String) -> Self {
        let output = Command::new("which").args(["rsp"]).output().unwrap().stdout;
        if output.len() == 0 {
            panic!("rsp process not found in PATH")
        }

        Self {
            last_proved_block,
            proof_path,
            rpc_url,
        }
    }

    /// calculates the execution proof of the specified block using rsp and returns the file path.
    pub fn prove(&self, block_number: u64) -> ExitStatus {
        let output = Command::new("rsp")
            .args([
                "--block-number",
                &format!("{}", block_number),
                "--rpc-url",
                "http://localhost:8545/",
                "--chain-id",
                "1337",
                "--prove",
            ])
            .output()
            .expect("failed to run the process");
        output.status
    }
}
