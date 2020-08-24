// Copyright (c) 2020, Hugues GUILLEUS <ghugues@netc.fr>. All rights reserved.
// Use of this source code is governed by a BSD
// license that can be found in the LICENSE file.

use std::io;
use std::path::Path;
use std::time::Duration;
pub mod srt;
pub mod webvtt;

#[derive(std::fmt::Debug)]
pub struct Cue {
    begin: Duration,
    end: Duration,
    text: Vec<String>,
}

impl Cue {
    pub fn new(begin: Duration, end: Duration, t: Vec<String>) -> Cue {
        Cue {
            begin: begin,
            end: end,
            text: t,
        }
    }
}

/// Convert the .srt file to a .vtt file.
pub fn str2vtt_file(file_name: &str) -> io::Result<()> {
    srt2vtt(
        &mut std::fs::File::create(Path::new(file_name).with_extension("vtt"))?,
        std::fs::File::open(file_name)?,
    )
}

// Convert
pub fn srt2vtt<R: io::Read, W: io::Write>(output: &mut W, input: R) -> io::Result<()> {
    webvtt::write_head(output)?;

    let mut parser = srt::Srt::parse(input);

    if let Some(e) = (&mut parser)
        .filter_map(|c| webvtt::write(output, c).err())
        .find(|_| true)
    {
        return Err(e);
    }

    match parser.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
