use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = "Decodes a Wii's uid.sys file")]
struct Cli {
    /// Print the type of a particular title according to its prefix
    #[arg(long, short)]
    decode_prefix: bool,
 
    #[arg(long, short)]
    /// Path to a Wii Title Database text file. If provided, the name of each title will be printed if known.
    title_db: Option<String>,

    uid_file: String,
}

fn main() {
    let args = Cli::parse();

    let entries = match get_entries_from_file(&args.uid_file) {
        Some(e) => e,
        None => return
    };

    print_entries(&entries, args.decode_prefix, args.title_db);
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IoError(e) => write!(f, "{e}"),
            Error::ReadError => write!(f, "File format error"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

#[derive(Debug)]
enum Error {
    IoError(std::io::Error),
    ReadError,
}

fn read_titledb(path: impl AsRef<Path>) -> Result<HashMap<String, String>, Error> {
    let file = BufReader::new(File::open(path)?);
    let mut result = HashMap::<String, String>::new();

    for line in file.lines() {
        let line = line?;
        let mut entry = line.split(" = ");

        let title_id;
        let human_name;

        if let (Some(t), Some(h)) = (entry.next(), entry.next()) {
            title_id = t;
            human_name = h;
        } else {
            return Err(Error::ReadError);
        }

        result.insert(title_id.to_owned(), human_name.to_owned());
    }

    Ok(result)
}

fn print_entries(entries: &[Entry], pretty_prefix: bool, title_db_path: Option<impl AsRef<Path>>) {
    let title_db;

    if let Some(path) = title_db_path {
        title_db = match read_titledb(path) {
            Ok(m) => Some(m),
            Err(e) => {
                eprintln!("error while reading title database: {e}");
                None
            }
        };
    } else {
        title_db = None;
    }

    for entry in entries {
        let title_id_prefix = if pretty_prefix {
            match (entry.title_id >> 32) as u32 {
                0x00000001 => "SYSTEM ESSENTIAL",
                0x00000007 => "vWII ESSENTIAL",
                0x00010000 => "DISC-BASED GAME",
                0x00010001 => "DOWNLOADED CHANNEL",
                0x00010002 => "SYSTEM CHANNEL",
                0x00070002 => "vWII SYSTEM CHANNEL",
                0x00010004 => "GAME CHANNEL",
                0x00010005 => "GAME DLC",
                0x00010008 => "HIDDEN CHANNEL",
                0x00070008 => "vWII HIDDEN",

                _ => "Error",
            }
        } else {
            ""
        };

        let title_id_prefix_raw = format!("{:08X}", (entry.title_id >> 32) as u32);

        let title_id_gameid_string = make_gameid_string(entry.title_id as u32);

        let title_id_gameid_raw = format!("{:08X}", (entry.title_id as u32));

        let install_num = entry.uid - 4095;

        let title_human_name = match &title_db {
            Some(title_db) => match title_db.get(title_id_gameid_string.as_str()) {
                Some(s) => format!(" - {s}"),
                None => {
                    if (entry.title_id as u32) < 255 {
                        format!(" - IOS {}", (entry.title_id as u32))
                    } else {
                        " - ????".to_owned()
                    }
                }
            },

            None => "".to_owned(),
        };

        if pretty_prefix {
            println!("{install_num}: {: <19}{title_id_prefix_raw}-{title_id_gameid_raw} ({title_id_gameid_string}){title_human_name}", title_id_prefix)
        } else {
            println!("{install_num}: {title_id_prefix_raw}-{title_id_gameid_raw} ({title_id_gameid_string}){title_human_name}")
        }
    }
}

fn make_gameid_string(gameid: u32) -> String {
    let bytes = gameid.to_be_bytes();

    let mut result = String::new();

    for byte in bytes {
        let character = if (32..128).contains(&byte) {
            char::from(byte)
        } else {
            '.'
        };

        result.push(character);
    }

    result
}

fn get_entries_from_file(file_name: &str) -> Option<Vec<Entry>> {
    let bytes = match fs::read(file_name) {
        Ok(v) => v,
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {
                eprintln!("\"{file_name}\": File not found");
                return None;
            }

            _ => {
                eprintln!("\"{file_name}\": Error opening file");
                return None;
            }
        },
    };

    let mut entries: Vec<Entry> = vec![];

    for entry in bytes.chunks(12) {
        entries.push(Entry::from(
            <&[u8] as TryInto<&[u8; 12]>>::try_into(entry).unwrap(),
        ))
    }

    Some(entries)
}

impl From<&[u8; 12]> for Entry {
    fn from(value: &[u8; 12]) -> Self {
        Self {
            title_id: u64::from_be_bytes(value[0..8].try_into().unwrap()),
            // padding: u16::from_be_bytes(value[8..10].try_into().unwrap()),
            uid: u16::from_be_bytes(value[10..12].try_into().unwrap()),
        }
    }
}

struct Entry {
    title_id: u64,
    // padding: u16,
    uid: u16,
}
