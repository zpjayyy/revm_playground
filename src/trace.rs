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
use ethers_providers::{Provider, Ws};
use log::info;

pub async fn mempool_watching(target_address: String) -> Result<()> {
    let wss_url = std::env::var("wss_url").unwrap();
    let provider = Provider::<Ws>::connect(wss_url).await?;
    let provider = Arc::new(provider);

    let checkpoint_path = ".cfmms-checkpoint.json";
    let checkpoint_exists = Path::new(checkpoint_path).exists();

    let pools = DashMap::new();
    let dexes_data = [(
        // UNISWAP V3
        "0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD",
        DexVariant::UniswapV3,
        12369621u64
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
    Ok(())
}