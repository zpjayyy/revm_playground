use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use anyhow::Result;
use anyhow::Ok;
use cfmms::checkpoint::sync_pools_from_checkpoint;
use cfmms::dex::{Dex, DexVariant};
use cfmms::sync::sync_pairs;
use dashmap::DashMap;
use ethers::prelude::H160;
use ethers::types::U256;
use ethers::types::U64;
use ethers::types::Transaction;
use ethers_providers::{Middleware, Provider, Ws};
use log::info;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::SendError;
use tokio::sync::broadcast::Sender;
use tokio::task::JoinSet;
use tokio_stream::StreamExt;

pub async fn mempool_watching(target_address: String) -> Result<()> {
    let wss_url = std::env::var("WSS_URL").unwrap();
    let provider = Provider::<Ws>::connect(wss_url).await?;
    let provider = Arc::new(provider);

    let checkpoint_path = ".cfmms-checkpoint.json";
    let checkpoint_exists = Path::new(checkpoint_path).exists();

    let pools = DashMap::new();
    let dexes_data = [(
        // UNISWAP V3
        "0x1F98431c8aD98523631AE4a59f267346ea31F984",
        DexVariant::UniswapV3,
        4734394u64
    )];

    let dexes: Vec<_> = dexes_data
        .into_iter()
        .map(|(address, variant, number)| {
            Dex::new(H160::from_str(address).unwrap(), variant, number, Some(300))
        })
        .collect();

    let pools_vec = if checkpoint_exists {
        let (_, pools_vec) = sync_pools_from_checkpoint(checkpoint_path, 100000, provider.clone()).await?;
        pools_vec
    } else {
        sync_pairs(dexes.clone(), provider.clone(), Some(checkpoint_path)).await?
    };

    for pool in pools_vec {
        pools.insert(pool.address(), pool);
    }
    info!("Uniswap v3 pool synced {}", pools.len());

    let (event_sender, _): (Sender<Event>, _) = broadcast::channel(512);
    let mut set = JoinSet::new();

    {
        let provider = provider.clone();
        let event_sender = event_sender.clone();
        set.spawn(async move {
            let stream = provider.subscribe_blocks().await.unwrap();
            let mut stream = stream.filter_map(|block| match block.number {
                None => None,
                Some(number) => Some(NewBlock {
                    number,
                    gas_used: block.gas_used,
                    gas_limit: block.gas_limit,
                    base_fee_per_price: block.base_fee_per_gas.unwrap_or_default(),
                    timestamp: block.timestamp,
                })
            });

            while let Some(block) = stream.next().await {
                match event_sender.send(Event::NewBlock(block)) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        });
    }

    {
        let provider = provider.clone();
        let event_sender = event_sender.clone();

        set.spawn(async move {
            let stream = provider.subscribe_pending_txs().await.unwrap();
            let mut stream = stream.transactions_unordered(256).fuse();

            while let Some(result) = stream.next().await {
                match result {
                    Ok(tx) => match event_sender.send(Event::Transaction(tx)) {
                        Result::Ok(_) => {}
                        Err(_) => {}
                    }
                    Err(_) => {}
                }
            }
        });
    }
    Ok(())
}

#[derive(Default, Debug, Clone)]
pub struct NewBlock {
    pub number: U64,
    pub gas_used: U256,
    pub gas_limit: U256,
    pub base_fee_per_price: U256,
    pub timestamp: U256,
}

pub enum Event {
    NewBlock(NewBlock),
    Transaction(Transaction),
}