use std::{
  fs::File,
  io::{BufReader, BufWriter},
  path::PathBuf,
};

use clap_verbosity_flag::Verbosity;
use lazy_static::lazy_static;
use log::{debug, info};
use regex::Regex;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

type CliResult = Result<(), exitfailure::ExitFailure>;

const BUFFER_SIZE: usize = 64 * 1024;
const MOB_REGEX_STR: &str = "^((20)|(966))([0-9]{9,11})$";

lazy_static! {
  static ref MOB_RE: Regex = Regex::new(MOB_REGEX_STR).unwrap();
  static ref REPLACER_RE: Regex =
    Regex::new(r#"^(0)|^(00)|[!@+#$%\-^&*() ]"#).unwrap();
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
  for r in rdr.deserialize() {
    let record = r?;
    if let Some(d) = is_good_ph(record) {
      wrt.serialize(d)?;
    }
  }
  wrt.flush()?;
  info!("Done !");
  Ok(())
}

#[inline]
fn remove_bad_chars(mut record: Record) -> Record {
  // we need to remove all spacial characters to empty one, so we can then
  // validate the mobile number.
  record.ph = REPLACER_RE.replace_all(&record.ph.trim(), "").trim().into();
  record
}

fn standardize_ph(mut record: Record) -> Record {
  match record.ph.chars().next() {
    Some('1') => {
      // Egypt, so we need to add 20
      record.ph = "20".to_owned() + &record.ph;
      record
    },
    Some('5') => {
      // Saudi Arabia, add 966
      record.ph = "966".to_owned() + &record.ph;
      record
    },
    //    Some('0') => {
    //      // yup it is Egypt, add just 2
    //      if record.ph.starts_with("01") {
    //        record.ph = "2".to_owned() + &record.ph;
    //      }
    //      record
    //    },
    // if none of the above matched then just return it
    _ => record,
  }
}

fn is_good_ph(record: Record) -> Option<Record> {
  let r = remove_bad_chars(record);
  let r = standardize_ph(r);
  if MOB_RE.is_match(&r.ph) {
    Some(r)
  } else {
    debug!("Not Acceptable: {:?}", r);
    None
  }
}

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
    let bad_record3 = Record::new("1232131", "test3", 0);
    let bad_record4 = Record::new("00", "test4", 0);
    let bad_record5 = Record::new("2011166130", "test5", 0);
    assert!(is_good_ph(bad_record).is_none());
    assert!(is_good_ph(bad_record2).is_none());
    assert!(is_good_ph(bad_record3).is_none());
    assert!(is_good_ph(bad_record4).is_none());
    assert!(is_good_ph(bad_record5).is_none());
  }

  #[test]
  fn should_pass_good_numbers() {
    let good_record = Record::new("201116613061", "test1", 0);
    let good_record2 = Record::new("00201116613061", "test2", 0);
    let good_record3 = Record::new("+2(0111)6613061", "test3", 0);
    let good_record4 = Record::new("+2011-1661-3061", "test4", 0);
    let good_record5 = Record::new("+201116613061", "test5", 0);
    let good_record6 = Record::new("1116613061", "test6", 0);
    let good_record7 = Record::new("540029129", "test7", 0);
    let good_record8 = Record::new("5400 291 29", "test8", 0);
    assert!(is_good_ph(good_record).is_some());
    assert!(is_good_ph(good_record2).is_some());
    assert!(is_good_ph(good_record3).is_some());
    assert!(is_good_ph(good_record4).is_some());
    assert!(is_good_ph(good_record5).is_some());
    assert!(is_good_ph(good_record6).is_some());
    assert!(is_good_ph(good_record7).is_some());
    assert!(is_good_ph(good_record8).is_some());
  }

  #[test]
  fn should_standardize_ph() {
    let good_record = Record::new("1116613061", "test1", 0);
    let good_record2 = Record::new("511661306", "test2", 0);
    assert_eq!(standardize_ph(good_record).ph, "201116613061");
    assert_eq!(standardize_ph(good_record2).ph, "966511661306");
  }
}
