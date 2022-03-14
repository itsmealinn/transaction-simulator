# Transaction simulator

## Assumptions

- transaction ids are unique
- disputes are only possible for deposits. The description of dispute, resolve and chargeback operations
  as detailed in the problem statement only make sense for deposits. They are also the target of the fraud
  scenario detailed in the statement.

## Error handling

Any potential CSV I/O errors are bubbled up and `expect()`-ed.
Other invalid transactions are simply ignored (considered an input error).

## Other notes

- all described transaction types are handled.
- whitespace is trimmed in the input file
- accounts are created only when a first deposit is made for an inexistent account id
- invalid input such as deposits/withdrawals without an amount or disputes/ with an amount
  are ignored.
- in order to get floating point precision suitable for financial applications, the project
  uses the `rust_decimal` crate.
- The time complexity of the end-to-end program is
  O(N), where N is the number of transactions in the file.
  The engine stores a hashmap of client accounts and each
  account a hashmap of disputable transactions mapped by their
  ids. Space complexity is therefore O(nr_deposits + nr_clients) = O(nr_deposits), since the number of deposits dominates the number of clients, but both are comparable to N.
- non-essential data is only streamed through memory and discarded afterwards.
- Strongly-typed data structures are used for input validation
  using `serde`. An additional `valid()` method is called on
  each line of input to make sure that the presence of the `amount` field
  corresponds to the transaction type.

## Extensibility

- The `Transaction` struct includes a `type` field, which can be used to extend the
  stored transaction record to other types as well (beyond deposits).
- The `Engine` can be extended to support multiple input formats (reusing under the hood the `process_transaction` method),
  coming from different sources as well. 

## Testing

Automated, integration-like tests are in the `main.rs` file, exercising both
invalid and valid inputs, covering as many corner cases as possible to prevent
future regressions.
