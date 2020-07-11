use std::fs;
use serde_json;
use ethers::providers::{Provider, Ws};
use ethers_core::{abi::Abi, types::Filter};
use futures_util::stream::StreamExt;
use futures_util::join;

/// Infura websocket address
static INFURA: &str = "wss://ropsten.infura.io/ws/v3/c60b0bb42f8a4c6481ecd229eddaca27";

/// Ropsten contract addresses
static DEPOSIT_FACTORY: &str = "5536a33ed2d7e055f7f380a78ae9187a3b1d8f75";
static TBTC_SYSTEM: &str = "14dc06f762e7f4a756825c1a1da569b3180153cb";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    async fn watcher(event: &str, address: &str) -> Result<(), Box<dyn std::error::Error>> {
        let ws = Ws::connect(INFURA).await.unwrap();
        let provider = Provider::new(ws);

        let json = fs::read_to_string("depositLog.json")?;
        let abi: Abi = serde_json::from_str(&json)?;
        let event = abi.event(event)?;
        let signature = event.signature();

        println!("Event: {:?}", event.name);
        println!("Topic: {:?}", signature);

        let filter = Filter::new()
            .address_str(address)
            .unwrap()
            .topic0(signature);

        let mut stream = provider.watch(&filter).await?;

        while let Some(item) = stream.next().await {
            println!("{:?}", item);
        }

        Ok(())
    }

    let created = watcher("Created", TBTC_SYSTEM);
    let registered = watcher("RegisteredPubkey", TBTC_SYSTEM);

    let redemption_signature = watcher("GotRedemptionSignature", DEPOSIT_FACTORY);
    let setup_failed = watcher("SetupFailed", DEPOSIT_FACTORY);

    let (_, _, _, _) = join!(created, registered, redemption_signature, setup_failed);

    Ok(())
}
