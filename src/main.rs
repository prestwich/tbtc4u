use std::fs;
use serde_json;
use ethers::providers::{Provider, Ws};
use ethers_core::{abi::{Abi, Detokenize, Token, InvalidOutputType}, types::{Address, U256, Filter}};
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

        let addr = "5536a33Ed2D7e055F7F380a78Ae9187A3b1d8f75";

        let filter = Filter::new()
            .address_str(addr)
            .unwrap()
            .event("ValueChanged(address,string,string)");
            //.topic1(t1)

        let stream = provider.watch(&filter).await?;

        println!("{}", stream);

        while let Some(item) = stream.next().await {
            println!("{}", item);
        }

        Ok(())
    }

    let _result = foo().await;

    Ok(())
}
