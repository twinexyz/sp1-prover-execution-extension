mod utils;
use dotenv::dotenv;
use futures_util::StreamExt;
use prover::prover::Prover;
use reth::api::FullNodeComponents;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_ethereum::EthereumNode;
use reth_tracing::tracing::info;
use std::{
    sync::{mpsc, Arc, Mutex},
    time::Instant,
    u64,
};
mod poster;
mod prover;

async fn my_exex<Node: FullNodeComponents>(
    mut ctx: ExExContext<Node>,
    cmd_mut: Arc<Mutex<i32>>,
    prover: Prover,
) -> eyre::Result<()> {
    while let Some(notification) = ctx.notifications.next().await {
        match &notification {
            ExExNotification::ChainCommitted { new } => {
                let blocks = new.blocks_iter();
                let (tx, rx) = mpsc::channel::<u64>();
                {
                    let mut mut_guard = cmd_mut.lock().unwrap();
                    for block in blocks {
                        // if block.transaction_root_is_empty() {
                        //     println!("always continue?????");
                        //     tx.send(0).unwrap();
                        //     continue;
                        // }
                        let start_time = Instant::now();
                        // let exit_status = prover.prove(block.block.number);
                        // if !exit_status.success() {
                        //     println!("proof generation failed.")
                        // }
                        let elapsed_time = start_time.elapsed();
                        println!(
                            "***********************Total proving time: {:?}secs",
                            elapsed_time.as_secs()
                        );
                        tx.send(1101).unwrap();
                    }
                    *mut_guard += 1;
                }
                tx.send(u64::MAX).unwrap();
                prover.poster.send_proof_to_aggregator(rx).await;
            }
            ExExNotification::ChainReorged { old, new } => {
                info!(from_chain = ?old.range(), to_chain = ?new.range(), "Received reorg");
            }
            ExExNotification::ChainReverted { old } => {
                info!(reverted_chain = ?old.range(), "Received revert");
            }
        };

        if let Some(committed_chain) = notification.committed_chain() {
            ctx.events
                .send(ExExEvent::FinishedHeight(committed_chain.tip().number))?;
        }
    }

    Ok(())
}

fn main() -> eyre::Result<()> {
    dotenv().ok();

    let aggregator_url = std::env::var("AGGREGATOR_URL").unwrap();
    let rpc_url = std::env::var("CHAIN_RPC_URL").unwrap();
    let identifier = std::env::var("IDENTIFIER").unwrap();
    let last_proved_block = std::env::var("LAST_BLOCK_PROVED").unwrap();
    let proof_path = std::env::var("PROOF_PATH").unwrap();
    let chain_id = std::env::var("CHAIN_ID").unwrap();
    let last_proved_block: u64 = last_proved_block.parse().unwrap();

    let prover = Prover::new(
        last_proved_block,
        proof_path,
        rpc_url,
        identifier,
        aggregator_url,
        chain_id
    );

    let cmd_mut = Arc::new(Mutex::new(0));
    reth::cli::Cli::parse_args().run(|builder, _| async move {
        let handle = builder
            .node(EthereumNode::default())
            .install_exex("my-exex", |ctx| async move {
                println!("installing exex");
                Ok(my_exex(ctx, cmd_mut, prover))
            })
            .launch()
            .await?;

        handle.wait_for_node_exit().await
    })
}
