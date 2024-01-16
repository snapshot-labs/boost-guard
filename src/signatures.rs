use crate::{ServerError, BOOST_NAME, BOOST_VERSION, VERIFYING_CONTRACT};
use core::convert::From;
use ethers::signers::LocalWallet;
use ethers::types::{
    transaction::eip712::{Eip712, TypedData},
    Address, Signature, U256,
};

#[derive(Debug, Clone)]
pub struct ClaimConfig {
    // The boost id where the claim is being made
    boost_id: U256,
    chain_id: U256,
    // The address of the recipient for the claim
    recipient: Address, // address
    // The amount of boost token in the claim
    amount: U256, // uint256
}

impl ClaimConfig {
    pub fn new(
        boost_id: &str,
        chain_id: &str,
        recipient: &str,
        amount: u128,
    ) -> Result<Self, ServerError> {
        Ok(Self {
            boost_id: U256::from_str_radix(boost_id, 10)?,
            chain_id: U256::from_str_radix(chain_id, 10)?,
            recipient: recipient.parse()?,
            amount: U256::from(amount),
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
              }
            ]
          },
          "primaryType": "Claim",
          "domain": {
            "name": BOOST_NAME.as_str(),
            "version": BOOST_VERSION.as_str(),
            "chainId": self.chain_id,
            "verifyingContract": VERIFYING_CONTRACT.as_str(),
          },
          "message": {
            "boostId": format!("{}", self.boost_id),
            "recipient": format!("{:?}", self.recipient),
            "amount": format!("{}", self.amount),
          }
        });

        let typed_data: TypedData = serde_json::from_value(json).expect("invalid json");
        let digest = typed_data.encode_eip712().expect("failed to encode eip712");

        signer
            .sign_hash(digest.into())
            .map_err(|e| ServerError::ErrorString(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::ClaimConfig;
    use ethers::types::U256;
    use std::{env, str::FromStr};

    #[test]
    fn test_simple_sig() {
        // Fix those env vars
        std::env::set_var(
            "PRIVATE_KEY",
            "0xafdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890",
        );
        std::env::set_var("BOOST_NAME", "boost");
        std::env::set_var("BOOST_VERSION", "1");
        std::env::set_var(
            "VERIFYING_CONTRACT",
            "0x3a18420C0646CC8e6D46E43d792335AeCB657fd0",
        );

        let claim_cfg = ClaimConfig {
            boost_id: U256::from(24),
            chain_id: U256::from(11155111),
            recipient: "0x3901D0fDe202aF1427216b79f5243f8A022d68cf"
                .parse()
                .unwrap(),
            amount: U256::from(1000000000000000_u128),
        };

        let private_key = env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
        let wallet = ethers::signers::LocalWallet::from_str(&private_key)
            .expect("failed to create a local wallet");

        let sig = claim_cfg.create_signature(&wallet).unwrap();
        assert!(sig.to_string() == "e299620773c7aa0ef7c715cd005eb48d0eacd8f6809bfa4505c96d7028b75d4931bdba5098e89259c97b2b059f9baea13e75a0ffe2d9379bbebbcfb5b8a932e01c");
    }
}
