use anyhow::Error;
use structopt::StructOpt;

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use riff_wave_reader::RiffWaveReader;

fn main() -> Result<(), Error> {
    let opts = Opts::from_args();

    match opts.command {
        Command::Print { input } => {
            let file = File::open(input)?;
            let reader = BufReader::new(file);

            let reader = RiffWaveReader::new(reader)?;

            reader.print_info();
        }
        Command::Raw { input } => {
            let file = File::open(input)?;
            let reader = BufReader::new(file);

            let mut reader = RiffWaveReader::new(reader)?;

            let data = reader.data()?.collect::<Vec<_>>();
            println!("{}", data.len());
        }
    }

    Ok(())
}

#[derive(StructOpt)]
#[structopt(name = "riff-cli")]
struct Opts {
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
enum Command {
    Print {
        #[structopt(parse(from_os_str))]
        input: PathBuf,
    },
    Raw {
        #[structopt(parse(from_os_str))]
        input: PathBuf,
    },
}
