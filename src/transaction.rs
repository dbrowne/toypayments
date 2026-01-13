use rust_decimal::Decimal;
use serde::Deserialize;

///  The transactions described in the spec.  HUMAN GENERATED CODE
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
  Deposit,
  Withdrawal,
  Dispute,
  Resolve,
  Chargeback,
}

///  The CSV input deserialized for serde.
#[derive(Debug, Clone, Deserialize)]
pub struct TransactionRecord {
  #[serde(rename = "type")]
  pub tx_type: TransactionType,
  pub client: u16,
  pub tx: u32,
  #[serde(default, deserialize_with = "deserialize_optional_decimal")]
  pub amount: Option<Decimal>,
}

///  THis is needed to address empty strings in the csv
fn deserialize_optional_decimal<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  use serde::de::Error;

  let s: Option<String> = Option::deserialize(deserializer)?;
  match s {
    None => Ok(None),
    Some(s) if s.trim().is_empty() => Ok(None),
    Some(s) => s
      .trim()
      .parse::<Decimal>()
      .map(Some)
      .map_err(|e| D::Error::custom(format!("invalid decimal: {}", e))),
  }
}

/// the  stored transaction (deposit/withdrawal) that may be referenced by disputes
#[derive(Debug, Clone)]
pub struct StoredTransaction {
  pub tx_type: TransactionType,
  pub client: u16,
  pub amount: Decimal,
  pub disputed: bool,
}

impl StoredTransaction {
  pub fn new(tx_type: TransactionType, client: u16, amount: Decimal) -> Self {
    Self { tx_type, client, amount, disputed: false }
  }
}

///AI Generated tests
/// PROMPT:  Generate the necessary test cases for the code in transaction.rs
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_deserialize_deposit() {
    let data = "type,client,tx,amount\ndeposit,1,1,100.5";
    let mut reader = csv::ReaderBuilder::new().trim(csv::Trim::All).from_reader(data.as_bytes());

    let record: TransactionRecord = reader.deserialize().next().unwrap().unwrap();
    assert_eq!(record.tx_type, TransactionType::Deposit);
    assert_eq!(record.client, 1);
    assert_eq!(record.tx, 1);
    assert_eq!(record.amount, Some(Decimal::new(1005, 1)));
  }

  #[test]
  fn test_deserialize_dispute_no_amount() {
    let data = "type,client,tx,amount\ndispute,1,1,";
    let mut reader = csv::ReaderBuilder::new().trim(csv::Trim::All).from_reader(data.as_bytes());

    let record: TransactionRecord = reader.deserialize().next().unwrap().unwrap();
    assert_eq!(record.tx_type, TransactionType::Dispute);
    assert_eq!(record.amount, None);
  }
}
