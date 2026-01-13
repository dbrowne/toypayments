use std::collections::HashMap;

use thiserror::Error;
use tracing::{debug, instrument, trace};

use crate::account::{Account, AccountError};
use crate::transaction::{StoredTransaction, TransactionRecord, TransactionType};

/// Can you stream values through memory as opposed to loading the entire dataset upfront? YES.
/// This code processes each line of the csv individually and is limited by host memory.
/// Since hashmap growth requires the temporary allocation of double the memory one way around that
/// would be to just pre allocate the hashmap like this:
///    pub fn new() -> Self {
///       Self {
///           accounts: HashMap::new(),
///           transactions: HashMap::with_capacity(SOME_AVERAGE_RUNTIME_CAPACITY),
///       }
///   }
/// Not doing that since I do not know the details of the testing env.
///
/// The payments engine that processes transactions and maintains state for non concurrent processing
/// If I were bundled  in a server I would use DashMap for concurrent Hashmap access
/// Would have a Per-client RwLock<Account> with separate transaction storage
/// would implement external storage (redis or postgres) for horizontal scaling.
/// This is currently not thread safe
pub struct Engine {
  /// Client accounts indexed by client id
  accounts: HashMap<u16, Account>,
  ///  The stored transactions that can be disputed
  transactions: HashMap<u32, StoredTransaction>,
}

impl Engine {
  pub fn new() -> Self {
    Self { accounts: HashMap::new(), transactions: HashMap::new() }
  }

  pub fn process(&mut self, record: TransactionRecord) -> Result<(), EngineError> {
    match record.tx_type {
      TransactionType::Deposit => self.proc_deposit(record),
      TransactionType::Withdrawal => self.proc_withdrawal(record),
      TransactionType::Dispute => self.proc_dispute(record),
      TransactionType::Resolve => self.proc_resolve(record),
      TransactionType::Chargeback => self.proc_chargeback(record),
    }
  }

  #[instrument(skip(self), fields(tx = record.tx, client = record.client))]
  fn proc_deposit(&mut self, record: TransactionRecord) -> Result<(), EngineError> {
    let amount =
      record.amount.ok_or(EngineError::MissingAmount { tx: record.tx, tx_type: record.tx_type })?;

    trace!(%amount, "Processing deposit");

    // Do we have a dupe Id?
    if self.transactions.contains_key(&record.tx) {
      return Err(EngineError::DuplicateTransaction { tx: record.tx });
    }

    let is_new_account = !self.accounts.contains_key(&record.client);
    let account = self.accounts.entry(record.client).or_insert_with(|| Account::new(record.client));

    if is_new_account {
      debug!(client = record.client, "Created new account");
    }

    account.deposit(amount).map_err(|e| EngineError::AccountError {
      tx: record.tx,
      client: record.client,
      error: e,
    })?;

    // Save the transaction
    self
      .transactions
      .insert(record.tx, StoredTransaction::new(TransactionType::Deposit, record.client, amount));

    trace!(new_balance = %account.available, "Deposit complete");
    Ok(())
  }

  fn proc_withdrawal(&mut self, record: TransactionRecord) -> Result<(), EngineError> {
    let amount =
      record.amount.ok_or(EngineError::MissingAmount { tx: record.tx, tx_type: record.tx_type })?;

    // Do we have a dupe ID?
    if self.transactions.contains_key(&record.tx) {
      return Err(EngineError::DuplicateTransaction { tx: record.tx });
    }

    let account = self.accounts.entry(record.client).or_insert_with(|| Account::new(record.client));

    account.withdraw(amount).map_err(|e| EngineError::AccountError {
      tx: record.tx,
      client: record.client,
      error: e,
    })?;

    // Store the transaction for potential future disputes
    // Note: The spec is ambiguous about whether withdrawals can be disputed
    // We store them to be safe, but only deposits make sense to dispute
    self.transactions.insert(
      record.tx,
      StoredTransaction::new(TransactionType::Withdrawal, record.client, amount),
    );

    Ok(())
  }

