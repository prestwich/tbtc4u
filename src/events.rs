use ethers_core::{
    abi::{Abi, Detokenize, Token, InvalidOutputType},
    types::{Address, U256, Filter}
};

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

#[derive(Clone, Debug)]
struct Created {
    deposit_contract_address: Address,
    keep_address: Address,
    timestamp: U256
}

impl Detokenize for Created {
    fn from_tokens(tokens: Vec<Token>) -> Result<Created, InvalidOutputType> {
        let deposit_contract_address = tokens[0].clone().to_address().unwrap();
        let keep_address = tokens[1].clone().to_address().unwrap();
        let timestamp = tokens[2].clone().to_uint().unwrap();

        Ok(Self {
            deposit_contract_address,
            keep_address,
            timestamp
        })
    }
}

#[derive(Clone, Debug)]
struct GotRedemptionSignature {
    deposit_contract_address: Address,
    digest: Vec<u8>,
    r: Vec<u8>,
    s: Vec<u8>,
    timestamp: U256
}

impl Detokenize for GotRedemptionSignature {
    fn from_tokens(tokens: Vec<Token>) -> Result<GotRedemptionSignature, InvalidOutputType> {
        let deposit_contract_address = tokens[0].clone().to_address().unwrap();
        let digest = tokens[1].clone().to_fixed_bytes().unwrap();
        let r = tokens[2].clone().to_fixed_bytes().unwrap();
        let s = tokens[3].clone().to_fixed_bytes().unwrap();
        let timestamp = tokens[4].clone().to_uint().unwrap();

        Ok(Self {
            deposit_contract_address,
            digest,
            r,
            s,
            timestamp
        })
    }
}

