use reqwest::Client;
use reth_tracing::tracing::error;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Poster {
    aggregator_url: String,
    proof_path: String,
    identifier: String,
}

static MAX_RETRY: u8 = 10;

impl Poster {
    pub fn new(aggregator_url: String, proof_path: String, identifier: String) -> Self {
        Self {
            aggregator_url,
            proof_path,
            identifier,
        }
    }

    pub async fn send_proof_to_aggregator(&self, block_height: u64) {
        let proof_file_path = format!("{}/execution_proof_{}.proof", self.proof_path, block_height);
        let proof_file = fs::File::open(proof_file_path).expect(format!("proof file for block {} not found", block_height).as_str()); 
        let proof_object: Result<sp1_sdk::SP1ProofWithPublicValues, serde_json::Error> =
            serde_json::from_reader(proof_file);

        match proof_object {
            Ok(proof_buffer) => {
                let payload = json!({
                    "jsonrpc": "2.0",
                    "method": "twarb_sendProof",
                    "params": [
                        {
                            "type": "SP1Proof",
                            "identifier": &self.identifier,
                            "proof": proof_buffer
                        }
                        ],
                    "id": 1
                });

                let client = Client::new();
                let mut retry = 0u8;
                loop {
                    let response = client
                        .post(&self.aggregator_url)
                        .json(&payload)
                        .send()
                        .await;

                    match response {
                        Ok(res) => {
                            if !res.status().is_success() {
                                if retry < MAX_RETRY {
                                    retry += 1;
                                    continue;
                                } else {
                                    error!("Proof could not be sent to the aggregator"); // TODO extended retry logic
                                    break;
                                }
                            }
                            break;
                        }
                        Err(_) => {
                            error!("Proof sending failed.");
                            break;
                        }
                    }
                }
            }
            Err(_) => error!("proof not found"),
        }
    }
}
