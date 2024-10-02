use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
struct StorageProof {
    key: String,
    proof: Vec<String>,
    value: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EthProofResult {
    account_proof: Vec<String>,
    balance: String,
    code_hash: String,
    nonce: String,
    storage_hash: String,
    storage_proof: Vec<StorageProof>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EthRpcResponse {
    jsonrpc: String,
    id: u64,
    result: EthProofResult,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define the Ethereum JSON-RPC endpoint (your node URL)
    let url = "http://localhost:8545"; // Replace with your Ethereum node's URL

    // Define the account and storage keys for which you want to get proof
    let account = "0x1CBd3b2770909D4e10f157cABC84C7264073C9Ec"; // Replace with the target account
    let storage_keys = vec![
        "0x0000000000000000000000000000000000000000000000000000000000000000" // Replace with the storage keys you're interested in
    ];

    // Prepare the JSON-RPC payload for eth_getProof
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_getProof",
        "params": [
            account,
            storage_keys,
            "latest" // Specify the block number or "latest" for the most recent block
        ],
        "id": 1
    });

    // Create an HTTP client
    let client = Client::new();

    // Send the JSON-RPC request
    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await?;

    // Deserialize the JSON-RPC response
    let rpc_response: EthRpcResponse = response.json().await?;

    // Print the account and storage proof
    println!("Account Proof: {:?}", rpc_response.result.account_proof);
    println!("Balance: {}", rpc_response.result.balance);
    println!("Nonce: {}", rpc_response.result.nonce);
    println!("Code Hash: {}", rpc_response.result.code_hash);
    println!("Storage Hash: {}", rpc_response.result.storage_hash);
    println!("Storage Proofs: {:?}", rpc_response.result.storage_proof);

    Ok(())
}