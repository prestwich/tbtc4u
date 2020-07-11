use std::fs;
use serde_json;
use ethers::providers::{Provider, Ws};
use ethers_core::{abi::Abi, types::Filter};
use futures_util::stream::StreamExt;
use futures_util::join;

/// Infura websocket address
static INFURA: &str = "wss://ropsten.infura.io/ws/v3/c60b0bb42f8a4c6481ecd229eddaca27";
/// Ropsten deploy address
static DEPLOY_ADDRESS: &str = "5536a33Ed2D7e055F7F380a78Ae9187A3b1d8f75";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    async fn watcher(event: &str) -> Result<(), Box<dyn std::error::Error>> {
        let ws = Ws::connect(INFURA).await.unwrap();
        let provider = Provider::new(ws);

        let json = fs::read_to_string("depositLog.json")?;
        let abi: Abi = serde_json::from_str(&json)?;
        let event = abi.event(event)?;

        let filter = Filter::new()
            .address_str(DEPLOY_ADDRESS)
            .unwrap()
            .topic0(event.signature());

        let mut stream = provider.watch(&filter).await?;
        println!("Watching {:?}", event.name);

        while let Some(item) = stream.next().await {
            println!("{:?}", item);
        }

        Ok(())
    }

    let created = watcher("Created");
    let registered = watcher("RegisteredPubkey");
    let redemption_signature = watcher("GotRedemptionSignature");

    let (_, _, _) = join!(created, registered, redemption_signature);

    Ok(())
}
