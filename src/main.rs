use std::collections::{hash_map::Entry, HashMap};
use std::env;
use std::fs::File;

use csv::{Error as CsvError, Trim};

mod account;
mod io;
mod transaction;

use account::Account;
use io::{InputLine, InputLineType};

/// Useful type alias for client id.
type ClientId = u16;
/// Useful type alias for transaction id.
type TransactionId = u32;
/// Application result type. Currently only possible error comes from CSV.
/// If more error types are to be added, we should define a new Error enum.
type Result<T> = std::result::Result<T, CsvError>;

#[derive(Default)]
/// The transaction engine type.
/// Uses the `newtype` design pattern, since it only has one field and
/// we want to be able to have methods defined on it.
struct Engine(HashMap<ClientId, Account>);

impl Engine {
    /// Process transactions in CSV format, in-order.
    pub fn process_from_csv<R: std::io::Read>(&mut self, reader: R) -> Result<()> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .trim(Trim::All)
            .from_reader(reader);

        for result in reader.deserialize::<InputLine>() {
            self.process_transaction(result?);
        }

        Ok(())
    }

    /// Write the status of every account in CSV format.
    pub fn write_status_to_csv<W: std::io::Write>(&self, writer: W) -> Result<()> {
        let mut writer = csv::WriterBuilder::new().from_writer(writer);

        for (client_id, account) in self.0.iter() {
            writer.serialize(account.get_status(*client_id))?;
        }

        Ok(())
    }

    /// Main function for transaction processing. Can be used for data expressed in multiple
    /// formats. For example, it may be called in the future for processing a transaction
    /// formatted as JSON received over a TCP socket.
    fn process_transaction(&mut self, input_line: InputLine) {
        if !input_line.valid() {
            return;
        }
        let entry = self.0.entry(input_line.client);

        let account = match entry {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) if input_line.r#type == InputLineType::Deposit => {
                e.insert(Default::default())
            }
            // If we got a Vacant entry but the tx type is not Deposit, ignore.
            _ => return,
        };

        account.process(input_line);
    }
}

fn run_engine<R: std::io::Read, W: std::io::Write>(reader: R, writer: W) -> Result<()> {
    let mut engine: Engine = Default::default();
    engine.process_from_csv(reader)?;

    engine.write_status_to_csv(writer)?;

    Ok(())
}

fn main() {
    let mut args: Vec<String> = env::args().collect();
    // We assume we get only one argument, the input file.
    assert_eq!(args.len(), 2);
    let input_path = args.remove(1);

    // Unrecoverable errors are bubbled up to here, where we panic.
    run_engine(File::open(input_path).unwrap(), std::io::stdout()).expect("Unrecoverable error");
}

#[cfg(test)]
mod tests {
    use super::Engine;
    use crate::account::AccountStatus;
    use rust_decimal_macros::dec;

    fn validate(input: &[u8], mut expected_output: Vec<AccountStatus>) {
        let mut engine: Engine = Default::default();
        engine.process_from_csv(input).unwrap();

        let mut output = vec![];
        for (client_id, account) in engine.0.iter() {
            output.push(account.get_status(*client_id));
        }

        expected_output.sort_by_key(|line| line.client);
        output.sort_by_key(|line| line.client);

        assert_eq!(output, expected_output);
    }

    #[test]
    fn test_deposit_and_withdrawal() {
        // No transactions.
        validate("type, client, tx, amount".as_bytes(), vec![]);

        validate("".as_bytes(), vec![]);

        // No amount specified.
        validate(
            r#"
                type, client, tx, amount
                withdrawal, 2, 4,
                deposit, 1, 1,
                withdrawal, 1, 5,"#
                .as_bytes(),
            vec![],
        );

        // Non-continuous client and tx ids.
        // Attempts to have a withdrawal for an account with 0 balance.
        // Attempts to withdraw too much
        validate(
            r#"
                type, client, tx, amount
                withdrawal, 2, 4, 1.5
                deposit, 1, 1, 3.5
                withdrawal, 1, 5, 2.0
                withdrawal, 1, 6, 2.0"#
                .as_bytes(),
            vec![AccountStatus {
                client: 1,
                available: dec!(1.5),
                held: dec!(0.0),
                total: dec!(1.5),
                locked: false,
            }],
        );

        // Sample from PDF.
        validate(
            r#"
                type, client, tx, amount
                deposit, 1, 1, 1.0
                deposit, 2, 2, 2.0
                deposit, 1, 3, 2.0
                withdrawal, 1, 4, 1.5
                withdrawal, 2, 5, 3.0"#
                .as_bytes(),
            vec![
                AccountStatus {
                    client: 1,
                    available: dec!(1.5),
                    held: dec!(0.0),
                    total: dec!(1.5),
                    locked: false,
                },
                AccountStatus {
                    client: 2,
                    available: dec!(2.0),
                    held: dec!(0.0),
                    total: dec!(2.0),
                    locked: false,
                },
            ],
        );
    }

