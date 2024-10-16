use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, io::Read, sync::mpsc};

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

    pub async fn send_proof_to_aggregator(&self, rx: mpsc::Receiver<u64>) {
        loop {
            let block_number = rx.recv();
            match block_number {
                Ok(bn) => {
                    if bn == 0u64 {
                        continue;
                    } else if bn == u64::MAX {
                        return;
                    }
                    let proof_file_path =
                        format!("{}/execution_proof_{}.proof", self.proof_path, bn);
                    let mut proof_file = fs::File::open(proof_file_path).unwrap();
                    let mut proof_buffer = String::new();
                    let proof_json = proof_file.read_to_string(&mut proof_buffer);
                    match proof_json {
                        Ok(_) => {
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
                                    .await
                                    .unwrap();
                                if !response.status().is_success() {
                                    if retry < MAX_RETRY {
                                        retry += 1;
                                        continue;
                                    } else {
                                        println!("Proof could not be sent to the aggregator"); // TODO extended retry logic
                                        break;
                                    }
                                }
                                break;
                            }
                        }
                        Err(_) => println!("proof not found"),
                    }
                }
                Err(_) => break,
            }
        }
    }
}
