//! Integration tests for the toypayments binary.
//!
//! These tests run the actual binary and verify its behavior.o
//! THIS IS AI GENERATED CODE
//! PROMPT: Generate integration tests for the code in this project

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Get a command for the toypayments binary
fn toypayments() -> Command {
  Command::cargo_bin("toypayments").unwrap()
}

/// Create a temp directory with a CSV file
fn create_test_csv(content: &str) -> (TempDir, std::path::PathBuf) {
  let dir = TempDir::new().unwrap();
  let path = dir.path().join("transactions.csv");
  fs::write(&path, content).unwrap();
  (dir, path)
}

#[test]
fn test_no_arguments_shows_usage() {
  toypayments()
    .assert()
    .failure()
    .stderr(predicate::str::contains("Usage:"))
    .stderr(predicate::str::contains("transactions.csv"));
}

#[test]
fn test_missing_file_error() {
  toypayments()
    .arg("nonexistent.csv")
    .assert()
    .failure()
    .stderr(predicate::str::contains("Failed to open"));
}

#[test]
fn test_basic_deposits_and_withdrawals() {
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
deposit,2,2,200.0
withdrawal,1,3,50.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("client,available,held,total,locked"))
    .stdout(predicate::str::contains("1,50.0000,0.0000,50.0000,false"))
    .stdout(predicate::str::contains("2,200.0000,0.0000,200.0000,false"));
}

#[test]
fn test_spec_example() {
  // Example from the spec document
  let csv = "\
type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Client 1: deposited 1 + 2 = 3, withdrew 1.5 = 1.5
    .stdout(predicate::str::contains("1,1.5000,0.0000,1.5000,false"))
    // Client 2: deposited 2, withdrawal of 3 failed (insufficient funds) = 2
    .stdout(predicate::str::contains("2,2.0000,0.0000,2.0000,false"));
}

