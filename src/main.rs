// mod deposit;
mod new_deposit;

use lazy_static::lazy_static;
use std::{sync::Arc, time::Duration};
use tokio::time;

use ethers::{
    providers::{JsonRpcClient, Provider, Ws},
    signers::{Client, Wallet},
};
use ethers_core::types::Address;

use bitcoins_provider::{esplora::EsploraProvider, provider::{CachingProvider, PollingBTCProvider}};

use ethers_contract::abigen;

static DEFAULT_POLL_INTERVAL_SECS: u64 = 10;

pub(crate) fn default_duration() -> Duration {
    Duration::from_secs(DEFAULT_POLL_INTERVAL_SECS)
}

/// Infura websocket address
static INFURA: &str = "wss://ropsten.infura.io/ws/v3/c60b0bb42f8a4c6481ecd229eddaca27";

/// Ropsten contract addresses
// static DEPOSIT_FACTORY: &str = "5536a33ed2d7e055f7f380a78ae9187a3b1d8f75";
static TBTC_SYSTEM: &str = "14dc06f762e7f4a756825c1a1da569b3180153cb";
// static WETH: &str = "0a180a76e4466bf68a7f86fb029bed3cccfaaac5";

abigen!(Weth, "abi/weth.json");
abigen!(DepositLog, "abi/depositLog.json");

#[allow(clippy::too_many_arguments)]
mod gen_deposit {
    use super::*;
    abigen!(Deposit, "abi/deposit.json");
}

use gen_deposit::*;

lazy_static! {
    static ref APP: App = Default::default();
}

#[derive(Default)]
struct App {
    already_tracked: futures_util::lock::Mutex<std::collections::HashSet<Address>>,
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
    crate::new_deposit::check(logger.as_ref(), &contract, bitcoin.as_ref().as_ref())
        .await
        .is_ok()
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let btc: Arc<Box<dyn PollingBTCProvider>> = Arc::new(Box::new(CachingProvider::from(EsploraProvider::default())));

    let ws = Ws::connect(INFURA).await.unwrap();
    let eth = Provider::new(ws);

    // This is a privkey. But it belongs to Georgios
    let signer: Wallet = "380eb0f3d505f087e438eca80bc4df9a7faa24f868e69fc0440261a0fc0567dc"
        .parse()
        .unwrap();

    let client = Arc::new(Client::new(eth, signer));

    let deposit_log = Arc::new(DepositLog::new(
        TBTC_SYSTEM.parse::<Address>().unwrap(),
        client.clone(),
    ));

    println!("Setting up chain watcher");
    // set up watcher loop
    let mut last: u64 = client.get_block_number().await.unwrap().low_u64() + 1;
    println!("Most recent block is {:?}", last);

    loop {
        time::delay_for(default_duration()).await;
        let current = client.get_block_number().await.unwrap().low_u64();
        if current == last {
            continue;
        }

        if let Ok(block) = client.get_block(current).await {
            last += 1;
            println!("Got new block {:?}", block.hash);
            let logs: Vec<CreatedFilter> = deposit_log.created_filter().query().await.unwrap();
            println!("Got {:?} logs in the newest block", logs.len());
            for log in logs.iter() {
                tokio::spawn(watch_deposit(
                    deposit_log.clone(),
                    log.deposit_contract_address,
                    client.clone(),
                    btc.clone(),
                ));
            }
        }
    }
}
