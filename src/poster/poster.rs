use std::{fs, sync::mpsc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Poster {
    aggregator_url: String,
    proof_path: String,
}

impl Poster {
    pub fn new(aggregator_url: String, proof_path: String) -> Self {
        Self {
            aggregator_url,
            proof_path,
        }
    }

    pub async fn send_proof_to_aggregator(rx: mpsc::Receiver<u64>) {
        loop {
            let block_number = rx.recv();
            match block_number {
                Ok(bn) => {
                    if bn == 0u64 {
                        continue;
                    } else if bn == u64::MAX {
                        return;
                    }
                    let proof_file_path = format!("proofs/execution_proof_{}.proof", bn);
                    let proof_file = fs::File::open(proof_file_path).unwrap();
                    let proof_object: Result<sp1_sdk::SP1ProofWithPublicValues, serde_json::Error> =
                        serde_json::from_reader(proof_file);
                    match proof_object {
                        Ok(proof) => {
                            let plonk_proof = proof.proof.try_as_plonk().unwrap();
                            let encoded_proof = plonk_proof.encoded_proof;
                            let public_values = proof.public_values.raw();
                            // TODO: send json rpc to ...
                        }
                        Err(_) => println!("proof not found"),
                    }
                }
                Err(_) => break,
            }
        }
    }
}
