use std::ffi::OsString;
use std::io::{self, Read};

use clap::Arg;
use pica::{PicaWriter, Reader, ReaderBuilder, WriterBuilder};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::util::{CliArgs, CliError, CliResult, Command};
use crate::{gzip_flag, skip_invalid_flag};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct SliceConfig {
    pub(crate) skip_invalid: Option<bool>,
    pub(crate) gzip: Option<bool>,
}

pub(crate) fn cli() -> Command {
    Command::new("slice")
        .about("Return records within a range (half-open interval).")
        .arg(
            Arg::new("skip-invalid")
                .short('s')
                .long("skip-invalid")
                .help("skip invalid records"),
        )
        .arg(
            Arg::new("start")
                .long("start")
                .help("The lower bound of the range (inclusive).")
                .default_value("0"),
        )
        .arg(
            Arg::new("end")
                .long("end")
                .help("The upper bound of the range (exclusive).")
                .takes_value(true),
        )
        .arg(
            Arg::new("length")
                .long("length")
                .help("The length of the slice.")
                .conflicts_with("end")
                .takes_value(true),
        )
        .arg(
            Arg::new("gzip")
                .short('g')
                .long("gzip")
                .help("compress output with gzip"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("--output")
                .value_name("file")
                .help("Write output to <file> instead of stdout."),
        )
        .arg(
            Arg::new("filenames")
                .help(
                    "Read one or more files in normalized PICA+ format. If the file \
                    ends with .gz the content is automatically decompressed. With no \
                    <filenames>, or when filename is -, read from standard input (stdin).")
                .value_name("filenames")
                .multiple_values(true)
        )
}

pub(crate) fn run(args: &CliArgs, config: &Config) -> CliResult<()> {
    let skip_invalid = skip_invalid_flag!(args, config.slice, config.global);
    let gzip_compression = gzip_flag!(args, config.slice);

    let mut writer: Box<dyn PicaWriter> = WriterBuilder::new()
        .gzip(gzip_compression)
        .from_path_or_stdout(args.value_of("output"))?;

    // SAFETY: It's safe to call `unwrap()` because start has a default value.
    let start = match args.value_of("start").unwrap().parse::<usize>() {
        Ok(start) => start,
        Err(_) => {
            return Err(CliError::Other("invalid start option".to_string()))
        }
    };

    let end = args.value_of("end");
    let length = args.value_of("length");

    let mut range = if let Some(end) = end {
        let end = match end.parse::<usize>() {
            Ok(end) => end,
            Err(_) => {
                return Err(CliError::Other("invalid end option".to_string()))
            }
        };

        start..end
    } else if let Some(length) = length {
        let length = match length.parse::<usize>() {
            Ok(end) => end,
            Err(_) => {
                return Err(CliError::Other(
                    "invalid length option".to_string(),
                ))
            }
        };

        start..start + length
    } else {
        start..::std::usize::MAX
    };

    let filenames = args
        .values_of_t::<OsString>("filenames")
        .unwrap_or_else(|_| vec![OsString::from("-")]);

    let mut i = 0;

    for filename in filenames {
        let builder = ReaderBuilder::new().skip_invalid(false);
        let mut reader: Reader<Box<dyn Read>> = match filename.to_str() {
            Some("-") => builder.from_reader(Box::new(io::stdin())),
            _ => builder.from_path(filename)?,
        };

        for result in reader.byte_records() {
            match result {
                Ok(record) => {
                    if range.contains(&i) {
                        writer.write_byte_record(&record)?;
                    } else if i < range.start {
                        i += 1;
                        continue;
                    } else {
                        break;
                    }
                }
                Err(e) if !skip_invalid => return Err(CliError::from(e)),
                _ => {
                    if length.is_some() && range.end < std::usize::MAX {
                        range.end += 1;
                    }
                }
            }

            i += 1;
        }
    }

    writer.finish()?;
    Ok(())
}
