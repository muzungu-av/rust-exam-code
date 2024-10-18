use std::fmt::Debug;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Template {
    pub ixs: usize, //str_index_start
    pub ixe: usize, //str_index_end
}

const COUNT_FIELD_PS: usize = 1; //колво полей в разделе AltCurrency
const COUNT_FIELD_CI: usize = 2; //колво полей в разделе AltCurrency

pub(crate) trait Currency: Debug {
    fn symbol(&self) -> &str;
}

impl Currency for AltCurrency {
    fn symbol(&self) -> &str {
        &self.symbol
    }
}

impl Currency for BaseCurrency {
    fn symbol(&self) -> &str {
        &self.symbol
    }
}

#[derive(Debug)]
pub struct BaseCurrency {
    pub symbol: String,
    pub percentage: f32,
}

#[derive(Debug)]
pub struct AltCurrency {
    pub symbol: String,
}

#[derive(Clone, Debug)]
pub struct ParsedPairs {
    pub symbol: String,
    pub symbol_template: Template,
    pub price_template: Template,
    pub volume_template: Template,
}

impl ParsedPairs {
    pub fn new(
        symbol: String,
        symbol_template: Template,
        price_template: Template,
        volume_template: Template,
    ) -> Self {
        ParsedPairs {
            symbol,
            symbol_template,
            price_template,
            volume_template,
        }
    }
}
pub fn read_setting_base_currency<P: AsRef<Path>>(path: P) -> Vec<BaseCurrency> {
    let file = File::open(path).expect("Failed to open file");
    let reader = BufReader::new(file);

    let mut data = Vec::new();
    let mut inside_section = false;

    for line in reader.lines() {
        let line = line.expect("Failed to read line");

        if line.trim() == "[> BaseCurrency >]" {
            inside_section = true;
        } else if line.trim() == "[< BaseCurrency <]" {
            inside_section = false;
        } else if inside_section {
            if !line.trim_start().starts_with(";") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == COUNT_FIELD_CI {
                    let symbol = parts[0].to_string();
                    let percentage = parts[1]
                        .trim_end_matches('%')
                        .parse::<f32>()
                        .expect("Invalid percentage format");

                    let info = BaseCurrency { symbol, percentage };
                    data.push(info);
                }
            }
        }
    }
    data
}

pub fn read_setting_alt_currency<P: AsRef<Path>>(path: P) -> Vec<AltCurrency> {
    let file = File::open(path).expect("Failed to open file");
    let reader = BufReader::new(file);

    let mut data = Vec::new();
    let mut inside_section = false;

    for line in reader.lines() {
        let line = line.expect("Failed to read line");

        if line.trim() == "[> AltCurrency >]" {
            inside_section = true;
        } else if line.trim() == "[< AltCurrency <]" {
            inside_section = false;
        } else if inside_section {
            if !line.trim_start().starts_with(";") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == COUNT_FIELD_PS {
                    let symbol = parts[0].to_string();
                    let info = AltCurrency { symbol };
                    data.push(info);
                }
            }
        }
    }
    data
}
