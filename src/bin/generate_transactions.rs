use rand::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::process;
/// THIS IS AI GENERATED FOR THE SAKE OF CREATING CSV FILES FOR TESTING
/// Configuration for the transaction generator
#[derive(Debug, Deserialize)]
struct Config {
  accounts: AccountsConfig,
  transactions: TransactionsConfig,
  amounts: AmountsConfig,
  withdrawals: WithdrawalsConfig,
  disputes: DisputesConfig,
  output: OutputConfig,
}

#[derive(Debug, Deserialize)]
struct AccountsConfig {
  count: u16,
}

#[derive(Debug, Deserialize)]
struct TransactionsConfig {
  min_per_account: u32,
  max_per_account: u32,
}

#[derive(Debug, Deserialize)]
struct AmountsConfig {
  min: f64,
  max: f64,
  precision: u8,
}

#[derive(Debug, Deserialize)]
struct WithdrawalsConfig {
  probability: f64,
  overdraw_probability: f64,
}

#[derive(Debug, Deserialize)]
struct DisputesConfig {
  probability: f64,
  resolution_probability: f64,
}

#[derive(Debug, Deserialize)]
struct OutputConfig {
  file: String,
  seed: Option<u64>,
}

/// Tracks state for each account during generation
#[derive(Debug, Default)]
struct AccountState {
  available: f64,
  deposits: Vec<(u32, f64)>, // (tx_id, amount) - for potential disputes
}

/// A generated transaction
#[derive(Debug)]
enum Transaction {
  Deposit { client: u16, tx: u32, amount: f64 },
  Withdrawal { client: u16, tx: u32, amount: f64 },
  Dispute { client: u16, tx: u32 },
  Resolve { client: u16, tx: u32 },
  Chargeback { client: u16, tx: u32 },
}

impl Transaction {
  fn to_csv_row(&self, precision: u8) -> String {
    match self {
      Transaction::Deposit { client, tx, amount } => {
        format!("deposit,{},{},{:.prec$}", client, tx, amount, prec = precision as usize)
      }
      Transaction::Withdrawal { client, tx, amount } => {
        format!("withdrawal,{},{},{:.prec$}", client, tx, amount, prec = precision as usize)
      }
      Transaction::Dispute { client, tx } => {
        format!("dispute,{},{},", client, tx)
      }
      Transaction::Resolve { client, tx } => {
        format!("resolve,{},{},", client, tx)
      }
      Transaction::Chargeback { client, tx } => {
        format!("chargeback,{},{},", client, tx)
      }
    }
  }
}

fn main() {
  if let Err(e) = run() {
    eprintln!("Error: {}", e);
    process::exit(1);
  }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
  let args: Vec<String> = env::args().collect();

  let config_path =
    if args.len() >= 2 { args[1].clone() } else { "generator_params.toml".to_string() };
  let config_content = fs::read_to_string(&config_path)
    .map_err(|e| format!("Failed to read '{}': {}", config_path, e))?;

  let config: Config =
    toml::from_str(&config_content).map_err(|e| format!("Failed to parse config: {}", e))?;

  validate_config(&config)?;

  let transactions = generate_transactions(&config);

  write_output(&config, &transactions)?;

  eprintln!("Generated {} transactions for {} accounts", transactions.len(), config.accounts.count);

  Ok(())
}

fn validate_config(config: &Config) -> Result<(), String> {
  if config.accounts.count == 0 {
    return Err("accounts.count must be > 0".to_string());
  }
  if config.transactions.min_per_account > config.transactions.max_per_account {
    return Err("transactions.min_per_account must be <= max_per_account".to_string());
  }
  if config.amounts.min > config.amounts.max {
    return Err("amounts.min must be <= amounts.max".to_string());
  }
  if config.amounts.min < 0.0 {
    return Err("amounts.min must be >= 0".to_string());
  }
  if config.amounts.precision > 4 {
    return Err("amounts.precision must be <= 4".to_string());
  }
  if !(0.0..=1.0).contains(&config.withdrawals.probability) {
    return Err("withdrawals.probability must be between 0.0 and 1.0".to_string());
  }
  if !(0.0..=1.0).contains(&config.withdrawals.overdraw_probability) {
    return Err("withdrawals.overdraw_probability must be between 0.0 and 1.0".to_string());
  }
  if !(0.0..=1.0).contains(&config.disputes.probability) {
    return Err("disputes.probability must be between 0.0 and 1.0".to_string());
  }
  if !(0.0..=1.0).contains(&config.disputes.resolution_probability) {
    return Err("disputes.resolution_probability must be between 0.0 and 1.0".to_string());
  }
  Ok(())
}

