// Copyright (c) 2020, Hugues GUILLEUS <ghugues@netc.fr>. All rights reserved.
// Use of this source code is governed by a BSD
// license that can be found in the LICENSE file.

use std::io;
use std::io::{Read, Write};
use std::time::Duration;

mod webvtt;
pub use webvtt::out as webvttOut;
pub use webvtt::Parser as webvttParser;

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
            webvtt::Parser::parse(input_reader),
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

/// Will be remove and a parser will be a Iterator of a result.
pub trait CueParser: Iterator<Item = Cue> {
    /// Move and return the error from the parser.
    fn get_err(&mut self) -> Option<io::Error>;
}

/// Apply the delta time to all input cues and save them into the output_writer.
pub fn convert_output<I: CueParser, W: Write>(
    mut input: I,
    output_writer: W,
    output_format: Format,
    delta: Delta,
) -> io::Result<usize> {
    let delayer = (&mut input).map(|mut c| {
        delta.apply(&mut c);
        c
    });

    let nb = match output_format {
        Format::WebVTT => webvtt::out,
        Format::B => b_out,
    }(delayer, output_writer)?;

    match input.get_err() {
        Some(e) => Err(e),
        None => Ok(nb),
    }
}

/* ONLY FOR TEST THE `convert` function */

/// Just for dev
struct Bparser<R: Read> {
    _r: R,
}
impl<R: Read> CueParser for Bparser<R> {
    fn get_err(&mut self) -> Option<io::Error> {
        None
    }
}
impl<R: Read> Iterator for Bparser<R> {
    type Item = Cue;
    fn next(&mut self) -> Option<Cue> {
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