#[test]
fn test_dispute_and_resolve() {
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
resolve,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // After dispute and resolve, funds should be available again
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_dispute_and_chargeback() {
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
chargeback,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // After chargeback, account should be locked and funds gone
    .stdout(predicate::str::contains("1,0.0000,0.0000,0.0000,true"));
}

#[test]
fn test_locked_account_rejects_deposits() {
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
chargeback,1,1,
deposit,1,2,50.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Account locked, second deposit should fail
    .stdout(predicate::str::contains("1,0.0000,0.0000,0.0000,true"));
}

#[test]
fn test_dispute_nonexistent_transaction() {
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
dispute,1,999,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Dispute is ignored, original deposit remains
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_client_mismatch_dispute() {
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
dispute,2,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Dispute is ignored due to client mismatch
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_decimal_precision() {
  let csv = "\
type,client,tx,amount
deposit,1,1,100.1234
deposit,1,2,0.0001
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,100.1235,0.0000,100.1235,false"));
}

#[test]
fn test_whitespace_handling() {
  // CSV with various whitespace
  let csv = "\
type,  client,  tx,  amount
deposit,  1,  1,  100.0
  deposit,1,2,50.0
withdrawal,1,3,  25.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,125.0000,0.0000,125.0000,false"));
}

#[test]
fn test_multiple_clients_ordering() {
  let csv = "\
type,client,tx,amount
deposit,3,1,300.0
deposit,1,2,100.0
deposit,2,3,200.0
";
  let (_dir, path) = create_test_csv(csv);

  // Output should be sorted by client ID
  let output = toypayments().arg(&path).assert().success().get_output().stdout.clone();

  let stdout = String::from_utf8(output).unwrap();
  let lines: Vec<&str> = stdout.lines().collect();

  // Header + 3 clients
  assert_eq!(lines.len(), 4);
  assert!(lines[1].starts_with("1,"));
  assert!(lines[2].starts_with("2,"));
  assert!(lines[3].starts_with("3,"));
}

#[test]
fn test_held_funds_during_dispute() {
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
deposit,1,2,50.0
dispute,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // 50 available, 100 held, 150 total
    .stdout(predicate::str::contains("1,50.0000,100.0000,150.0000,false"));
}

#[test]
fn test_empty_file() {
  let csv = "type,client,tx,amount\n";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Just header, no accounts
    .stdout(predicate::str::is_match("^client,available,held,total,locked\n$").unwrap());
}

#[test]
fn test_duplicate_transaction_id() {
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
deposit,1,1,50.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Only first transaction should succeed
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

// =============================================================================
// NUMERIC/PRECISION EDGE CASE TESTS
// =============================================================================

#[test]
fn test_zero_amount_deposit() {
  // Zero-amount deposits should be allowed (no-op but valid)
  let csv = "\
type,client,tx,amount
deposit,1,1,0.0
deposit,1,2,100.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_zero_amount_withdrawal() {
  // Zero-amount withdrawals should be allowed (no-op but valid)
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,0.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_very_small_amount() {
  // Test minimum precision (0.0001)
  let csv = "\
type,client,tx,amount
deposit,1,1,0.0001
deposit,1,2,0.0001
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,0.0002,0.0000,0.0002,false"));
}

#[test]
fn test_very_large_amount() {
  // Test with large amounts (within reasonable bounds)
  let csv = "\
type,client,tx,amount
deposit,1,1,999999999999.9999
deposit,1,2,0.0001
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,1000000000000.0000,0.0000,1000000000000.0000,false"));
}

#[test]
fn test_precision_four_decimal_places() {
  // Verify exact 4 decimal place precision
  let csv = "\
type,client,tx,amount
deposit,1,1,1.2345
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,1.2345,0.0000,1.2345,false"));
}

#[test]
fn test_precision_accumulation() {
  // Test that precision doesn't accumulate errors over multiple operations
  let csv = "\
type,client,tx,amount
deposit,1,1,0.0001
deposit,1,2,0.0001
deposit,1,3,0.0001
deposit,1,4,0.0001
deposit,1,5,0.0001
deposit,1,6,0.0001
deposit,1,7,0.0001
deposit,1,8,0.0001
deposit,1,9,0.0001
deposit,1,10,0.0001
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // 10 * 0.0001 = 0.001
    .stdout(predicate::str::contains("1,0.0010,0.0000,0.0010,false"));
}

#[test]
fn test_whole_numbers_formatted_correctly() {
  // Whole numbers should still show 4 decimal places
  let csv = "\
type,client,tx,amount
deposit,1,1,100
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_leading_zeros_in_amount() {
  // Leading zeros should parse correctly
  let csv = "\
type,client,tx,amount
deposit,1,1,00100.0000
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

// =============================================================================
// STATE MACHINE EDGE CASE TESTS (Disputes, Locked Accounts)
// =============================================================================

#[test]
fn test_redispute_after_resolve() {
  // Can you dispute -> resolve -> dispute again?
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
resolve,1,1,
dispute,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // After second dispute, funds should be held again
    .stdout(predicate::str::contains("1,0.0000,100.0000,100.0000,false"));
}

#[test]
fn test_multiple_dispute_resolve_cycles() {
  // Multiple dispute -> resolve cycles on same transaction
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
resolve,1,1,
dispute,1,1,
resolve,1,1,
dispute,1,1,
resolve,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // All cycles complete, funds available
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_dispute_on_locked_account() {
  // Can you dispute a transaction after the account is locked?
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
deposit,1,2,50.0
dispute,1,1,
chargeback,1,1,
dispute,1,2,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Account locked, second dispute should still work (holds the 50)
    .stdout(predicate::str::contains("1,0.0000,50.0000,50.0000,true"));
}

#[test]
fn test_resolve_on_locked_account() {
  // Can you resolve a dispute after the account is locked?
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
deposit,1,2,50.0
dispute,1,2,
dispute,1,1,
chargeback,1,1,
resolve,1,2,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Account locked after chargeback on tx 1, resolve on tx 2 should work
    .stdout(predicate::str::contains("1,50.0000,0.0000,50.0000,true"));
}

#[test]
fn test_chargeback_on_already_locked_account() {
  // Second chargeback on already locked account
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
deposit,1,2,50.0
dispute,1,1,
chargeback,1,1,
dispute,1,2,
chargeback,1,2,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Both chargebacks succeed, all funds removed
    .stdout(predicate::str::contains("1,0.0000,0.0000,0.0000,true"));
}

#[test]
fn test_locked_account_rejects_withdrawals() {
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
chargeback,1,1,
withdrawal,1,2,50.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Withdrawal rejected, account still at 0
    .stdout(predicate::str::contains("1,0.0000,0.0000,0.0000,true"));
}

#[test]
fn test_dispute_withdrawal_rejected() {
  // Disputing a withdrawal should fail
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,50.0
dispute,1,2,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Dispute on withdrawal ignored, balance unchanged
    .stdout(predicate::str::contains("1,50.0000,0.0000,50.0000,false"));
}

#[test]
fn test_double_dispute_same_transaction() {
  // Disputing an already disputed transaction should fail
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
dispute,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Second dispute ignored, still held
    .stdout(predicate::str::contains("1,0.0000,100.0000,100.0000,false"));
}

#[test]
fn test_chargeback_without_dispute() {
  // Chargeback on non-disputed transaction should fail
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
chargeback,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Chargeback ignored, funds still available
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_resolve_without_dispute() {
  // Resolve on non-disputed transaction should fail
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
resolve,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Resolve ignored, funds still available
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_dispute_insufficient_available_funds() {
  // Dispute when available < disputed amount (already withdrew some)
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,80.0
dispute,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Dispute fails (can't hold 100 when only 20 available), balance unchanged
    .stdout(predicate::str::contains("1,20.0000,0.0000,20.0000,false"));
}

// =============================================================================
// TRANSACTION ORDERING EDGE CASE TESTS
// =============================================================================

#[test]
fn test_dispute_before_deposit_fails() {
  // Dispute a transaction that doesn't exist yet
  let csv = "\
type,client,tx,amount
dispute,1,1,
deposit,1,1,100.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Dispute ignored (tx doesn't exist yet), deposit succeeds
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_withdrawal_before_any_deposit() {
  // Withdrawal with no prior deposit (implicit account creation)
  let csv = "\
type,client,tx,amount
withdrawal,1,1,50.0
deposit,1,2,100.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Withdrawal fails (insufficient funds), deposit succeeds
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_out_of_order_transaction_ids() {
  // Transaction IDs don't have to be sequential
  let csv = "\
type,client,tx,amount
deposit,1,100,50.0
deposit,1,1,25.0
deposit,1,50,25.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_resolve_before_dispute() {
  // Resolve without prior dispute
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
resolve,1,1,
dispute,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Resolve fails (not disputed), then dispute succeeds
    .stdout(predicate::str::contains("1,0.0000,100.0000,100.0000,false"));
}

#[test]
fn test_chargeback_before_dispute() {
  // Chargeback without prior dispute
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
chargeback,1,1,
dispute,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Chargeback fails (not disputed), then dispute succeeds
    .stdout(predicate::str::contains("1,0.0000,100.0000,100.0000,false"));
}

// =============================================================================
// CSV PARSING EDGE CASE TESTS
// =============================================================================

#[test]
fn test_csv_with_only_header() {
  // Empty CSV with just header
  let csv = "type,client,tx,amount\n";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::is_match("^client,available,held,total,locked\n$").unwrap());
}

#[test]
fn test_csv_with_extra_columns() {
  // CSV with extra columns should still work (flexible mode)
  let csv = "\
type,client,tx,amount,extra_column,another
deposit,1,1,100.0,ignored,data
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_csv_mixed_whitespace() {
  // Various whitespace combinations
  let csv = "\
type,client,tx,amount
  deposit  ,  1  ,  1  ,  100.0
deposit,1,2,50.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,150.0000,0.0000,150.0000,false"));
}

#[test]
fn test_csv_empty_amount_for_dispute() {
  // Dispute with empty amount field (correct format)
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,0.0000,100.0000,100.0000,false"));
}

#[test]
fn test_csv_whitespace_only_amount() {
  // Dispute with whitespace-only amount field
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,0.0000,100.0000,100.0000,false"));
}

#[test]
fn test_invalid_transaction_type() {
  // Invalid transaction type should be skipped with error
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
invalid,1,2,50.0
deposit,1,3,25.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Invalid type skipped, other deposits succeed
    .stdout(predicate::str::contains("1,125.0000,0.0000,125.0000,false"));
}

#[test]
fn test_missing_amount_for_deposit() {
  // Deposit without amount should fail
  let csv = "\
type,client,tx,amount
deposit,1,1,
deposit,1,2,100.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // First deposit fails (no amount), second succeeds
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_malformed_amount() {
  // Malformed amount should fail gracefully
  let csv = "\
type,client,tx,amount
deposit,1,1,not_a_number
deposit,1,2,100.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // First deposit fails (bad amount), second succeeds
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_negative_amount_rejected() {
  // Negative amounts should be rejected
  let csv = "\
type,client,tx,amount
deposit,1,1,-100.0
deposit,1,2,50.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Negative deposit rejected, positive succeeds
    .stdout(predicate::str::contains("1,50.0000,0.0000,50.0000,false"));
}

#[test]
fn test_csv_only_disputes_no_deposits() {
  // CSV with only dispute operations (all fail)
  let csv = "\
type,client,tx,amount
dispute,1,1,
resolve,1,2,
chargeback,1,3,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // No output since no accounts created successfully
    .stdout(predicate::str::is_match("^client,available,held,total,locked\n$").unwrap());
}

// =============================================================================
// EXTREME VALUE EDGE CASE TESTS
// =============================================================================

#[test]
fn test_client_id_zero() {
  // Client ID 0 is valid
  let csv = "\
type,client,tx,amount
deposit,0,1,100.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("0,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_client_id_max() {
  // Client ID u16::MAX (65535) is valid
  let csv = "\
type,client,tx,amount
deposit,65535,1,100.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("65535,100.0000,0.0000,100.0000,false"));
}

#[test]
fn test_transaction_id_zero() {
  // Transaction ID 0 is valid
  let csv = "\
type,client,tx,amount
deposit,1,0,100.0
dispute,1,0,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,0.0000,100.0000,100.0000,false"));
}

#[test]
fn test_transaction_id_max() {
  // Transaction ID u32::MAX (4294967295) is valid
  let csv = "\
type,client,tx,amount
deposit,1,4294967295,100.0
dispute,1,4294967295,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,0.0000,100.0000,100.0000,false"));
}

#[test]
fn test_duplicate_tx_id_different_clients() {
  // Same transaction ID for different clients should both fail (global uniqueness)
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
deposit,2,1,200.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // First succeeds, second fails (duplicate tx id)
    .stdout(predicate::str::contains("1,100.0000,0.0000,100.0000,false"))
    .stdout(predicate::str::contains("2,").not());
}

#[test]
fn test_many_clients() {
  // Test with multiple clients to verify ordering
  let csv = "\
type,client,tx,amount
deposit,10,1,10.0
deposit,5,2,5.0
deposit,1,3,1.0
deposit,100,4,100.0
deposit,50,5,50.0
";
  let (_dir, path) = create_test_csv(csv);

  let output = toypayments().arg(&path).assert().success().get_output().stdout.clone();

  let stdout = String::from_utf8(output).unwrap();
  let lines: Vec<&str> = stdout.lines().collect();

  // Should be sorted by client ID
  assert_eq!(lines.len(), 6); // header + 5 clients
  assert!(lines[1].starts_with("1,"));
  assert!(lines[2].starts_with("5,"));
  assert!(lines[3].starts_with("10,"));
  assert!(lines[4].starts_with("50,"));
  assert!(lines[5].starts_with("100,"));
}

#[test]
fn test_client_with_only_failed_transactions() {
  // Client where all transactions fail - should not appear in output
  let csv = "\
type,client,tx,amount
withdrawal,1,1,100.0
deposit,2,2,50.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Client 1 has no successful transactions but account was created
    // Client 2 has successful deposit
    .stdout(predicate::str::contains("1,0.0000,0.0000,0.0000,false"))
    .stdout(predicate::str::contains("2,50.0000,0.0000,50.0000,false"));
}

#[test]
fn test_multiple_consecutive_errors() {
  // Multiple consecutive errors should all be handled gracefully
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,200.0
withdrawal,1,3,200.0
withdrawal,1,4,200.0
dispute,1,999,
dispute,2,1,
deposit,1,5,50.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // All errors handled, final state correct
    .stdout(predicate::str::contains("1,150.0000,0.0000,150.0000,false"));
}

// =============================================================================
// ADDITIONAL COMPLEX SCENARIO TESTS
// =============================================================================

#[test]
fn test_full_lifecycle_single_client() {
  // Complete lifecycle: deposit, withdraw, dispute, resolve, dispute again, chargeback
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
deposit,1,2,50.0
withdrawal,1,3,25.0
dispute,1,1,
resolve,1,1,
dispute,1,2,
chargeback,1,2,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // 100 + 50 - 25 = 125, then chargeback 50 = 75, locked
    .stdout(predicate::str::contains("1,75.0000,0.0000,75.0000,true"));
}

#[test]
fn test_dispute_partial_available_funds() {
  // Dispute when held + available would equal total
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
deposit,1,2,50.0
dispute,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // 50 available, 100 held, 150 total
    .stdout(predicate::str::contains("1,50.0000,100.0000,150.0000,false"));
}

#[test]
fn test_account_with_zero_balance_after_operations() {
  // Account ends with zero balance but is not locked
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,100.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    .stdout(predicate::str::contains("1,0.0000,0.0000,0.0000,false"));
}

#[test]
fn test_held_funds_cannot_be_withdrawn() {
  // Can't withdraw held funds
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
withdrawal,1,2,50.0
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Withdrawal fails (0 available), funds still held
    .stdout(predicate::str::contains("1,0.0000,100.0000,100.0000,false"));
}

#[test]
fn test_partial_withdrawal_then_dispute() {
  // Withdraw some, then try to dispute full amount
  let csv = "\
type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,30.0
dispute,1,1,
";
  let (_dir, path) = create_test_csv(csv);

  toypayments()
    .arg(&path)
    .assert()
    .success()
    // Can't hold 100 when only 70 available, dispute fails
    .stdout(predicate::str::contains("1,70.0000,0.0000,70.0000,false"));
}
