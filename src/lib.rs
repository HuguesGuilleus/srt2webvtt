// Copyright (c) 2020, Hugues GUILLEUS <ghugues@netc.fr>. All rights reserved.
// Use of this source code is governed by a BSD
// license that can be found in the LICENSE file.

use std::io;
use std::io::{BufRead, BufReader, Lines, Read, Write};
use std::time::Duration;

mod webvtt;
pub use webvtt::out as webvtt_out;
pub use webvtt::WebVTTParser;

/// One cue.
#[derive(Clone, Debug, PartialEq)]
pub struct Cue {
    pub id: Option<String>,
    pub begin: Duration,
    pub end: Duration,
    pub text: Vec<String>,
}
impl Cue {
    /// Create a new cue.
    pub fn new(id: Option<String>, begin: Duration, end: Duration, t: Vec<String>) -> Cue {
        if begin > end {
            Cue {
                id: id,
                begin: end,
                end: begin,
                text: t,
            }
        } else {
            Cue {
                id: id,
                begin: begin,
                end: end,
                text: t,
            }
        }
    }
}

/// A delta time to apply on a cue.
#[derive(Clone, Debug)]
pub enum Delta {
    Add(Duration),
    Sub(Duration),
    None,
}
impl Delta {
    /// Apply the delta time to the cue.
    fn apply(&self, c: &mut Cue) {
        match self {
            Delta::Add(d) => {
                c.begin += *d;
                c.end += *d;
            }
            Delta::Sub(d) => {
                c.begin -= *d;
                c.end -= *d;
            }
            Delta::None => {}
        }
    }
}
#[test]
fn delat_apply() {
    let c = Cue::new(None, Duration::new(5, 10), Duration::new(6, 20), Vec::new());

    let mut cc = c.clone();
    Delta::None.apply(&mut cc);
    assert_eq!(cc, c);

    let mut cc = c.clone();
    Delta::Add(Duration::new(10, 0)).apply(&mut cc);
    assert_eq!(
        cc,
        Cue::new(
            None,
            Duration::new(15, 10),
            Duration::new(16, 20),
            Vec::new()
        )
    );

    let mut cc = c.clone();
    Delta::Sub(Duration::new(2, 0)).apply(&mut cc);
    assert_eq!(
        cc,
        Cue::new(None, Duration::new(3, 10), Duration::new(4, 20), Vec::new())
    );
}

/// The format for input or output stream.
#[derive(Debug, Copy, Clone)]
pub enum Format {
    WebVTT,
    B,
}

/// Convert cues from input reader of a format (defined in this crate) to an other
pub fn convert<R: Read, W: Write>(
    input_reader: R,
    input_format: Format,
    output_writer: W,
    output_format: Format,
    delta: Delta,
) -> io::Result<usize> {
    match input_format {
        Format::WebVTT => convert_output(
            WebVTTParser::parse(input_reader)?,
            output_writer,
            output_format,
            delta,
        ),
        Format::B => convert_output(
            Bparser { _r: input_reader },
            output_writer,
            output_format,
            delta,
        ),
    }
}
#[test]
fn test_convert() {
    let mut out: Vec<u8> = Vec::new();

    convert(
        "WEBVTT

NOTE Hello World

00:01.000 --> 00:04.000
Never drink liquid nitrogen.

identifier
00:05.000 --> 00:09.000
— It will perforate your stomach.
— You could die."
            .as_bytes(),
        Format::WebVTT,
        &mut out,
        Format::WebVTT,
        Delta::Add(Duration::new(1, 0)),
    )
    .unwrap();

    assert_eq!(
        std::str::from_utf8(&out).unwrap(),
        "WEBVTT

00:02.000 --> 00:05.000
Never drink liquid nitrogen.

identifier
00:06.000 --> 00:10.000
— It will perforate your stomach.
— You could die.

"
    );
}

/// Apply the delta time to all input cues and save them into the output_writer.
pub fn convert_output<I: Iterator<Item = io::Result<Cue>>, W: Write>(
    mut input: I,
    output_writer: W,
    output_format: Format,
    delta: Delta,
) -> io::Result<usize> {
    let mut error: Option<io::Error> = None;

    let cues = (&mut input)
        .filter_map(|r| {
            if error.is_some() {
                None
            } else {
                match r {
                    Ok(c) => Some(c),
                    Err(e) => {
                        error = Some(e);
                        None
                    }
                }
            }
        })
        .map(|mut c| {
            delta.apply(&mut c);
            c
        });

    let nb = match output_format {
        Format::WebVTT => webvtt_out,
        Format::B => b_out,
    }(cues, output_writer)?;

    match error {
        Some(e) => Err(e),
        None => Ok(nb),
    }
}

/* ONLY FOR TEST THE `convert` function */

/// Just for dev
struct Bparser<R: Read> {
    _r: R,
}
impl<R: Read> Iterator for Bparser<R> {
    type Item = io::Result<Cue>;
    fn next(&mut self) -> Option<io::Result<Cue>> {
        unimplemented!()
    }
}

/// Just for dev
pub fn b_out<I, W>(_: I, _: W) -> Result<usize, std::io::Error>
where
    W: Write,
    I: std::iter::Iterator<Item = Cue>,
{
    unimplemented!()
}

// A line by line reader that count readed line.
struct LineNb<R: Read> {
    lines: Lines<BufReader<R>>,
    nb: usize,
}
impl<R: Read> LineNb<R> {
    pub fn new(r: R) -> Self {
        Self {
            lines: BufReader::new(r).lines(),
            nb: 0,
        }
    }
    /// Return the current line number.
    pub fn current(&self) -> usize {
        self.nb - 1
    }
}
impl<R: Read> Iterator for LineNb<R> {
    type Item = io::Result<String>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.lines.next() {
            Some(Ok(l)) => {
                self.nb += 1;
                Some(Ok(l))
            }
            x => x,
        }
    }
}
