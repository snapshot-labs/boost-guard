use crate::ServerError;
use ethers::signers::LocalWallet;
use ethers::types::transaction::eip712::TypedData;
use ethers::types::{transaction::eip712::Eip712, Address, U256};
use ethers::types::{Bytes, Signature};
use ethers::utils::hex::FromHex;

#[derive(Debug, Clone)]
pub struct ClaimConfig {
    // The boost id where the claim is being made
    boost_id: U256, // todo: check this is copied to CamelCase
    chain_id: U256,
    // The address of the recipient for the claim
    recipient: Address, // address
    // The amount of boost token in the claim
    amount: U256, // uint256
    // A reference string for the claim
    ref_: Bytes,
}

const BOOST_NAME: &str = "boost";
const BOOST_VERSION: &str = "1";
const VERIFYING_CONTRACT: &str = "0xe370E89f87fA67e3c18d8F34c40EA962b8feDB5D"; // singleton deployment ?

impl ClaimConfig {
    pub fn new(
        boost_id: &str,
        chain_id: &str,
        recipient: &str,
        amount: u128,
    ) -> Result<Self, ServerError> {
        Ok(Self {
            boost_id: U256::from_str_radix(boost_id, 16)?, // todo: decide on hex or decimal
            chain_id: U256::from_str_radix(chain_id, 16)?,
            recipient: recipient.parse()?,
            amount: U256::from(amount),
            ref_: Bytes::from_hex(
                "0x0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
        })
    }

    pub fn create_signature(&self, signer: &LocalWallet) -> Result<Signature, ServerError> {
        let json = serde_json::json!( {
          "types": {
            "EIP712Domain": [
              {
                "name": "name",
                "type": "string"
              },
              {
                "name": "version",
                "type": "string"
              },
              {
                "name": "chainId",
                "type": "uint256"
              },
              {
                "name": "verifyingContract",
                "type": "address"
              }
            ],
            "Claim": [
              {
                  "name": "boostId",
                  "type": "uint256"
              },
              {
                  "name": "recipient",
                  "type": "address"
              },
              {
                  "name": "amount",
                  "type": "uint256"
              },
              {
                  "name": "ref",
                  "type": "bytes32"
              }
            ]
          },
          "primaryType": "Claim",
          "domain": {
            "name": format!("{BOOST_NAME:}"),
            "version": format!("{BOOST_VERSION:}"),
            "chainId": format!("{:}", self.chain_id),
            "verifyingContract": format!("{VERIFYING_CONTRACT:}"),
          },
          "message": {
            "boostId": format!("{}", self.boost_id),
            "recipient": format!("{:?}", self.recipient),
            "amount": format!("{}", self.amount),
            "ref": format!("{}", self.ref_),
          }
        });
        let typed_data: TypedData = serde_json::from_value(json).expect("invalid json");
        let hash = typed_data.encode_eip712().expect("failed to encode eip712");
        signer
            .sign_hash(hash.into())
            .map_err(|e| ServerError::ErrorString(e.to_string()))
    }
}

// todo: add tests
