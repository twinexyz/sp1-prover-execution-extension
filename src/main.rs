mod utils;
use futures_util::StreamExt;
use reth::api::FullNodeComponents;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_ethereum::EthereumNode;
use reth_tracing::tracing::info;
use std::{
    process::Command,
    sync::{Arc, Mutex},
    time::Instant,
};
mod poster;
use poster::poster::post_to_l1;
use dotenv::dotenv;

async fn my_exex<Node: FullNodeComponents>(
    mut ctx: ExExContext<Node>,
    cmd_mut: Arc<Mutex<i32>>,
) -> eyre::Result<()> {
    while let Some(notification) = ctx.notifications.next().await {
        match &notification {
            ExExNotification::ChainCommitted { new } => {
                let blocks = new.blocks_iter();
                {
                    let mut mut_guard = cmd_mut.lock().unwrap();
                    for block in blocks {
                        let start_time = Instant::now();
                        let output = Command::new("rsp")
                            .args([
                                "--block-number",
                                &format!("{}", block.block.number),
                                "--rpc-url",
                                "http://localhost:8545/",
                                "--chain-id",
                                "1337",
                                "--prove",
                            ])
                            .output()
                            .expect("failed to run the process");
                        println!("************************************************************************");
                        println!("*******************************exit status******************************: {}", output.status);
                        println!("************************************************************************");
                        let elapsed_time = start_time.elapsed();
                        println!(
                            "***********************Total proving time: {:?}secs",
                            elapsed_time.as_secs()
                        )
                    }
                    *mut_guard += 1;
                }
                // take input from the file 
                post_to_l1().await;
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
    let cmd_mut = Arc::new(Mutex::new(0));
    reth::cli::Cli::parse_args().run(|builder, _| async move {
        let handle = builder
            .node(EthereumNode::default())
            .install_exex("my-exex", |ctx| async move {
                println!("installing exex");
                Ok(my_exex(ctx, cmd_mut))
            })
            .launch()
            .await?;

        handle.wait_for_node_exit().await
    })
}
