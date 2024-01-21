//! The bank contract is one of the two "core" contracts required by CWD,
//! meaning contracts that provide core functionalities of the chain, the other
//! being the tax man, which levies transaction fees.
//!
//! The bank contract must implement the following two entry points:
//!
//! ```rust
//! #[entry_point]
//! fn transfer<E>(ctx: TransferCtx, msg: TransferMsg) -> Result<Response, E>;
//!
//! #[entry_point]
//! fn query_bank<E>(ctx: QueryCtx, msg: BankQuery) -> Result<BankQueryResponse, E>;
//! ```

use {
    crate::{Addr, Coin, Coins},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TransferMsg {
    pub from:  Addr,
    pub to:    Addr,
    pub coins: Coins,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum BankQuery {
    Balance {
        address: Addr,
        denom:   String,
    },
    Balances {
        address:     Addr,
        start_after: Option<String>,
        limit:       Option<u32>,
    },
    Supply {
        denom: String,
    },
    Supplies {
        start_after: Option<String>,
        limit:       Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum BankQueryResponse {
    Balance(Coin),
    Balances(Coins),
    Supply(Coin),
    Supplies(Coins),
}