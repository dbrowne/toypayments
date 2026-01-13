use rust_decimal::Decimal;
use serde::Serialize;
use thiserror::Error;

/// The account as described in the problem  using rust_decimal to avoid rounding errors
/// and to also avoid overflow since we probably won't have octillion dollar balances
/// in the test case
#[derive(Debug, Clone)]
pub struct Account {
  pub client: u16,
  pub available: Decimal,
  pub held: Decimal,
  pub locked: bool,
}

impl Account {
  pub fn new(client: u16) -> Self {
    Self { client, available: Decimal::ZERO, held: Decimal::ZERO, locked: false }
  }

  pub fn total(&self) -> Decimal {
    self.available + self.held
  }

  pub fn deposit(&mut self, amount: Decimal) -> Result<(), AccountError> {
    if self.locked {
      return Err(AccountError::AccountLocked);
    }
    if amount < Decimal::ZERO {
      return Err(AccountError::NegativeAmount);
    }
    self.available += amount;
    Ok(())
  }
  pub fn withdraw(&mut self, amount: Decimal) -> Result<(), AccountError> {
    if self.locked {
      return Err(AccountError::AccountLocked);
    }
    if amount < Decimal::ZERO {
      return Err(AccountError::NegativeAmount);
    }
    if self.available < amount {
      return Err(AccountError::InsufficientFunds { requested: amount, available: self.available });
    }
    self.available -= amount;
    Ok(())
  }

  pub fn hold(&mut self, amount: Decimal) -> Result<(), AccountError> {
    if amount < Decimal::ZERO {
      return Err(AccountError::NegativeAmount);
    }
    if self.available < amount {
      return Err(AccountError::InsufficientFunds { requested: amount, available: self.available });
    }
    self.available -= amount;
    self.held += amount;
    Ok(())
  }

  pub fn release(&mut self, amount: Decimal) -> Result<(), AccountError> {
    if amount < Decimal::ZERO {
      return Err(AccountError::NegativeAmount);
    }
    if self.held < amount {
      return Err(AccountError::InsufficientHeldFunds { requested: amount, held: self.held });
    }
    self.held -= amount;
    self.available += amount;
    Ok(())
  }

  pub fn chargeback(&mut self, amount: Decimal) -> Result<(), AccountError> {
    if amount < Decimal::ZERO {
      return Err(AccountError::NegativeAmount);
    }
    if self.held < amount {
      return Err(AccountError::InsufficientHeldFunds { requested: amount, held: self.held });
    }
    self.held -= amount;
    self.locked = true;
    Ok(())
  }
}

/// THIS IS AI generated after initial testing
/// PROMPT: implement error handling using thiserror
#[derive(Debug, Clone, PartialEq, Error)]
pub enum AccountError {
  #[error("account is locked")]
  AccountLocked,
  #[error("negative amount not allowed")]
  NegativeAmount,
  #[error("insufficient funds: requested {requested}, available {available}")]
  InsufficientFunds { requested: Decimal, available: Decimal },
  #[error("insufficient held funds: requested {requested}, held {held}")]
  InsufficientHeldFunds { requested: Decimal, held: Decimal },
}

/// THIS IS HUMAN CREATED code
#[derive(Debug, Serialize)]
pub struct AccountOutput {
  pub client: u16,
  pub available: Decimal,
  pub held: Decimal,
  pub total: Decimal,
  pub locked: bool,
}

impl From<&Account> for AccountOutput {
  fn from(account: &Account) -> Self {
    Self {
      client: account.client,
      available: account.available,
      held: account.held,
      total: account.total(),
      locked: account.locked,
    }
  }
}