  fn proc_dispute(&mut self, record: TransactionRecord) -> Result<(), EngineError> {
    let stored_tx = self
      .transactions
      .get_mut(&record.tx)
      .ok_or(EngineError::TransactionNotFound { tx: record.tx })?;

    // Verify the client matches
    if stored_tx.client != record.client {
      return Err(EngineError::ClientMismatch {
        tx: record.tx,
        expected: stored_tx.client,
        actual: record.client,
      });
    }

    // Check if already under dispute
    if stored_tx.disputed {
      return Err(EngineError::AlreadyDisputed { tx: record.tx });
    }

    // Only deposits can be meaningfully disputed (reversing a deposit)
    // Disputing a withdrawal would mean giving money back, which doesn't make sense
    if stored_tx.tx_type != TransactionType::Deposit {
      return Err(EngineError::CannotDisputeWithdrawal { tx: record.tx });
    }

    let account = self
      .accounts
      .get_mut(&record.client)
      .ok_or(EngineError::ClientNotFound { client: record.client })?;

    // Move funds from available to held
    account.hold(stored_tx.amount).map_err(|e| EngineError::AccountError {
      tx: record.tx,
      client: record.client,
      error: e,
    })?;

    stored_tx.disputed = true;

    Ok(())
  }

  fn proc_resolve(&mut self, record: TransactionRecord) -> Result<(), EngineError> {
    let stored_tx = self
      .transactions
      .get_mut(&record.tx)
      .ok_or(EngineError::TransactionNotFound { tx: record.tx })?;

    // Verify the client matches
    if stored_tx.client != record.client {
      return Err(EngineError::ClientMismatch {
        tx: record.tx,
        expected: stored_tx.client,
        actual: record.client,
      });
    }

    // Must be under dispute to resolve
    if !stored_tx.disputed {
      return Err(EngineError::NotUnderDispute { tx: record.tx });
    }

    let account = self
      .accounts
      .get_mut(&record.client)
      .ok_or(EngineError::ClientNotFound { client: record.client })?;

    // Move funds from held back to available
    account.release(stored_tx.amount).map_err(|e| EngineError::AccountError {
      tx: record.tx,
      client: record.client,
      error: e,
    })?;

    stored_tx.disputed = false;

    Ok(())
  }

  fn proc_chargeback(&mut self, record: TransactionRecord) -> Result<(), EngineError> {
    let stored_tx = self
      .transactions
      .get_mut(&record.tx)
      .ok_or(EngineError::TransactionNotFound { tx: record.tx })?;

    // does the client match?
    if stored_tx.client != record.client {
      return Err(EngineError::ClientMismatch {
        tx: record.tx,
        expected: stored_tx.client,
        actual: record.client,
      });
    }

    if !stored_tx.disputed {
      return Err(EngineError::NotUnderDispute { tx: record.tx });
    }

    let account = self
      .accounts
      .get_mut(&record.client)
      .ok_or(EngineError::ClientNotFound { client: record.client })?;

    // Remove held funds and lock the account
    account.chargeback(stored_tx.amount).map_err(|e| EngineError::AccountError {
      tx: record.tx,
      client: record.client,
      error: e,
    })?;

    stored_tx.disputed = false;

    Ok(())
  }

  pub fn accounts(&self) -> impl Iterator<Item = &Account> {
    self.accounts.values()
  }
}

impl Default for Engine {
  fn default() -> Self {
    Self::new()
  }
}

/// AI GENERATED Errors that can occur during transaction processing
/// PROMPT: Re implement error handling using thiserror
#[derive(Debug, Error)]
pub enum EngineError {
  #[error("tx {tx}: {tx_type:?} requires an amount")]
  MissingAmount { tx: u32, tx_type: TransactionType },
  #[error("tx {tx}: duplicate transaction ID")]
  DuplicateTransaction { tx: u32 },
  #[error("tx {tx}: transaction not found")]
  TransactionNotFound { tx: u32 },
  #[error("client {client}: not found")]
  ClientNotFound { client: u16 },
  #[error("tx {tx}: client mismatch (expected {expected}, got {actual})")]
  ClientMismatch { tx: u32, expected: u16, actual: u16 },
  #[error("tx {tx}: already under dispute")]
  AlreadyDisputed { tx: u32 },
  #[error("tx {tx}: not under dispute")]
  NotUnderDispute { tx: u32 },
  #[error("tx {tx}: cannot dispute a withdrawal")]
  CannotDisputeWithdrawal { tx: u32 },
  #[error("tx {tx} (client {client}): {error}")]
  AccountError {
    tx: u32,
    client: u16,
    #[source]
    error: AccountError,
  },
}

