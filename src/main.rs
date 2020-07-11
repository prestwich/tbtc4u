use std::fs;
use lazy_static::lazy_static;
use ethers::providers::{JsonRpcClient, Provider, Ws};
use ethers_core::{abi::Abi, types::Filter};
use futures_util::join;
use tokio::time;

/// Infura websocket address
static INFURA: &str = "wss://ropsten.infura.io/ws/v3/c60b0bb42f8a4c6481ecd229eddaca27";

/// Ropsten contract addresses
static DEPOSIT_FACTORY: &str = "5536a33ed2d7e055f7f380a78ae9187a3b1d8f75";
static TBTC_SYSTEM: &str = "14dc06f762e7f4a756825c1a1da569b3180153cb";

lazy_static! {
    static ref ABI: Abi = {
        let json = fs::read_to_string("depositLog.json").unwrap();
        serde_json::from_str(&json).unwrap()
    };
}

// infinite loop printing events
async fn watcher<P: JsonRpcClient>(provider: &Provider<P>, event: &str, address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let event = ABI.event(event)?;
    let signature = event.signature();

    println!("Event: {:?}", event.name);
    println!("Topic: {:?}", signature);

    let filter = Filter::new()
        .address_str(address)
        .unwrap()
        .topic0(signature);

    loop {
        let logs = provider.get_logs(&filter).await?;

        for log in logs {
            println!("{:?}", log);
        }

        time::delay_for(std::time::Duration::new(5, 0)).await;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let ws = Ws::connect(INFURA).await.unwrap();
    let provider = Provider::new(ws);

    let created = watcher(&provider, "Created", TBTC_SYSTEM);
    let registered = watcher(&provider, "RegisteredPubkey", TBTC_SYSTEM);
    let redemption_signature = watcher(&provider, "GotRedemptionSignature", DEPOSIT_FACTORY);
    let setup_failed = watcher(&provider, "SetupFailed", DEPOSIT_FACTORY);

    let (_, _, _, _) = join!(created, registered, redemption_signature, setup_failed);

    Ok(())
}
