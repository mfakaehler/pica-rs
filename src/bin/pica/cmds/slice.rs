use crate::util::{App, CliArgs, CliError, CliResult};
use clap::Arg;
use pica::{ReaderBuilder, Writer, WriterBuilder};
use std::io::Write;

pub fn cli() -> App {
    App::new("slice")
        .about("Return records within a range (half-open interval).")
        .arg(
            Arg::new("skip-invalid")
                .short('s')
                .long("skip-invalid")
                .about("skip invalid records"),
        )
        .arg(
            Arg::new("start")
                .long("start")
                .about("The lower bound of the range (inclusive).")
                .default_value("0"),
        )
        .arg(
            Arg::new("end")
                .long("end")
                .about("The upper bound of the range (exclusive).")
                .takes_value(true),
        )
        .arg(
            Arg::new("length")
                .long("length")
                .about("The length of the slice.")
                .conflicts_with("end")
                .takes_value(true),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("--output")
                .value_name("file")
                .about("Write output to <file> instead of stdout."),
        )
        .arg(Arg::new("filename"))
}

pub fn run(args: &CliArgs) -> CliResult<()> {
    let mut reader = ReaderBuilder::new()
        .skip_invalid(false)
        .from_path_or_stdin(args.value_of("filename"))?;

    let mut writer: Writer<Box<dyn Write>> =
        WriterBuilder::new().from_path_or_stdout(args.value_of("output"))?;

    let skip_invalid = args.is_present("skip-invalid");
    let start = args.value_of("start").unwrap().parse::<usize>().unwrap();
    let end = args.value_of("end");
    let length = args.value_of("length");

    let mut range = if let Some(end) = end {
        start..end.parse::<usize>().unwrap()
    } else if let Some(length) = length {
        let length = length.parse::<usize>().unwrap();
        start..start + length
    } else {
        start..::std::usize::MAX
    };

    for (i, result) in reader.byte_records().enumerate() {
        match result {
            Ok(record) => {
                if range.contains(&i) {
                    writer.write_byte_record(&record)?;
                } else if i < range.start {
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
    }

    writer.flush()?;
    Ok(())
}