fn generate_transactions(config: &Config) -> Vec<Transaction> {
  let mut rng: Box<dyn RngCore> = match config.output.seed {
    Some(seed) => Box::new(StdRng::seed_from_u64(seed)),
    None => Box::new(rand::thread_rng()),
  };

  let mut transactions = Vec::new();
  let mut account_states: HashMap<u16, AccountState> = HashMap::new();
  let mut next_tx_id: u32 = 1;

  // Track deposits that can be disputed (not yet disputed or resolved)
  let mut disputable_deposits: Vec<(u16, u32, f64)> = Vec::new(); // (client, tx, amount)
  // Track disputed deposits pending resolution/chargeback
  let mut pending_disputes: Vec<(u16, u32, f64)> = Vec::new(); // (client, tx, amount)

  // Determine transaction count for each account and track remaining
  let mut remaining_txs: HashMap<u16, u32> = HashMap::new();
  for client in 1..=config.accounts.count {
    let tx_count =
      rng.gen_range(config.transactions.min_per_account..=config.transactions.max_per_account);
    remaining_txs.insert(client, tx_count);
  }

  // Build a list of accounts that still need transactions
  let mut active_accounts: Vec<u16> = (1..=config.accounts.count).collect();

  // Generate transactions in interleaved fashion
  while !active_accounts.is_empty() {
    // Pick a random account from those still active
    let idx = rng.gen_range(0..active_accounts.len());
    let client = active_accounts[idx];

    let state = account_states.entry(client).or_default();
    let remaining = remaining_txs.get_mut(&client).unwrap();

    let is_withdrawal = rng.gen::<f64>() < config.withdrawals.probability;

    if is_withdrawal {
      let amount =
        if state.available <= 0.0 || rng.gen::<f64>() < config.withdrawals.overdraw_probability {
          generate_amount(&mut rng, &config.amounts)
        } else {
          let max_withdraw = state.available.min(config.amounts.max);
          let min_withdraw = config.amounts.min.min(max_withdraw);
          round_to_precision(rng.gen_range(min_withdraw..=max_withdraw), config.amounts.precision)
        };

      transactions.push(Transaction::Withdrawal { client, tx: next_tx_id, amount });

      if amount <= state.available {
        state.available -= amount;
      }
    } else {
      let amount = generate_amount(&mut rng, &config.amounts);
      transactions.push(Transaction::Deposit { client, tx: next_tx_id, amount });
      state.available += amount;
      state.deposits.push((next_tx_id, amount));

      if rng.gen::<f64>() < config.disputes.probability {
        disputable_deposits.push((client, next_tx_id, amount));
      }
    }

    next_tx_id += 1;
    *remaining -= 1;

    // Remove account from active list if done
    if *remaining == 0 {
      active_accounts.swap_remove(idx);
    }
  }

  // Now add disputes scattered through the end of transactions
  // Shuffle disputable deposits to randomize which get disputed
  disputable_deposits.shuffle(&mut *rng);

  for (client, tx, amount) in disputable_deposits {
    transactions.push(Transaction::Dispute { client, tx });
    pending_disputes.push((client, tx, amount));
  }

  // Add resolutions and chargebacks for disputed transactions
  for (client, tx, _amount) in pending_disputes {
    if rng.gen::<f64>() < config.disputes.resolution_probability {
      transactions.push(Transaction::Resolve { client, tx });
    } else {
      transactions.push(Transaction::Chargeback { client, tx });
    }
  }

  transactions
}

fn generate_amount(rng: &mut dyn RngCore, config: &AmountsConfig) -> f64 {
  let amount = rng.gen_range(config.min..=config.max);
  round_to_precision(amount, config.precision)
}

fn round_to_precision(value: f64, precision: u8) -> f64 {
  let factor = 10_f64.powi(precision as i32);
  (value * factor).round() / factor
}

fn write_output(
  config: &Config,
  transactions: &[Transaction],
) -> Result<(), Box<dyn std::error::Error>> {
  let mut writer: Box<dyn Write> = if config.output.file == "-" {
    Box::new(io::stdout())
  } else {
    Box::new(BufWriter::new(File::create(&config.output.file)?))
  };

  // Write header
  writeln!(writer, "type,client,tx,amount")?;

  // Write transactions
  for tx in transactions {
    writeln!(writer, "{}", tx.to_csv_row(config.amounts.precision))?;
  }

  writer.flush()?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_config() -> Config {
    Config {
      accounts: AccountsConfig { count: 5 },
      transactions: TransactionsConfig { min_per_account: 3, max_per_account: 5 },
      amounts: AmountsConfig { min: 10.0, max: 100.0, precision: 2 },
      withdrawals: WithdrawalsConfig { probability: 0.3, overdraw_probability: 0.1 },
      disputes: DisputesConfig { probability: 0.2, resolution_probability: 0.5 },
      output: OutputConfig { file: "-".to_string(), seed: Some(42) },
    }
  }

  #[test]
  fn test_generate_produces_transactions() {
    let config = test_config();
    let transactions = generate_transactions(&config);

    // Should have at least min_per_account * count transactions
    assert!(
      transactions.len()
        >= (config.transactions.min_per_account * config.accounts.count as u32) as usize
    );
  }

  #[test]
  fn test_round_to_precision() {
    assert_eq!(round_to_precision(1.2345, 2), 1.23);
    assert_eq!(round_to_precision(1.2355, 2), 1.24);
    assert_eq!(round_to_precision(1.5, 0), 2.0);
  }

  #[test]
  fn test_validate_config() {
    let config = test_config();
    assert!(validate_config(&config).is_ok());
  }

  #[test]
  fn test_validate_config_invalid_probability() {
    let mut config = test_config();
    config.withdrawals.probability = 1.5;
    assert!(validate_config(&config).is_err());
  }

  #[test]
  fn test_seeded_generation_is_reproducible() {
    let config = test_config();
    let tx1 = generate_transactions(&config);
    let tx2 = generate_transactions(&config);

    assert_eq!(tx1.len(), tx2.len());
  }
}