/// AI GENERATED TESTS
/// PROMPT: create the necessary test cases for the code in engine.rs
#[cfg(test)]
mod tests {
  use super::*;
  use rust_decimal::Decimal;

  fn deposit(client: u16, tx: u32, amount: &str) -> TransactionRecord {
    TransactionRecord {
      tx_type: TransactionType::Deposit,
      client,
      tx,
      amount: Some(amount.parse().unwrap()),
    }
  }

  fn withdrawal(client: u16, tx: u32, amount: &str) -> TransactionRecord {
    TransactionRecord {
      tx_type: TransactionType::Withdrawal,
      client,
      tx,
      amount: Some(amount.parse().unwrap()),
    }
  }

  fn dispute(client: u16, tx: u32) -> TransactionRecord {
    TransactionRecord { tx_type: TransactionType::Dispute, client, tx, amount: None }
  }

  fn resolve(client: u16, tx: u32) -> TransactionRecord {
    TransactionRecord { tx_type: TransactionType::Resolve, client, tx, amount: None }
  }

  fn chargeback(client: u16, tx: u32) -> TransactionRecord {
    TransactionRecord { tx_type: TransactionType::Chargeback, client, tx, amount: None }
  }

  #[test]
  fn test_basic_deposit_withdrawal() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    engine.process(deposit(1, 2, "50.0")).unwrap();
    engine.process(withdrawal(1, 3, "75.0")).unwrap();

