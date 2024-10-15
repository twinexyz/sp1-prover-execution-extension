mod utils;
use futures_util::StreamExt;
use reth::api::FullNodeComponents;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_ethereum::EthereumNode;
use reth_tracing::tracing::info;
use serde::{Deserialize, Serialize};
use std::{
    fs::read_to_string,
    process::ExitStatus,
    sync::{mpsc, Arc, Mutex},
    time::Instant,
    u64,
};
use yaml_config::load;
mod poster;
mod prover;
use clap::Parser;
use dotenv::dotenv;
use poster::poster::post_to_l1;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Config {
    prover: prover::prover::Prover,
}

async fn my_exex<Node: FullNodeComponents>(
    mut ctx: ExExContext<Node>,
    cmd_mut: Arc<Mutex<i32>>,
    config: Config,
) -> eyre::Result<()> {
    while let Some(notification) = ctx.notifications.next().await {
        match &notification {
            ExExNotification::ChainCommitted { new } => {
                let blocks = new.blocks_iter();
                let (tx, rx) = mpsc::channel::<u64>();
                {
                    let mut mut_guard = cmd_mut.lock().unwrap();
                    for block in blocks {
                        _ = block;
                        if block.transaction_root_is_empty() {
                            println!("always continue?????");
                            tx.send(0).unwrap();
                            continue;
                        }
                        let start_time = Instant::now();
                        let exit_status = config.prover.prove(block.block.number);
                        if !exit_status.success() {
                            println!("proof generation failed.")
                        }
                        let elapsed_time = start_time.elapsed();
                        println!(
                            "***********************Total proving time: {:?}secs",
                            elapsed_time.as_secs()
                        );
                        tx.send(block.block.number).unwrap();
                    }
                    *mut_guard += 1;
                }
                tx.send(u64::MAX).unwrap();
                post_to_l1(rx).await;
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
    let args = Args::parse();

    let config_file = read_to_string(args.config).unwrap();
    let config: Config = serde_yaml::from_str(&config_file).unwrap();

    let cmd_mut = Arc::new(Mutex::new(0));
    reth::cli::Cli::parse_args().run(|builder, _| async move {
        let handle = builder
            .node(EthereumNode::default())
            .install_exex("my-exex", |ctx| async move {
                println!("installing exex");
                Ok(my_exex(ctx, cmd_mut, config))
            })
            .launch()
            .await?;

        handle.wait_for_node_exit().await
    })
}
