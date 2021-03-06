use tokio::time;

use ethers::{contract::ContractError, providers::JsonRpcClient, signers::Wallet};

use crate::{default_duration, Deposit, DepositLog};
use bitcoins::prelude::*;
use bitcoins_provider::provider::PollingBTCProvider;

macro_rules! check_state {
    ($deposit:expr) => {{
        let s = state($deposit).await?;
        if s > 2 {
            println!("Deposit closed okay {}", $deposit.address());
            return Ok(false);
        }
    }};
}

pub(crate) async fn state<P: JsonRpcClient>(
    deposit: &Deposit<P, Wallet>,
) -> Result<u64, ContractError> {
    deposit
        .get_current_state()
        .call()
        .await
        .map(|u| u.low_u64())
}

pub fn script(x: [u8; 32], y: [u8; 32]) -> bitcoins::types::ScriptPubkey {
    let mut pubkey = [0u8; 33];
    pubkey[0] = (y[31] & 1) + 2;
    pubkey[1..].copy_from_slice(&x);
    let mut script = Vec::with_capacity(22);
    script.push(0);
    script.push(0x14);
    script.extend(&bitcoin_spv::btcspv::hash160(&pubkey)[..]);
    bitcoins::types::ScriptPubkey::from(pubkey.to_vec())
}

pub fn flatten<A: AsRef<[u8]>>(nodes: &[A]) -> Vec<u8> {
    let mut n = vec![];
    for node in nodes {
        n.extend(node.as_ref());
    }
    n
}

pub fn flatten_headers(headers: &[RawHeader]) -> Vec<u8> {
    let mut h = vec![];
    for header in headers {
        h.extend(header.as_ref());
    }
    h
}

pub fn write_vin(tx: &BitcoinTx) -> Vec<u8> {
    let mut vin = vec![];
    LegacyTx::write_prefix_vec(&mut vin, &tx.outputs()).unwrap();
    vin
}

pub fn write_vout(tx: &BitcoinTx) -> Vec<u8> {
    let mut vout = vec![];
    LegacyTx::write_prefix_vec(&mut vout, &tx.outputs()).unwrap();
    vout
}

pub(crate) async fn check<P: JsonRpcClient>(
    logger: &DepositLog<P, Wallet>,
    deposit: &Deposit<P, Wallet>,
    bitcoin: &dyn PollingBTCProvider,
) -> Result<bool, Box<dyn std::error::Error>> {
    check_state!(deposit);
    let expected_sats = deposit.lot_size_satoshis().call().await?;

    check_state!(deposit);
    println!("Waiting for pubkey of {}", deposit.address());

    let (pubkey_x, pubkey_y) = loop {
        let registered = logger
            .registered_pubkey_filter()
            .topic1(deposit.address())
            .query()
            .await?;
        // TODO: delay
        if registered.is_empty() {
            time::delay_for(default_duration()).await;
            continue;
        }
        let event = &registered[0];
        break (event.signing_group_pubkey_x, event.signing_group_pubkey_y);
    };

    let script = script(pubkey_x, pubkey_y);
    check_state!(deposit);
    println!(
        "BTC address for {} is {:?}",
        deposit.address(),
        bitcoins::enc::encoder::TestnetEncoder::encode_address(&script)
    );
    println!("Waiting for funding of {}", deposit.address());

    let utxo = loop {
        let utxos = bitcoin.get_utxos_by_script(&script).await?;
        if let Some(utxo) = utxos.into_iter().find(|u| u.value >= expected_sats) {
            break utxo;
        } else {
            time::delay_for(default_duration()).await;
            continue;
        }
    };
    let txid = utxo.outpoint.txid;

    check_state!(deposit);
    println!("Waiting for confs for {}", deposit.address());

    loop {
        if let Some(confs) = bitcoin.get_confs(txid).await? {
            if confs > 10 {
                break;
            }
        }
        time::delay_for(default_duration()).await;
    }

    check_state!(deposit);
    println!("building SPV proof for {}", deposit.address());

    let tx = bitcoin.get_tx(txid).await?.expect("has confs");
    let output_idx = utxo.outpoint.idx;
    let (pos, nodes) = bitcoin.get_merkle(txid).await?.expect("has confs");
    let headers = bitcoin.get_confirming_headers(txid, 10).await?;
    let result = deposit
        .provide_btc_funding_proof(
            tx.version().to_le_bytes(),
            write_vin(&tx),
            write_vout(&tx),
            tx.locktime().to_le_bytes(),
            output_idx as u8,
            flatten(&nodes),
            pos.into(),
            flatten_headers(&headers),
        )
        .send()
        .await;

    println!("result! {} {:?}", deposit.address(), &result);
    Ok(result.is_ok())
}
