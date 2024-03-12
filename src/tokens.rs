use ethers::types::Address;
use std::collections::HashSet;

const ETHEREUM_CHAIN_ID: &str = "1";
const POLYGON_CHAIN_ID: &str = "137";

/// Returns a set where they key is (token_address, chain_id)
pub fn create_disabled_token_list() -> HashSet<(Address, &'static str)> {
    let data = json::parse(get_list()).expect("error in the list");
    let mut disabled_tokens = HashSet::new();

    data.members().for_each(|token| {
        let eth = token["ethereum"]
            .as_str()
            .expect("error in the JSON object");
        let poly = token["polygon"].as_str().expect("error in the JSON object");

        if !eth.is_empty() {
            disabled_tokens.insert((eth.parse().unwrap(), ETHEREUM_CHAIN_ID));
        }

        if !poly.is_empty() {
            disabled_tokens.insert((poly.parse().unwrap(), POLYGON_CHAIN_ID));
        }
    });

    disabled_tokens
}

fn get_list() -> &'static str {
    r#"[
    {
        "symbol": "USDT",
        "ethereum": "0xdac17f958d2ee523a2206206994597c13d831ec7",
        "polygon": "0xc2132d05d31c914a87c6611c10748aeb04b58e8f"
    },
    {
        "symbol": "USDC",
        "ethereum": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        "polygon": "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359"
    },
    {
        "symbol": "DAI",
        "ethereum": "0x6b175474e89094c44da98b954eedeac495271d0f",
        "polygon": "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063"
    },
    {
        "symbol": "FDUSD",
        "ethereum": "0xc5f0f7b66764f6ec8c8dff7ba683102295e16409",
        "polygon": ""
    },
    {
        "symbol": "TUSD",
        "ethereum": "0x0000000000085d4780b73119b644ae5ecd22b376",
        "polygon": ""
    },
    {
        "symbol": "FRAX",
        "ethereum": "0x853d955acef822db058eb8505911ed77f175b99e",
        "polygon": "0x45c32fa6df82ead1e2ef74d17b76547eddfaff89"
    },
    {
        "symbol": "GUSD",
        "ethereum": "0x056fd409e1d7a124bd7017459dfea2f387b6d5cd",
        "polygon": ""
    },
    {
        "symbol": "PYUSD",
        "ethereum": "0x6c3ea9036406852006290770bedfcaba0e23a0e8",
        "polygon": ""
    },
    {
        "symbol": "sUSD",
        "ethereum": "0x57ab1ec28d129707052df4df418d58a2d46d5f51",
        "polygon": ""
    },
    {
        "symbol": "USDP",
        "ethereum": "0x8e870d67f660d95d5be530380d0ec0bd388289e1",
        "polygon": ""
    },
    {
        "symbol": "LUSD",
        "ethereum": "0x5f98805a4e8be255a32880fdec7f6728c6568ba0",
        "polygon": "0x23001f892c0c82b79303edc9b9033cd190bb21c7"
    },
    {
        "symbol": "GHO",
        "ethereum": "0x40d16fc0246ad3160ccc09b8d0d3a2cd28ae6c2f",
        "polygon": ""
    },
    {
        "symbol": "WBTC",
        "ethereum": "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599",
        "polygon": "0x1bfd67037b42cf73acF2047067bd4F2C47D9BfD6"
    },
    {
        "symbol": "WETH",
        "ethereum": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
        "polygon": "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619"
    },
    {
        "symbol": "STETH",
        "ethereum": "0xae7ab96520de3a18e5e111b5eaab095312d7fe84",
        "polygon": ""
    },
    {
        "symbol": "WSTETH",
        "ethereum": "0x7f39c581f595b53c5cb19bd0b3f8da6c935e2ca0",
        "polygon": "0x03b54a6e9a984069379fae1a4fc4dbae93b3bccd"
    },
    {
        "symbol": "CBETH",
        "ethereum": "0xbe9895146f7af43049ca1c1ae358b0541ea49704",
        "polygon": "0x4b4327db1600b8b1440163f667e199cef35385f5"
    },
    {
        "symbol": "ANKRETH",
        "ethereum": "0xe95a203b1a91a908f9b9ce46459d101078c2c3cb",
        "polygon": ""
    },
    {
        "symbol": "OSETH",
        "ethereum": "0xf1c9acdc66974dfb6decb12aa385b9cd01190e38",
        "polygon": ""
    }
]"#
}

#[cfg(test)]
mod test_tokens {
    use super::create_disabled_token_list;

    #[test]
    fn test_create_disabled_token_list() {
        let disabled_tokens = create_disabled_token_list();
        assert_eq!(disabled_tokens.len(), 28);
        assert!(disabled_tokens.contains(&(
            "0xdac17f958d2ee523a2206206994597c13d831ec7"
                .parse()
                .unwrap(),
            "1"
        )));
    }
}
