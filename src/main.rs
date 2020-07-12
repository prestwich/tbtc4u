use ethers::providers::{FilterStream, JsonRpcClient, Provider, ProviderError, Ws};
use ethers_core::{abi::Abi, types::Filter};
use futures_util::stream::StreamExt;
use lazy_static::lazy_static;
use std::{fs, sync::Arc};

/// Infura websocket address
static INFURA: &str = "wss://ropsten.infura.io/ws/v3/c60b0bb42f8a4c6481ecd229eddaca27";

/// Ropsten contract addresses
static DEPOSIT_FACTORY: &str = "5536a33ed2d7e055f7f380a78ae9187a3b1d8f75";
static TBTC_SYSTEM: &str = "14dc06f762e7f4a756825c1a1da569b3180153cb";
static WETH: &str = "0a180a76e4466bf68a7f86fb029bed3cccfaaac5";

lazy_static! {
    static ref ABI: Abi = {
        let json = fs::read_to_string("depositLog.json").unwrap();
        serde_json::from_str(&json).unwrap()
    };
    static ref WETH_ABI: Abi = {
        let json = fs::read_to_string("weth.json").unwrap();
        serde_json::from_str(&json).unwrap()
    };
}

// infinite loop printing events
async fn watcher<P: JsonRpcClient>(
    provider: Arc<Provider<P>>,
    abi: &Abi,
    event: &str,
    address: &str,
) -> Result<(), ProviderError> {
    let event = abi.event(event).unwrap();
    let signature = event.signature();

    println!("Event: {:?}", event.name);
    println!("Topic: {:?}", signature);

    let filter = Filter::new()
        .address_str(address)
        .unwrap()
        .topic0(signature);

    dbg!(serde_json::to_string(&filter).unwrap());

    let mut stream = provider.watch(&filter).await?;
    dbg!(&stream.id());
    while let Some(item) = stream.next().await {
        println!("{:?}", item);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let ws = Ws::connect(INFURA).await.unwrap();
    let provider = Arc::new(Provider::new(ws));

    let a = watcher(provider.clone(), &WETH_ABI, "Transfer", WETH);
    let b = watcher(provider.clone(), &WETH_ABI, "Approval", WETH);
    let c = watcher(provider, &WETH_ABI, "Deposit", WETH);
    tokio::spawn(a);
    tokio::spawn(b);
    c.await;

    // let created = watcher(&provider, &ABI, "Created", TBTC_SYSTEM);
    // let registered = watcher(&provider, &ABI, "RegisteredPubkey", TBTC_SYSTEM);
    // let redemption_signature = watcher(&provider, &ABI, "GotRedemptionSignature", DEPOSIT_FACTORY);
    // let setup_failed = watcher(&provider, &ABI, "SetupFailed", DEPOSIT_FACTORY);
    // let (_, _, _, _) = join!(created, registered, redemption_signature, setup_failed);

    Ok(())
}