    let account = engine.accounts.get(&1).unwrap();
    assert_eq!(account.available, Decimal::new(75, 0));
    assert_eq!(account.total(), Decimal::new(75, 0));
  }

  #[test]
  fn test_withdrawal_insufficient_funds() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "50.0")).unwrap();
    let result = engine.process(withdrawal(1, 2, "100.0"));

    assert!(matches!(result, Err(EngineError::AccountError { .. })));
  }

  #[test]
  fn test_dispute_resolve_flow() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    engine.process(dispute(1, 1)).unwrap();

    let account = engine.accounts.get(&1).unwrap();
    assert_eq!(account.available, Decimal::ZERO);
    assert_eq!(account.held, Decimal::new(100, 0));
    assert_eq!(account.total(), Decimal::new(100, 0));

    engine.process(resolve(1, 1)).unwrap();

    let account = engine.accounts.get(&1).unwrap();
    assert_eq!(account.available, Decimal::new(100, 0));
    assert_eq!(account.held, Decimal::ZERO);
  }

  #[test]
  fn test_dispute_chargeback_flow() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    engine.process(dispute(1, 1)).unwrap();
    engine.process(chargeback(1, 1)).unwrap();

    let account = engine.accounts.get(&1).unwrap();
    assert_eq!(account.available, Decimal::ZERO);
    assert_eq!(account.held, Decimal::ZERO);
    assert_eq!(account.total(), Decimal::ZERO);
    assert!(account.locked);
  }

  #[test]
  fn test_dispute_nonexistent_tx() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    let result = engine.process(dispute(1, 999));

    assert!(matches!(result, Err(EngineError::TransactionNotFound { .. })));
  }

  #[test]
  fn test_resolve_not_disputed() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    let result = engine.process(resolve(1, 1));

    assert!(matches!(result, Err(EngineError::NotUnderDispute { .. })));
  }

  #[test]
  fn test_client_mismatch() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    // Client 2 tries to dispute client 1's transaction
    let result = engine.process(dispute(2, 1));

    assert!(matches!(result, Err(EngineError::ClientMismatch { .. })));
  }

  #[test]
  fn test_multiple_clients() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    engine.process(deposit(2, 2, "200.0")).unwrap();
    engine.process(withdrawal(1, 3, "50.0")).unwrap();

    let account1 = engine.accounts.get(&1).unwrap();
    let account2 = engine.accounts.get(&2).unwrap();

    assert_eq!(account1.available, Decimal::new(50, 0));
    assert_eq!(account2.available, Decimal::new(200, 0));
  }

  #[test]
  fn test_duplicate_transaction_id() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    let result = engine.process(deposit(1, 1, "50.0"));

    assert!(matches!(result, Err(EngineError::DuplicateTransaction { .. })));
  }

  // =========================================================================
  // EDGE CASE UNIT TESTS
  // =========================================================================

  #[test]
  fn test_zero_amount_deposit() {
    let mut engine = Engine::new();
    engine.process(deposit(1, 1, "0.0")).unwrap();

    let account = engine.accounts.get(&1).unwrap();
    assert_eq!(account.available, Decimal::ZERO);
    assert_eq!(account.total(), Decimal::ZERO);
  }

  #[test]
  fn test_zero_amount_withdrawal() {
    let mut engine = Engine::new();
    engine.process(deposit(1, 1, "100.0")).unwrap();
    engine.process(withdrawal(1, 2, "0.0")).unwrap();

    let account = engine.accounts.get(&1).unwrap();
    assert_eq!(account.available, Decimal::new(100, 0));
  }

  #[test]
  fn test_negative_deposit_rejected() {
    let mut engine = Engine::new();
    let result = engine.process(deposit(1, 1, "-100.0"));

    assert!(matches!(result, Err(EngineError::AccountError { .. })));
  }

  #[test]
  fn test_negative_withdrawal_rejected() {
    let mut engine = Engine::new();
    engine.process(deposit(1, 1, "100.0")).unwrap();
    let result = engine.process(withdrawal(1, 2, "-50.0"));

    assert!(matches!(result, Err(EngineError::AccountError { .. })));
  }

  #[test]
  fn test_redispute_after_resolve() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    engine.process(dispute(1, 1)).unwrap();
    engine.process(resolve(1, 1)).unwrap();
    // Should be able to dispute again
    engine.process(dispute(1, 1)).unwrap();

    let account = engine.accounts.get(&1).unwrap();
    assert_eq!(account.available, Decimal::ZERO);
    assert_eq!(account.held, Decimal::new(100, 0));
  }

  #[test]
  fn test_double_dispute_rejected() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    engine.process(dispute(1, 1)).unwrap();
    let result = engine.process(dispute(1, 1));

    assert!(matches!(result, Err(EngineError::AlreadyDisputed { .. })));
  }

  #[test]
  fn test_dispute_withdrawal_rejected() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    engine.process(withdrawal(1, 2, "50.0")).unwrap();
    let result = engine.process(dispute(1, 2));

    assert!(matches!(result, Err(EngineError::CannotDisputeWithdrawal { .. })));
  }

  #[test]
  fn test_chargeback_without_dispute_rejected() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    let result = engine.process(chargeback(1, 1));

    assert!(matches!(result, Err(EngineError::NotUnderDispute { .. })));
  }

  #[test]
  fn test_dispute_insufficient_funds() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    engine.process(withdrawal(1, 2, "80.0")).unwrap();
    // Try to dispute 100 when only 20 available
    let result = engine.process(dispute(1, 1));

    assert!(matches!(result, Err(EngineError::AccountError { .. })));
  }

  #[test]
  fn test_duplicate_tx_id_different_clients() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    // Same tx ID, different client - should fail
    let result = engine.process(deposit(2, 1, "200.0"));

    assert!(matches!(result, Err(EngineError::DuplicateTransaction { .. })));
  }

  #[test]
  fn test_withdrawal_creates_account() {
    let mut engine = Engine::new();

    // Withdrawal on non-existent account creates it then fails
    let result = engine.process(withdrawal(1, 1, "50.0"));
    assert!(matches!(result, Err(EngineError::AccountError { .. })));

    // Account should exist with zero balance
    let account = engine.accounts.get(&1).unwrap();
    assert_eq!(account.available, Decimal::ZERO);
  }

  #[test]
  fn test_dispute_on_locked_account() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    engine.process(deposit(1, 2, "50.0")).unwrap();
    engine.process(dispute(1, 1)).unwrap();
    engine.process(chargeback(1, 1)).unwrap();

    // Account is now locked, but dispute on tx 2 should still work
    engine.process(dispute(1, 2)).unwrap();

    let account = engine.accounts.get(&1).unwrap();
    assert!(account.locked);
    assert_eq!(account.available, Decimal::ZERO);
    assert_eq!(account.held, Decimal::new(50, 0));
  }

  #[test]
  fn test_resolve_on_locked_account() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();
    engine.process(deposit(1, 2, "50.0")).unwrap();
    engine.process(dispute(1, 2)).unwrap();
    engine.process(dispute(1, 1)).unwrap();
    engine.process(chargeback(1, 1)).unwrap();

    // Account is now locked, but resolve on tx 2 should still work
    engine.process(resolve(1, 2)).unwrap();

    let account = engine.accounts.get(&1).unwrap();
    assert!(account.locked);
    assert_eq!(account.available, Decimal::new(50, 0));
    assert_eq!(account.held, Decimal::ZERO);
  }

  #[test]
  fn test_transaction_id_zero() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 0, "100.0")).unwrap();
    engine.process(dispute(1, 0)).unwrap();

    let account = engine.accounts.get(&1).unwrap();
    assert_eq!(account.held, Decimal::new(100, 0));
  }

  #[test]
  fn test_transaction_id_max() {
    let mut engine = Engine::new();

    engine.process(deposit(1, u32::MAX, "100.0")).unwrap();
    engine.process(dispute(1, u32::MAX)).unwrap();

    let account = engine.accounts.get(&1).unwrap();
    assert_eq!(account.held, Decimal::new(100, 0));
  }

  #[test]
  fn test_client_id_zero() {
    let mut engine = Engine::new();

    engine.process(deposit(0, 1, "100.0")).unwrap();

    let account = engine.accounts.get(&0).unwrap();
    assert_eq!(account.available, Decimal::new(100, 0));
  }

  #[test]
  fn test_client_id_max() {
    let mut engine = Engine::new();

    engine.process(deposit(u16::MAX, 1, "100.0")).unwrap();

    let account = engine.accounts.get(&u16::MAX).unwrap();
    assert_eq!(account.available, Decimal::new(100, 0));
  }

  #[test]
  fn test_very_small_amount() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "0.0001")).unwrap();
    engine.process(deposit(1, 2, "0.0001")).unwrap();

    let account = engine.accounts.get(&1).unwrap();
    assert_eq!(account.available, Decimal::new(2, 4)); // 0.0002
  }

  #[test]
  fn test_precision_accumulation() {
    let mut engine = Engine::new();

    // 10 deposits of 0.0001 should equal exactly 0.001
    for i in 1..=10 {
      engine.process(deposit(1, i, "0.0001")).unwrap();
    }

    let account = engine.accounts.get(&1).unwrap();
    assert_eq!(account.available, Decimal::new(10, 4)); // 0.0010
  }

  #[test]
  fn test_multiple_dispute_resolve_cycles() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();

    // Multiple cycles
    for _ in 0..5 {
      engine.process(dispute(1, 1)).unwrap();
      let account = engine.accounts.get(&1).unwrap();
      assert_eq!(account.held, Decimal::new(100, 0));

      engine.process(resolve(1, 1)).unwrap();
      let account = engine.accounts.get(&1).unwrap();
      assert_eq!(account.available, Decimal::new(100, 0));
    }
  }

  #[test]
  fn test_missing_amount_deposit() {
    let mut engine = Engine::new();

    let record =
      TransactionRecord { tx_type: TransactionType::Deposit, client: 1, tx: 1, amount: None };
    let result = engine.process(record);

    assert!(matches!(result, Err(EngineError::MissingAmount { .. })));
  }

  #[test]
  fn test_missing_amount_withdrawal() {
    let mut engine = Engine::new();

    engine.process(deposit(1, 1, "100.0")).unwrap();

    let record =
      TransactionRecord { tx_type: TransactionType::Withdrawal, client: 1, tx: 2, amount: None };
    let result = engine.process(record);

    assert!(matches!(result, Err(EngineError::MissingAmount { .. })));
  }
}