/// ALL TESTS WERE AI GENERATED
/// PROMPT:   Generate a complete set of test cases for the  Account implementation
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_deposit() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    assert_eq!(account.available, Decimal::new(100, 0));
    assert_eq!(account.total(), Decimal::new(100, 0));
  }

  #[test]
  fn test_withdraw_success() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.withdraw(Decimal::new(50, 0)).unwrap();
    assert_eq!(account.available, Decimal::new(50, 0));
  }

  #[test]
  fn test_withdraw_insufficient_funds() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(50, 0)).unwrap();
    let result = account.withdraw(Decimal::new(100, 0));
    assert!(matches!(result, Err(AccountError::InsufficientFunds { .. })));
  }

  #[test]
  fn test_hold_and_release() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(30, 0)).unwrap();

    assert_eq!(account.available, Decimal::new(70, 0));
    assert_eq!(account.held, Decimal::new(30, 0));
    assert_eq!(account.total(), Decimal::new(100, 0));

    account.release(Decimal::new(30, 0)).unwrap();
    assert_eq!(account.available, Decimal::new(100, 0));
    assert_eq!(account.held, Decimal::ZERO);
  }

  #[test]
  fn test_chargeback_locks_account() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(30, 0)).unwrap();
    account.chargeback(Decimal::new(30, 0)).unwrap();

    assert!(account.locked);
    assert_eq!(account.held, Decimal::ZERO);
    assert_eq!(account.total(), Decimal::new(70, 0));
  }

  #[test]
  fn test_locked_account_rejects_operations() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(30, 0)).unwrap();
    account.chargeback(Decimal::new(30, 0)).unwrap();

    assert!(matches!(account.deposit(Decimal::new(10, 0)), Err(AccountError::AccountLocked)));
    assert!(matches!(account.withdraw(Decimal::new(10, 0)), Err(AccountError::AccountLocked)));
  }

  // =========================================================================
  // EDGE CASE UNIT TESTS
  // =========================================================================

  #[test]
  fn test_zero_amount_deposit() {
    let mut account = Account::new(1);
    account.deposit(Decimal::ZERO).unwrap();
    assert_eq!(account.available, Decimal::ZERO);
    assert_eq!(account.total(), Decimal::ZERO);
  }

  #[test]
  fn test_zero_amount_withdrawal() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.withdraw(Decimal::ZERO).unwrap();
    assert_eq!(account.available, Decimal::new(100, 0));
  }

  #[test]
  fn test_zero_amount_hold() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::ZERO).unwrap();
    assert_eq!(account.available, Decimal::new(100, 0));
    assert_eq!(account.held, Decimal::ZERO);
  }

  #[test]
  fn test_zero_amount_release() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(50, 0)).unwrap();
    account.release(Decimal::ZERO).unwrap();
    assert_eq!(account.available, Decimal::new(50, 0));
    assert_eq!(account.held, Decimal::new(50, 0));
  }

  #[test]
  fn test_zero_amount_chargeback() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(50, 0)).unwrap();
    account.chargeback(Decimal::ZERO).unwrap();
    // Account gets locked even with zero chargeback
    assert!(account.locked);
    assert_eq!(account.held, Decimal::new(50, 0));
  }

  #[test]
  fn test_negative_deposit_rejected() {
    let mut account = Account::new(1);
    let result = account.deposit(Decimal::new(-100, 0));
    assert!(matches!(result, Err(AccountError::NegativeAmount)));
  }

  #[test]
  fn test_negative_withdrawal_rejected() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    let result = account.withdraw(Decimal::new(-50, 0));
    assert!(matches!(result, Err(AccountError::NegativeAmount)));
  }

  #[test]
  fn test_negative_hold_rejected() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    let result = account.hold(Decimal::new(-50, 0));
    assert!(matches!(result, Err(AccountError::NegativeAmount)));
  }

  #[test]
  fn test_negative_release_rejected() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(50, 0)).unwrap();
    let result = account.release(Decimal::new(-25, 0));
    assert!(matches!(result, Err(AccountError::NegativeAmount)));
  }

  #[test]
  fn test_negative_chargeback_rejected() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(50, 0)).unwrap();
    let result = account.chargeback(Decimal::new(-25, 0));
    assert!(matches!(result, Err(AccountError::NegativeAmount)));
  }

  #[test]
  fn test_withdraw_exact_balance() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.withdraw(Decimal::new(100, 0)).unwrap();
    assert_eq!(account.available, Decimal::ZERO);
    assert_eq!(account.total(), Decimal::ZERO);
  }

  #[test]
  fn test_hold_exact_available() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(100, 0)).unwrap();
    assert_eq!(account.available, Decimal::ZERO);
    assert_eq!(account.held, Decimal::new(100, 0));
    assert_eq!(account.total(), Decimal::new(100, 0));
  }

  #[test]
  fn test_release_exact_held() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(100, 0)).unwrap();
    account.release(Decimal::new(100, 0)).unwrap();
    assert_eq!(account.available, Decimal::new(100, 0));
    assert_eq!(account.held, Decimal::ZERO);
  }

  #[test]
  fn test_chargeback_exact_held() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(100, 0)).unwrap();
    account.chargeback(Decimal::new(100, 0)).unwrap();
    assert_eq!(account.available, Decimal::ZERO);
    assert_eq!(account.held, Decimal::ZERO);
    assert_eq!(account.total(), Decimal::ZERO);
    assert!(account.locked);
  }

  #[test]
  fn test_hold_more_than_available_rejected() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    let result = account.hold(Decimal::new(150, 0));
    assert!(matches!(result, Err(AccountError::InsufficientFunds { .. })));
  }

  #[test]
  fn test_release_more_than_held_rejected() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(50, 0)).unwrap();
    let result = account.release(Decimal::new(100, 0));
    assert!(matches!(result, Err(AccountError::InsufficientHeldFunds { .. })));
  }

  #[test]
  fn test_chargeback_more_than_held_rejected() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(50, 0)).unwrap();
    let result = account.chargeback(Decimal::new(100, 0));
    assert!(matches!(result, Err(AccountError::InsufficientHeldFunds { .. })));
  }

  #[test]
  fn test_very_small_amount_precision() {
    let mut account = Account::new(1);
    // 0.0001
    let small = Decimal::new(1, 4);
    account.deposit(small).unwrap();
    account.deposit(small).unwrap();
    assert_eq!(account.available, Decimal::new(2, 4)); // 0.0002
  }

  #[test]
  fn test_precision_accumulation() {
    let mut account = Account::new(1);
    let small = Decimal::new(1, 4); // 0.0001

    for _ in 0..10 {
      account.deposit(small).unwrap();
    }

    assert_eq!(account.available, Decimal::new(10, 4)); // 0.0010
  }

  #[test]
  fn test_total_invariant() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();

    // available + held should always equal total
    assert_eq!(account.available + account.held, account.total());

    account.hold(Decimal::new(30, 0)).unwrap();
    assert_eq!(account.available + account.held, account.total());

    account.release(Decimal::new(10, 0)).unwrap();
    assert_eq!(account.available + account.held, account.total());

    account.chargeback(Decimal::new(20, 0)).unwrap();
    assert_eq!(account.available + account.held, account.total());
  }

  #[test]
  fn test_locked_account_allows_hold() {
    // Locked accounts should still allow hold operations (for disputes)
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(50, 0)).unwrap();
    account.chargeback(Decimal::new(50, 0)).unwrap();

    // Account is locked, but hold should still work
    account.hold(Decimal::new(25, 0)).unwrap();
    assert_eq!(account.held, Decimal::new(25, 0));
  }

  #[test]
  fn test_locked_account_allows_release() {
    // Locked accounts should still allow release operations (for resolves)
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(50, 0)).unwrap();
    account.chargeback(Decimal::new(25, 0)).unwrap();

    // Account is locked, but release should still work
    account.release(Decimal::new(25, 0)).unwrap();
    assert_eq!(account.available, Decimal::new(75, 0));
    assert_eq!(account.held, Decimal::ZERO);
  }

  #[test]
  fn test_locked_account_allows_chargeback() {
    // Locked accounts should still allow chargeback operations
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(50, 0)).unwrap();
    account.chargeback(Decimal::new(25, 0)).unwrap();

    // Account is locked, but another chargeback should still work
    account.chargeback(Decimal::new(25, 0)).unwrap();
    assert_eq!(account.held, Decimal::ZERO);
  }

  #[test]
  fn test_multiple_hold_release_cycles() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();

    for _ in 0..5 {
      account.hold(Decimal::new(50, 0)).unwrap();
      assert_eq!(account.held, Decimal::new(50, 0));
      assert_eq!(account.available, Decimal::new(50, 0));

      account.release(Decimal::new(50, 0)).unwrap();
      assert_eq!(account.held, Decimal::ZERO);
      assert_eq!(account.available, Decimal::new(100, 0));
    }
  }

  #[test]
  fn test_client_id_stored() {
    let account = Account::new(12345);
    assert_eq!(account.client, 12345);
  }

  #[test]
  fn test_new_account_not_locked() {
    let account = Account::new(1);
    assert!(!account.locked);
  }

  #[test]
  fn test_account_output_conversion() {
    let mut account = Account::new(42);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(30, 0)).unwrap();

    let output = AccountOutput::from(&account);
    assert_eq!(output.client, 42);
    assert_eq!(output.available, Decimal::new(70, 0));
    assert_eq!(output.held, Decimal::new(30, 0));
    assert_eq!(output.total, Decimal::new(100, 0));
    assert!(!output.locked);
  }

  #[test]
  fn test_account_output_locked() {
    let mut account = Account::new(1);
    account.deposit(Decimal::new(100, 0)).unwrap();
    account.hold(Decimal::new(100, 0)).unwrap();
    account.chargeback(Decimal::new(100, 0)).unwrap();

    let output = AccountOutput::from(&account);
    assert!(output.locked);
    assert_eq!(output.total, Decimal::ZERO);
  }
}
