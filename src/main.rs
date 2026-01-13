mod account;
mod engine;
mod transaction;

use std::env;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::process;

use anyhow::{Context, Result};
use tracing::{Level, debug, error, info, warn};
use tracing_subscriber::EnvFilter;

use account::AccountOutput;
use engine::Engine;
use transaction::TransactionRecord;

/// THIS error file is created to log the ignored errors
const ERROR_FILE: &str = "errors.log";

fn main() {
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env().add_directive(Level::ERROR.into()))
    .with_writer(io::stderr)
    .init();

  if let Err(e) = run() {
    error!("Fatal error: {e:?}");
    eprintln!("Error: {e:?}");
    process::exit(1);
  }
}

fn run() -> Result<()> {
  let args: Vec<String> = env::args().collect();

  if args.len() != 2 {
    eprintln!("Usage: {} <transactions.csv>", args[0]);
    process::exit(1);
  }

  let input_path = &args[1];

  info!(input = %input_path, "Starting transaction processing");

  // Open the input file
  let file = File::open(input_path).with_context(|| format!("Failed to open '{}'", input_path))?;
  let reader = BufReader::new(file);
  debug!(path = %input_path, "Opened input file");

  // Create the error file, fall back to sink if it fails
  let mut error_writer: Box<dyn Write> = match File::create(ERROR_FILE) {
    Ok(file) => {
      debug!(path = %ERROR_FILE, "Writing errors to file");
      Box::new(BufWriter::new(file))
    }
    Err(e) => {
      debug!(error = %e, "Cannot create error file, ignoring errors");
      Box::new(io::sink())
    }
  };

  let mut csv_reader =
    csv::ReaderBuilder::new().trim(csv::Trim::All).flexible(true).from_reader(reader);

  let mut engine = Engine::new();

  for result in csv_reader.deserialize::<TransactionRecord>() {
    match result {
      Ok(record) => {
        debug!(tx = record.tx, client = record.client, "Processing transaction");
        if let Err(e) = engine.process(record) {
          warn!(error = %e, "Transaction processing failed");
          let _ = writeln!(error_writer, "{}", e);
        }
      }
      Err(e) => {
        warn!(error = %e, "Failed to parse record");
        let _ = writeln!(error_writer, "Failed to parse record: {}", e);
      }
    }
  }

  let _ = error_writer.flush();

  // Output account states
  write_output(&engine)?;

  Ok(())
}

fn write_output(engine: &Engine) -> Result<usize> {
  let stdout = io::stdout();
  let mut handle = stdout.lock();

  // the csv header
  writeln!(handle, "client,available,held,total,locked")?;

  // Since we have a u16, we can sort the accounts with reasonably low overhead
  let mut accounts: Vec<AccountOutput> = engine.accounts().map(AccountOutput::from).collect();
  accounts.sort_by_key(|a| a.client);

  let count = accounts.len();

  for account in accounts {
    writeln!(
      handle,
      "{},{},{},{},{}",
      account.client,
      format_decimal(account.available),
      format_decimal(account.held),
      format_decimal(account.total),
      account.locked
    )?;
  }

  Ok(count)
}

///  Per the spec "You can assume a precision of 4 places past the decimal"
fn format_decimal(d: rust_decimal::Decimal) -> String {
  format!("{:.4}", d)
}

/// AI GENERATED TESTS
/// PROMPT:  generate the necessary tests to verify the functionality in main.rs1
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_format_decimal() {
    use rust_decimal::Decimal;

    assert_eq!(format_decimal(Decimal::new(15, 1)), "1.5000");
    assert_eq!(format_decimal(Decimal::new(100, 0)), "100.0000");
    assert_eq!(format_decimal(Decimal::new(12345, 4)), "1.2345");
    assert_eq!(format_decimal(Decimal::new(10000, 4)), "1.0000");
  }
}
