use std::path::PathBuf;
use tokio::sync::mpsc;

//use configs::{init_logging, Opts, SubCommand};
use near_indexer;
use tracing::info;
use tracing_subscriber::EnvFilter;

use near_chain_configs::Genesis;

fn init_logging() {
    let env_filter = EnvFilter::new(
        "tokio_reactor=info,near=info,near=error,stats=info,telemetry=info,indexer_example=info,indexer=info,near-performance-metrics=info",
    );
    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();
}

async fn listen_blocks(mut stream: mpsc::Receiver<near_indexer::StreamerMessage>) {
    while let Some(streamer_message) = stream.recv().await {
        info!(
            target: "indexer_example",
            "#{} {} Shards: {}, Transactions: {}, Receipts: {}, ExecutionOutcomes: {}",
            streamer_message.block.header.height,
            streamer_message.block.header.hash,
            streamer_message.shards.len(),
            streamer_message.shards.iter().map(|shard| if let Some(chunk) = &shard.chunk { chunk.transactions.len() } else { 0usize }).sum::<usize>(),
            streamer_message.shards.iter().map(|shard| if let Some(chunk) = &shard.chunk { chunk.receipts.len() } else { 0usize }).sum::<usize>(),
            streamer_message.shards.iter().map(|shard| shard.receipt_execution_outcomes.len()).sum::<usize>(),
        );
    }
}

pub struct NearIndexer {
    homedir: PathBuf,
}

impl NearIndexer {
    pub fn new(homedir: PathBuf) -> Self {
        openssl_probe::init_ssl_cert_env_vars();
        init_logging();
        Self { homedir }
    }

    pub fn init(&self, network: String) {
        let buf = std::fs::read_to_string("/tmp/mainnet_genesis.json")
            .expect("failed to read genesis_config");
        let jd = &mut serde_json::Deserializer::from_str(&buf);
        let result: Result<Genesis, _> = serde_path_to_error::deserialize(jd);
        match result {
            Ok(_) => println!("that worked"),
            Err(err) => {
                let path = err.path().to_string();
                println!("error at {}: {}", path, err);
            }
        }

        let config = near_indexer::InitConfigArgs {
            chain_id: Some(network),
            account_id: None,
            test_seed: None,
            num_shards: 1,
            fast: false,
            genesis: None,
            download: true,
            download_genesis_url: None,
        };
        near_indexer::indexer_init_configs(&self.homedir, config.into())
    }

    pub fn run(&self) {
        let indexer_config = near_indexer::IndexerConfig {
            home_dir: self.homedir.clone(),
            sync_mode: near_indexer::SyncModeEnum::FromInterruption,
            await_for_node_synced: near_indexer::AwaitForNodeSyncedEnum::WaitForFullSync,
        };
        let system = actix::System::new();
        system.block_on(async move {
            let indexer = near_indexer::Indexer::new(indexer_config);
            let stream = indexer.streamer();
            actix::spawn(listen_blocks(stream));
        });
        system.run().unwrap();
    }
}

#[test]
fn serde() {
    // This file is a copy of nearcore/res/mainnet_genesis.json
    static BUF: &str = include_str!("mainnet_genesis.json");
    let _genesis: Genesis = serde_json::from_str(BUF).expect("we can parse the mainnet genesis");
}