    #[test]
    fn test_dispute() {
        // dispute an inexistent transaction. will be ignored
        validate(
            r#"
                type, client, tx, amount
                deposit, 1, 1, 3.5
                dispute, 1, 3,
                withdrawal, 1, 2, 2.0"#
                .as_bytes(),
            vec![AccountStatus {
                client: 1,
                available: dec!(1.5),
                held: dec!(0.0),
                total: dec!(1.5),
                locked: false,
            }],
        );

        // dispute a transaction belonging to another client. will be ignored.
        validate(
            r#"
                type, client, tx, amount
                deposit, 1, 1, 3.5
                deposit, 2, 2, 4
                dispute, 1, 2,
                withdrawal, 1, 2, 2.0"#
                .as_bytes(),
            vec![
                AccountStatus {
                    client: 1,
                    available: dec!(1.5),
                    held: dec!(0.0),
                    total: dec!(1.5),
                    locked: false,
                },
                AccountStatus {
                    client: 2,
                    available: dec!(4.0),
                    held: dec!(0.0),
                    total: dec!(4.0),
                    locked: false,
                },
            ],
        );

        // verify that disputing a withdrawal doesn't work
        validate(
            r#"
                type, client, tx, amount
                deposit, 1, 1, 3.5
                withdrawal, 1, 2, 2.0
                dispute, 1, 2,"#
                .as_bytes(),
            vec![AccountStatus {
                client: 1,
                available: dec!(1.5),
                held: dec!(0.0),
                total: dec!(1.5),
                locked: false,
            }],
        );

        // dispute same transaction twice
        validate(
            r#"
                type, client, tx, amount
                deposit, 1, 1, 3.5
                dispute, 1, 1,
                dispute, 1, 1,"#
                .as_bytes(),
            vec![AccountStatus {
                client: 1,
                available: dec!(0.0),
                held: dec!(3.5),
                total: dec!(3.5),
                locked: false,
            }],
        );

        // dispute specifying an amount. will be ignored.
        validate(
            r#"
                type, client, tx, amount
                deposit, 1, 1, 3.5
                dispute, 1, 1, 4"#
                .as_bytes(),
            vec![AccountStatus {
                client: 1,
                available: dec!(3.5),
                held: dec!(0.0),
                total: dec!(3.5),
                locked: false,
            }],
        );

        // Dispute getting a negative balance.
        validate(
            r#"
                type, client, tx, amount
                deposit, 1, 1, 3
                withdrawal, 1, 2, 2
                dispute, 1, 1,"#
                .as_bytes(),
            vec![AccountStatus {
                client: 1,
                available: dec!(-2.0),
                held: dec!(3.0),
                total: dec!(1.0),
                locked: false,
            }],
        );
    }

    #[test]
    fn test_resolve() {
        // Resolve specifying an amount, will be ignored.
        validate(
            r#"
                type, client, tx, amount
                deposit, 1, 1, 3
                withdrawal, 1, 2, 2
                dispute, 1, 1,
                resolve, 1, 1, 3"#
                .as_bytes(),
            vec![AccountStatus {
                client: 1,
                available: dec!(-2.0),
                held: dec!(3.0),
                total: dec!(1.0),
                locked: false,
            }],
        );

        // Valid resolve.
        validate(
            r#"
                type, client, tx, amount
                deposit, 1, 1, 3
                withdrawal, 1, 2, 2
                dispute, 1, 1,
                resolve, 1, 1,"#
                .as_bytes(),
            vec![AccountStatus {
                client: 1,
                available: dec!(1.0),
                held: dec!(0.0),
                total: dec!(1.0),
                locked: false,
            }],
        );
    }

    #[test]
    fn test_chargeback() {
        // Chargeback specifying an amount, will be ignored.
        validate(
            r#"
                type, client, tx, amount
                deposit, 1, 1, 3
                withdrawal, 1, 2, 2
                dispute, 1, 1,
                chargeback, 1, 1, 3"#
                .as_bytes(),
            vec![AccountStatus {
                client: 1,
                available: dec!(-2.0),
                held: dec!(3.0),
                total: dec!(1.0),
                locked: false,
            }],
        );

        // Valid chargeback.
        // No more transactions allowed on locked account.
        validate(
            r#"
                type, client, tx, amount
                deposit, 1, 1, 3
                withdrawal, 1, 2, 2
                dispute, 1, 1,
                chargeback, 1, 1,
                deposit, 2, 3, 100
                deposit, 1, 4, 100"#
                .as_bytes(),
            vec![
                AccountStatus {
                    client: 1,
                    available: dec!(-2.0),
                    held: dec!(0.0),
                    total: dec!(-2.0),
                    locked: true,
                },
                AccountStatus {
                    client: 2,
                    available: dec!(100.0),
                    held: dec!(0.0),
                    total: dec!(100.0),
                    locked: false,
                },
            ],
        );
    }
}
