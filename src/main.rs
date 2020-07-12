mod deposit;

use lazy_static::lazy_static;
use std::{fs, sync::Arc};
use tokio::time;

use ethers::providers::{JsonRpcClient, Provider, ProviderError, Ws};
use ethers_core::{abi::Abi, types::Filter};

use rmn_btc_provider::{esplora::EsploraProvider, PollingBTCProvider};

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
    provider: Arc<Box<Provider<P>>>,
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

    loop {
        let logs = provider.get_logs(&filter).await?;

        for log in logs {
            println!("{:?}", log);
        }

        time::delay_for(std::time::Duration::new(5, 0)).await;
    }
}

struct App<P: JsonRpcClient> {
    ether: Arc<Box<Provider<P>>>,
    bitcoin: Arc<Box<dyn PollingBTCProvider>>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let ws = Ws::connect(INFURA).await.unwrap();
    let eth = Arc::new(Box::new(Provider::new(ws)));
    let btc: Arc<Box<dyn PollingBTCProvider>> = Arc::new(Box::new(EsploraProvider::default()));

    let app = App {
        ether: eth.clone(),
        bitcoin: btc.clone(),
    };

    let a = watcher(eth.clone(), &WETH_ABI, "Transfer", WETH);
    let b = watcher(eth.clone(), &WETH_ABI, "Approval", WETH);
    let c = watcher(eth, &WETH_ABI, "Deposit", WETH);
    tokio::spawn(a);
    tokio::spawn(b);
    c.await; // never returns

    // let created = watcher(&eth, &ABI, "Created", TBTC_SYSTEM);
    // let registered = watcher(&eth, &ABI, "RegisteredPubkey", TBTC_SYSTEM);
    // let redemption_signature = watcher(&eth, &ABI, "GotRedemptionSignature", DEPOSIT_FACTORY);
    // let setup_failed = watcher(&eth, &ABI, "SetupFailed", DEPOSIT_FACTORY);
    // let (_, _, _, _) = join!(created, registered, redemption_signature, setup_failed);

    Ok(())
}
