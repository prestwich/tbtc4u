// mod deposit;
mod new_deposit;

use lazy_static::lazy_static;
use std::{sync::Arc, time::Duration};
use tokio::time;

use ethers::{
    providers::{JsonRpcClient, Provider, ProviderError, Ws},
    signers::{Client, Wallet},
};
use ethers_core::{
    abi::Abi,
    types::{Address, Filter},
};

use rmn_btc_provider::{esplora::EsploraProvider, PollingBTCProvider};

use ethers_contract::abigen;

static DEFAULT_POLL_INTERVAL_SECS: u64 = 15;

pub(crate) fn default_duration() -> Duration {
    Duration::from_secs(DEFAULT_POLL_INTERVAL_SECS)
}

/// Infura websocket address
static INFURA: &str = "wss://ropsten.infura.io/ws/v3/c60b0bb42f8a4c6481ecd229eddaca27";

/// Ropsten contract addresses
static DEPOSIT_FACTORY: &str = "5536a33ed2d7e055f7f380a78ae9187a3b1d8f75";
static TBTC_SYSTEM: &str = "14dc06f762e7f4a756825c1a1da569b3180153cb";
static WETH: &str = "0a180a76e4466bf68a7f86fb029bed3cccfaaac5";

abigen!(Weth, "abi/weth.json");
abigen!(DepositLog, "abi/depositLog.json");
abigen!(Deposit, "abi/deposit.json");

lazy_static! {
    static ref APP: App = Default::default();
}

#[derive(Default)]
struct App {
    already_tracked: futures_util::lock::Mutex<std::collections::HashSet<Address>>,
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

        time::delay_for(default_duration()).await;
    }
}

/// Make a new deposit state machine
async fn watch_deposit<'a, P: JsonRpcClient>(
    logger: Arc<DepositLog<P, Wallet>>,
    address: Address,
    client: Arc<Client<P, Wallet>>,
    bitcoin: Arc<Box<dyn PollingBTCProvider>>,
) -> bool {
    {
        let mut already_tracked = APP.already_tracked.lock().await;
        if already_tracked.contains(&address) {
            return false;
        }
        already_tracked.insert(address);
    }
    let contract = Deposit::new(address, client);
    crate::new_deposit::check(logger.as_ref(), &contract, bitcoin.as_ref().as_ref()).await.is_ok()
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let btc: Arc<Box<dyn PollingBTCProvider>> = Arc::new(Box::new(EsploraProvider::default()));

    let ws = Ws::connect(INFURA).await.unwrap();
    let eth = Provider::new(ws);

    // This is a privkey
    let signer: Wallet = "380eb0f3d505f087e438eca80bc4df9a7faa24f868e69fc0440261a0fc0567dc"
        .parse()
        .unwrap();

    let client = Arc::new(Client::new(eth, signer));

    let deposit_log = Arc::new(DepositLog::new(TBTC_SYSTEM.parse::<Address>().unwrap(), client.clone()));

    // pass in actual addresses like this
    tokio::spawn(watch_deposit(
        deposit_log.clone(),
        WETH.parse().unwrap(),
        client.clone(),
        btc.clone(),
    ));

    // TODO:
    // Poll deposit_log events.
    // Pass each event's new deposit address to watch_deposit

    // let a = watcher(eth.clone(), &WETH_ABI, "Transfer", WETH);
    // let b = watcher(eth.clone(), &WETH_ABI, "Approval", WETH);
    // let c = watcher(eth, &WETH_ABI, "Deposit", WETH);
    // tokio::spawn(a);
    // tokio::spawn(b);
    // c.await; // never returns

    /*
    let created = watcher(eth.clone(), &DEPOSITLOG_ABI, "Created", TBTC_SYSTEM);
    let registered = watcher(eth.clone(), &DEPOSITLOG_ABI, "RegisteredPubkey", TBTC_SYSTEM);
    let redemption_signature = watcher(eth.clone(), &DEPOSITLOG_ABI, "GotRedemptionSignature", DEPOSIT_FACTORY);
    let setup_failed = watcher(eth.clone(), &DEPOSITLOG_ABI, "SetupFailed", DEPOSIT_FACTORY);
    tokio::spawn(created);
    tokio::spawn(registered);
    tokio::spawn(redemption_signature);
    setup_failed.await;
    */

    Ok(())
}
