use std::{
  fs::File,
  io::{BufReader, BufWriter},
  path::PathBuf,
};

use clap_verbosity_flag::Verbosity;
use lazy_static::lazy_static;
use log::info;
use regex::Regex;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

type CliResult = Result<(), exitfailure::ExitFailure>;

const BUFFER_SIZE: usize = 64 * 1024;
const MOB_REGEX_STR: &str = "^[0-9]{11,14}$";

lazy_static! {
  static ref MOB_RE: Regex = Regex::new(MOB_REGEX_STR).unwrap();
  static ref REPLACER_RE: Regex = Regex::new(r#"[!@+#$%\-^&*() ]"#).unwrap();
}

#[derive(Debug, StructOpt)]
#[structopt(
  name = "mobcsv",
  about = "Validate and format mobile number in one standard way.",
  version = "0.1.0",
  author = "Shady Khalifa <shekohex@gmail>",
  rename_all = "kebab-case"
)]
struct Cli {
  /// The CSV output file path
  #[structopt(short = "o")]
  output_path: PathBuf,
  #[structopt(flatten)]
  verbosity: Verbosity,
  /// The input CSV file path
  input_path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
struct Record {
  /// The mobile phone number
  ph: String,
  name: String,
  count: u16,
}

fn main() -> CliResult {
  let args: Cli = Cli::from_args();
  args.verbosity.setup_env_logger(&env!("CARGO_PKG_NAME"))?;
  info!("Starting Application...");
  info!("I/O Buffer Size: {} byte", BUFFER_SIZE);
  info!("Reading from {:?}", args.input_path);
  let c = File::open(args.input_path)?;
  let buffer = BufReader::with_capacity(BUFFER_SIZE, c);
  let mut rdr = csv::Reader::from_reader(buffer);
  info!("Trying to write to {:?}", args.output_path);
  let out = File::create(args.output_path)?;
  let buffer = BufWriter::with_capacity(BUFFER_SIZE, out);
  let mut wrt = csv::Writer::from_writer(buffer);
  rdr
    .deserialize()
    .flatten() // because it returns result !
    .map(remove_bad_chars)
    .skip_while(is_bad_number)
    .try_for_each(|r| wrt.serialize(r))?;
  wrt.flush()?;
  info!("Done !");
  Ok(())
}

fn remove_bad_chars(mut record: Record) -> Record {
  // we need to remove all spacial characters to empty one, so we can then
  // validate the mobile number.
  record.ph = REPLACER_RE.replace_all(&record.ph, "").into();
  record
}

fn is_bad_number(record: &Record) -> bool { !MOB_RE.is_match(&record.ph) }

#[cfg(test)]
mod tests {
  use super::*;

  impl Record {
    pub fn new(ph: &str, name: &str, count: u16) -> Self {
      Record {
        ph: ph.to_owned(),
        name: name.to_owned(),
        count,
      }
    }
  }

  #[test]
  fn should_detect_bad_numbers() {
    let bad_record = Record::new("20111bad", "test1", 0);
    let bad_record2 = Record::new("hah2011166130", "test2", 0);
    // short !
    let bad_record3 = Record::new("1232131", "test3", 0);
    let bad_record4 = Record::new("00", "test3", 0);
    assert!(is_bad_number(&remove_bad_chars(bad_record)));
    assert!(is_bad_number(&remove_bad_chars(bad_record2)));
    assert!(is_bad_number(&remove_bad_chars(bad_record3)));
    assert!(is_bad_number(&remove_bad_chars(bad_record4)));
  }

  #[test]
  fn should_pass_good_numbers() {
    let good_record = Record::new("201116613061", "test1", 0);
    let good_record2 = Record::new("+201116613061", "test2", 0);
    let good_record3 = Record::new("+2(0111)6613061", "test3", 0);
    let good_record4 = Record::new("+2011-1661-3061", "test4", 0);
    let good_record5 = Record::new("+21116613061", "test5", 0);
    assert!(!is_bad_number(&remove_bad_chars(good_record)));
    assert!(!is_bad_number(&remove_bad_chars(good_record2)));
    assert!(!is_bad_number(&remove_bad_chars(good_record3)));
    assert!(!is_bad_number(&remove_bad_chars(good_record4)));
    assert!(!is_bad_number(&remove_bad_chars(good_record5)));
  }
}
