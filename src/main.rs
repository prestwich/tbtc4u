use std::fs;
use serde_json;
use ethers::providers::{Provider, Ws};
use ethers_core::{abi::{Abi, Detokenize, Token, InvalidOutputType}, types::{Address, U256}};
use ethers_contract::Contract;
use ethers_signers::Wallet;

#[derive(Clone, Debug)]
struct RegisteredPubkey {
    deposit_contract_address: Address,
    signing_group_pubkey_x: Vec<u8>,
    signing_group_pubkey_y: Vec<u8>,
    timestamp: U256
}

impl Detokenize for RegisteredPubkey {
    fn from_tokens(tokens: Vec<Token>) -> Result<RegisteredPubkey, InvalidOutputType> {
        let deposit_contract_address = tokens[0].clone().to_address().unwrap();
        let signing_group_pubkey_x = tokens[1].clone().to_fixed_bytes().unwrap();
        let signing_group_pubkey_y = tokens[2].clone().to_fixed_bytes().unwrap();
        let timestamp = tokens[3].clone().to_uint().unwrap();

        Ok(Self {
            deposit_contract_address,
            signing_group_pubkey_x,
            signing_group_pubkey_y,
            timestamp
        })
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {

    async fn foo() -> Result<(), Box<dyn std::error::Error>> {
        let url = "wss://rinkeby.infura.io/ws/v3/c60b0bb42f8a4c6481ecd229eddaca27";
        let ws = Ws::connect(url).await.unwrap();
        let provider = Provider::new(ws);

        let json = fs::read_to_string("depositLog.json")?;
        let abi: Abi = serde_json::from_str(&json)?;

        let address = "5536a33Ed2D7e055F7F380a78Ae9187A3b1d8f75".parse::<Address>()?;

        // what is the hex here? private key?
        let client = "380eb0f3d505f087e438eca80bc4df9a7faa24f868e69fc0440261a0fc0567dc".parse::<Wallet>()?.connect(provider);

        let contract = Contract::new(address, abi, client);

        let logs: Vec<RegisteredPubkey> = contract
            .event("RegisteredPubkey")?
            .from_block(50000u64)
            .query()
            .await?;

        // make a stream
        println!("Logs: {:?}", logs);

        Ok(())
    }

    let _result = foo().await;

    Ok(())
}
