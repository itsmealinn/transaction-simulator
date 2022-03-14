use crate::io::{InputLine, InputLineType};
use crate::transaction::Transaction;
use crate::{ClientId, TransactionId};

use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
#[cfg_attr(test, derive(PartialEq, Debug))]
/// Serializable struct for outputting information about a client's account.
pub struct AccountStatus {
    pub client: ClientId,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

pub struct Account {
    available: Decimal,
    held: Decimal,
    locked: bool,

    /// Collection of disputable transactions. Using a hashmap as we always refer to
    /// a transaction by its id. Currently disputes are implemented for deposits.
    deposits: HashMap<TransactionId, Transaction>,
}

impl Default for Account {
    fn default() -> Self {
        Self {
            available: Decimal::new(0, 0),
            held: Decimal::new(0, 0),
            locked: false,
            deposits: HashMap::new(),
        }
    }
}

impl Account {
    /// Process an account operation.
    pub fn process(&mut self, input_line: InputLine) {
        if !self.locked {
            match input_line.r#type {
                InputLineType::Deposit => self.deposit(input_line),
                InputLineType::Withdrawal => self.withdrawal(input_line),
                InputLineType::Dispute => self.dispute(input_line),
                InputLineType::Resolve => self.resolve(input_line),
                InputLineType::Chargeback => self.chargeback(input_line),
            };
        }
    }

    pub fn get_status(&self, client: ClientId) -> AccountStatus {
        AccountStatus {
            client,
            available: self.available,
            held: self.held,
            total: self.total(),
            locked: self.locked,
        }
    }

    fn deposit(&mut self, input_line: InputLine) {
        // The unwrap is safe because the Engine validated the input before sending the transaction here.
        let amount = input_line.amount.unwrap();
        // Just increase the available amount and add it the to the deposits history.
        self.available += amount;
        self.deposits
            .insert(input_line.id, Transaction::deposit(amount));
    }

    fn withdrawal(&mut self, input_line: InputLine) {
        // The unwrap is safe because the Engine validated the input before sending the transaction here.
        let amount = input_line.amount.unwrap();

        // If we don't have enough money, just ingore.
        if self.available >= amount {
            self.available -= amount;
        }
    }

    fn dispute(&mut self, input_line: InputLine) {
        // Disputes only work for deposits.
        if let Some(transaction) = self.deposits.get_mut(&input_line.id) {
            // Check that it's not already disputed.
            if !transaction.disputed() {
                // Mark this deposit as disputed, so that we can validate that resolves and
                // chargebacks are only applied on disputes.
                transaction.dispute();
                let amount = transaction.amount();
                // The client may end up with a negative balance if they already withdrew the money.
                self.available -= amount;
                self.held += amount;
            }
        }
    }

    fn resolve(&mut self, input_line: InputLine) {
        if let Some(transaction) = self.deposits.get_mut(&input_line.id) {
            if transaction.disputed() {
                self.available += transaction.amount();
                self.held -= transaction.amount();
                transaction.undispute();
            }
        }
    }

    fn chargeback(&mut self, input_line: InputLine) {
        if let Some(transaction) = self.deposits.get_mut(&input_line.id) {
            if transaction.disputed() {
                self.held -= transaction.amount();
                transaction.undispute();

                self.lock();
            }
        }
    }

    // Don't store the total, we can calculate it when needed
    fn total(&self) -> Decimal {
        self.available + self.held
    }

    fn lock(&mut self) {
        self.locked = true;
    }
}
