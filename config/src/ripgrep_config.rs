// The code in this file has in large parts been copied from the ripgrep
// code located at:
// https://github.com/BurntSushi/ripgrep/blob/0874aa115c92f102a6ec474944f589667463fcd0/crates/core/config.rs

//  ------------------------

// This module provides routines for reading ripgrep config "rc" files. The
// primary output of these routines is a sequence of arguments, where each
// argument corresponds precisely to one shell argument.

use std::env;
use std::error;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use bstr::{io::BufReadExt, ByteSlice};

type Result<T> = ::std::result::Result<T, Box<dyn error::Error>>;

/// Return a sequence of arguments derived from ripgrep rc configuration files.
///
/// * `environment` - The name of an environment variable to check for an additional
/// config file
pub fn args(environment: &str) -> Vec<OsString> {
    let config_path = match env::var_os(environment) {
        None => return vec![],
        Some(config_path) => {
            if config_path.is_empty() {
                return vec![];
            }
            PathBuf::from(config_path)
        }
    };
    let (args, errs) = match parse(&config_path) {
        Ok((args, errs)) => (args, errs),
        Err(err) => {
            println!("{}", err);
            return vec![];
        }
    };
    if !errs.is_empty() {
        for err in errs {
            println!("{}:{}", config_path.display(), err);
        }
    }
    println!(
        "{}: arguments loaded from config file: {:?}",
        config_path.display(),
        args
    );
    args
}

/// Parse a single ripgrep rc file from the given path.
///
/// On success, this returns a set of shell arguments, in order, that should
/// be pre-pended to the arguments given to ripgrep at the command line.
///
/// If the file could not be read, then an error is returned. If there was
/// a problem parsing one or more lines in the file, then errors are returned
/// for each line in addition to successfully parsed arguments.
fn parse<P: AsRef<Path>>(path: P) -> Result<(Vec<OsString>, Vec<Box<dyn Error>>)> {
    let path = path.as_ref();
    match File::open(&path) {
        Ok(file) => parse_reader(file),
        Err(err) => Err(From::from(format!("{}: {}", path.display(), err))),
    }
}

/// Parse a single ripgrep rc file from the given reader.
///
/// Callers should not provided a buffered reader, as this routine will use its
/// own buffer internally.
///
/// On success, this returns a set of shell arguments, in order, that should
/// be pre-pended to the arguments given to ripgrep at the command line.
///
/// If the reader could not be read, then an error is returned. If there was a
/// problem parsing one or more lines, then errors are returned for each line
/// in addition to successfully parsed arguments.
fn parse_reader<R: io::Read>(rdr: R) -> Result<(Vec<OsString>, Vec<Box<dyn Error>>)> {
    let bufrdr = io::BufReader::new(rdr);
    let (mut args, mut errs) = (vec![], vec![]);
    let mut line_number = 0;
    bufrdr.for_byte_line_with_terminator(|line| {
        line_number += 1;

        let line = line.trim();
        if line.is_empty() || line[0] == b'#' {
            return Ok(true);
        }
        match line.to_os_str() {
            Ok(osstr) => {
                args.push(osstr.to_os_string());
            }
            Err(err) => {
                errs.push(format!("{}: {}", line_number, err).into());
            }
        }
        Ok(true)
    })?;
    Ok((args, errs))
}