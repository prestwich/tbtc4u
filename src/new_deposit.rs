use tokio::time;

use ethers::{contract::ContractError, providers::JsonRpcClient, signers::Wallet};

use crate::{default_duration, Deposit, DepositLog};
use rmn_btc::prelude::*;
use rmn_btc_provider::PollingBTCProvider;

macro_rules! check_state {
    ($deposit:expr) => {{
        let s = state($deposit).await?;
        if s > 2 {
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

pub fn script(x: [u8; 32], y: [u8; 32]) -> rmn_btc::types::ScriptPubkey {
    let mut pubkey = Vec::with_capacity(35);
    pubkey.push(0);
    pubkey.push(0x14);
    pubkey.push((y[31] & 1) + 2);
    pubkey.extend(&x);
    rmn_btc::types::ScriptPubkey::from(pubkey.to_vec())
}

pub fn flatten_txids(nodes: &Vec<TXID>) -> Vec<u8> {
    let mut n = vec![];
    for node in nodes {
        n.extend(&node.internal());
    }
    n
}

pub fn flatten_headers(headers: &Vec<BlockHash>) -> Vec<u8> {
    let mut h = vec![];
    for header in headers {
        h.extend(&header.internal());
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

    loop {
        if let Some(confs) = bitcoin.get_confs(txid).await? {
            if confs > 10 {
                break;
            }
        }
        time::delay_for(default_duration()).await;
    }

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
            flatten_txids(&nodes),
            pos.into(),
            flatten_headers(&headers),
        )
        .send()
        .await;

    println!("result! {:?} {:?}", deposit.address(), &result);
    Ok(result.is_ok())
}
