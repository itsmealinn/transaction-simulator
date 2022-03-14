use crate::{ClientId, TransactionId};
use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
/// The possible types of transaction input.
pub enum InputLineType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Deserialize)]
/// Strongly typed struct for deserializing a transaction description.
pub struct InputLine {
    pub r#type: InputLineType,
    pub client: ClientId,
    #[serde(rename = "tx")]
    pub id: TransactionId,
    pub amount: Option<Decimal>,
}

impl InputLine {
    pub fn valid(&self) -> bool {
        match self.r#type {
            InputLineType::Deposit => self.amount.is_some(),
            InputLineType::Withdrawal => self.amount.is_some(),
            InputLineType::Dispute => self.amount.is_none(),
            InputLineType::Resolve => self.amount.is_none(),
            InputLineType::Chargeback => self.amount.is_none(),
        }
    }
}
