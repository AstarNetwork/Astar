use bstringify::bstringify;
use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::{
    convert::{Into, TryFrom, TryInto},
    prelude::*,
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

macro_rules! create_currency_id {
    ($(#[$meta:meta])*
    $vis:vis enum TokenSymbol {
        $($(#[$vmeta:meta])* $symbol:ident($name:expr, $deci:literal) = $val:literal,)*
    }) => {
        $(#[$meta])*
        $vis enum TokenSymbol {
            $($(#[$vmeta])* $symbol = $val,)*
        }

        impl TryFrom<u8> for TokenSymbol {
            type Error = ();

            fn try_from(v: u8) -> Result<Self, Self::Error> {
                match v {
                    $($val => Ok(TokenSymbol::$symbol),)*
                    _ => Err(()),
                }
            }
        }

        impl TryFrom<Vec<u8>> for CurrencyId {
            type Error = ();
            fn try_from(v: Vec<u8>) -> Result<CurrencyId, ()> {
                match v.as_slice() {
                    $(bstringify!($symbol) => Ok(CurrencyId::Token(TokenSymbol::$symbol)),)*
                    _ => Err(()),
                }
            }
        }

        impl GetDecimals for CurrencyId {
            fn decimals(&self) -> u32 {
                match self {
                    $(CurrencyId::Token(TokenSymbol::$symbol) => $deci,)*
                    // default decimals is 18
                    _ => 18,
                }
            }
        }

        $(pub const $symbol: CurrencyId = CurrencyId::Token(TokenSymbol::$symbol);)*

        impl TokenSymbol {
            pub fn get_info() -> Vec<(&'static str, u32)> {
                vec![
                    $((stringify!($symbol), $deci),)*
                ]
            }
        }

        #[test]
        #[ignore]
        fn generate_token_resources() {
            #[allow(non_snake_case)]
            #[derive(Serialize, Deserialize)]
            struct Token {
                name: String,
                symbol: String,
                decimals: u8,
                currencyId: u8,
            }

            let tokens = vec![
                $(
                    Token {
                        name: $name.to_string(),
                        symbol: stringify!($symbol).to_string(),
                        decimals: $deci,
                        currencyId: $val,
                    },
                )*
            ];
            frame_support::assert_ok!(std::fs::write("../predeploy-contracts/resources/tokens.json", serde_json::to_string_pretty(&tokens).unwrap()));
        }
    }
}

create_currency_id! {
    // Represent a Token symbol with 8 bit
    // Bit 8 : 0 for Pokladot Ecosystem, 1 for Kusama Ecosystem
    // Bit 7 : Reserved
    // Bit 6 - 1 : The token ID
    #[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[repr(u8)]
    pub enum TokenSymbol {
        // Polkadot Ecosystem
        ACA("Acala", 13) = 0, //TODO
        AUSD("Acala Dollar", 12) = 1, //TODO
        DOT("Polkadot", 10) = 2, //TODO
        LDOT("Liquid DOT", 10) = 3, //TODO
        XBTC("ChainX BTC", 8) = 4,
        RENBTC("Ren Protocol BTC", 8) = 5, //TODO
        POLKABTC("PolkaBTC", 8) = 6,
        PLM("Plasm", 18) = 7,
        PHA("Phala", 18) = 8,
        HDT("HydraDX", 12) = 9,
        BCG("Bit.Country", 18) = 11,

        // Kusama Ecosystem
        KAR("Karura", 12) = 128, //TODO:
        KUSD("Karura Dollar", 12) = 129, //TODO
        KSM("Kusama", 12) = 130, //TODO
        LKSM("Liquid KSM", 12) = 131, //TODO
        // Reserve for XBTC = 132
        // Reserve for RENBTC = 133
        // Reserve for POLKABTC = 134
        SDN("Shiden", 18) = 135,
        // Reserve for PHA = 136
        // Reserve for HDT = 137
        KILT("Kilt", 15) = 138,
        // Reserve for BCG = 139
    }
}

pub trait GetDecimals {
    fn decimals(&self) -> u32;
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
    Token(TokenSymbol),
}

impl CurrencyId {
    pub fn is_token_currency_id(&self) -> bool {
        matches!(self, CurrencyId::Token(_))
    }
}

impl TryFrom<[u8; 32]> for CurrencyId {
    type Error = ();

    fn try_from(v: [u8; 32]) -> Result<Self, Self::Error> {
        if !v.starts_with(&[0u8; 29][..]) {
            return Err(());
        }

        // token
        if v[29] == 0 && v[31] == 0 {
            return v[30].try_into().map(CurrencyId::Token);
        }

        Err(())
    }
}

impl From<CurrencyId> for [u8; 32] {
    fn from(val: CurrencyId) -> Self {
        let mut bytes = [0u8; 32];
        match val {
            CurrencyId::Token(token) => {
                bytes[30] = token as u8;
            }
            _ => {}
        }
        bytes
    }
}
