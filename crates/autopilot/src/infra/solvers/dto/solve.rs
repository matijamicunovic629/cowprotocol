use {
    crate::{
        boundary,
        domain,
        infra::persistence::{dto, dto::order::Order},
    },
    chrono::{DateTime, Utc},
    itertools::Itertools,
    number::serialization::HexOrDecimalU256,
    primitive_types::{H160, U256},
    serde::{Deserialize, Serialize},
    serde_with::{serde_as, DisplayFromStr},
    std::{
        collections::{HashMap, HashSet},
        time::Duration,
    },
};

impl Request {
    pub fn new(
        id: domain::auction::Id,
        auction: &domain::Auction,
        trusted_tokens: &HashSet<H160>,
        time_limit: Duration,
    ) -> Self {
        Self {
            id,
            orders: auction
                .orders
                .clone()
                .into_iter()
                .map(dto::order::from_domain)
                .collect(),
            tokens: auction
                .prices
                .iter()
                .map(|(address, price)| Token {
                    address: address.to_owned().into(),
                    price: Some(price.get().into()),
                    trusted: trusted_tokens.contains(&(address.0)),
                })
                .chain(trusted_tokens.iter().map(|&address| Token {
                    address,
                    price: None,
                    trusted: true,
                }))
                .unique_by(|token| token.address)
                .collect(),
            deadline: Utc::now() + chrono::Duration::from_std(time_limit).unwrap(),
            surplus_capturing_jit_order_owners: auction
                .surplus_capturing_jit_order_owners
                .iter()
                .map(|address| address.0)
                .collect::<Vec<_>>(),
        }
    }
}

impl Response {
    pub fn into_domain(
        self,
    ) -> Vec<Result<domain::competition::Solution, domain::competition::SolutionError>> {
        self.solutions
            .into_iter()
            .map(Solution::into_domain)
            .collect()
    }
}

#[serde_as]
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    #[serde_as(as = "DisplayFromStr")]
    pub id: i64,
    pub tokens: Vec<Token>,
    pub orders: Vec<Order>,
    pub deadline: DateTime<Utc>,
    pub surplus_capturing_jit_order_owners: Vec<H160>,
}

#[serde_as]
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    pub address: H160,
    #[serde_as(as = "Option<HexOrDecimalU256>")]
    pub price: Option<U256>,
    pub trusted: bool,
}

#[serde_as]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradedAmounts {
    /// The effective amount that left the user's wallet including all fees.
    #[serde_as(as = "HexOrDecimalU256")]
    pub sell_amount: U256,
    /// The effective amount the user received after all fees.
    #[serde_as(as = "HexOrDecimalU256")]
    pub buy_amount: U256,
}

impl Solution {
    pub fn into_domain(
        self,
    ) -> Result<domain::competition::Solution, domain::competition::SolutionError> {
        Ok(domain::competition::Solution::new(
            self.solution_id,
            self.submission_address.into(),
            domain::competition::Score::new(self.score.into())?,
            self.orders
                .into_iter()
                .map(|(o, amounts)| {
                    (
                        o.into(),
                        domain::competition::TradedAmounts {
                            sell: amounts.sell_amount.into(),
                            buy: amounts.buy_amount.into(),
                        },
                    )
                })
                .collect(),
            self.clearing_prices
                .into_iter()
                .map(|(token, price)| {
                    domain::auction::Price::new(price.into()).map(|price| (token.into(), price))
                })
                .collect::<Result<_, _>>()?,
        ))
    }
}

#[serde_as]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Solution {
    /// Unique ID of the solution (per driver competition), used to identify
    /// it in subsequent requests (reveal, settle).
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub solution_id: u64,
    #[serde_as(as = "HexOrDecimalU256")]
    pub score: U256,
    /// Address used by the driver to submit the settlement onchain.
    pub submission_address: H160,
    pub orders: HashMap<boundary::OrderUid, TradedAmounts>,
    #[serde_as(as = "HashMap<_, HexOrDecimalU256>")]
    pub clearing_prices: HashMap<H160, U256>,
    pub gas: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Response {
    pub solutions: Vec<Solution>,
}
