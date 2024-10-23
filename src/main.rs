//! Example of using the WS provider to subscribe to new blocks.

use std::time::Instant;
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use eyre::Result;
use futures_util::StreamExt;
mod poster;
mod prover;
use dotenv::dotenv;
use prover::prover::Prover;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let prover = initialize_prover();
    load_logging(&"info".to_string());

    let ws = WsConnect::new(std::env::var("CHAIN_WSS_URL").unwrap());
    let provider = ProviderBuilder::new().on_ws(ws).await?;

    // Subscribe to new blocks.
    let sub = provider.subscribe_blocks().await?;

    // Wait and take the next 4 blocks.
    let mut stream = sub.into_stream().take(4);

    println!("Awaiting blocks...");

    // Take the stream and print the block number upon receiving a new block.
    let handle = tokio::spawn(async move {
        while let Some(block) = stream.next().await {
            if block.transactions.len() == 0 {
                tracing::warn!("continued because of empty blocks");
                continue;
            }
            let start_time = Instant::now();
            let exit_status = prover.prove(block.header.number);
            if !exit_status.success() {
                tracing::error!("proof generation for block {} failed.", block.header.number);
            }
            let elapsed_time = start_time.elapsed();
            tracing::info!(
                "Block proving: Block: {} Total proving time: {:?}secs",
                block.header.number,
                elapsed_time.as_secs()
            );
            prover.poster.send_proof_to_aggregator(block.header.number).await;
        }
    });

    handle.await?;

    Ok(())
}

pub fn initialize_prover() -> Prover {
    let aggregator_url = std::env::var("AGGREGATOR_URL").unwrap(); // expect
    let rpc_url = std::env::var("CHAIN_RPC_URL").unwrap();
    let identifier = std::env::var("IDENTIFIER").unwrap();
    let last_proved_block = std::env::var("LAST_BLOCK_PROVED").unwrap();
    let proof_path = std::env::var("PROOF_PATH").unwrap();
    let chain_id = std::env::var("CHAIN_ID").unwrap();
    let last_proved_block: u64 = last_proved_block.parse().unwrap();

    Prover::new(
        last_proved_block,
        proof_path,
        rpc_url,
        identifier,
        aggregator_url,
        chain_id,
    )
}

pub fn load_logging(level: &String) {
    let log_level = match level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .init();
}
