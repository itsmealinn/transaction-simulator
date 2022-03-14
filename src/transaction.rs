use rust_decimal::Decimal;

#[derive(Clone, Copy)]
// Only need to store deposits for the moment.
pub enum TransactionType {
    Deposit,
}

/// A transaction stored in the account. Currently, only deposits are stored, since
/// those are the only transactions that may be disputed.
/// We store the transaction type for extensibility.
pub struct Transaction {
    #[allow(unused)]
    r#type: TransactionType,
    amount: Decimal,
    disputed: bool,
}

impl Transaction {
    pub fn deposit(amount: Decimal) -> Self {
        Self {
            r#type: TransactionType::Deposit,
            amount,
            disputed: false,
        }
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }

    pub fn dispute(&mut self) {
        if !self.disputed {
            self.disputed = true;
        }
    }

    pub fn undispute(&mut self) {
        if self.disputed {
            self.disputed = false;
        }
    }

    pub fn disputed(&self) -> bool {
        self.disputed
    }
}
