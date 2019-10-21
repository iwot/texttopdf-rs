// #[macro_use] extern crate itertools;

use std::env;
use std::fs;
use std::io::{BufReader, BufRead};
// use itertools::Itertools;

mod pdf;

// https://itchyny.hatenablog.com/entry/2015/09/16/100000

fn main() -> Result<(), Box<std::error::Error>> {
    let filename = env::args().nth(1).expect("missing input file");

    let mut lines = vec![];
    for line_opt in BufReader::new(fs::File::open(filename)?).lines() {
        let line = line_opt?;
        lines.push(line);
    }

    let mut texts = vec![];
    for slice in lines.chunks(45) {
        texts.push(slice.to_vec());
    }
    let result = pdf::text_to_pdf(texts);
    print!("{}", pdf::render_pdf(result));

    Ok(())
}
