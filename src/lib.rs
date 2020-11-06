// Copyright (c) 2020, Hugues GUILLEUS <ghugues@netc.fr>. All rights reserved.
// Use of this source code is governed by a BSD
// license that can be found in the LICENSE file.

use std::io;
use std::io::{BufRead, BufReader, Lines, Read, Write};
use std::str::FromStr;
use std::time::Duration;

mod srt;
pub use srt::out as srt_out;
pub use srt::SrtParser;

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

/// A delta duration to apply on a cue's time code.
#[derive(Clone, Debug, PartialEq)]
pub enum Delta {
    Add(Duration),
    Sub(Duration),
    None,
}
impl Delta {
    /// Apply the delta time to the cue.
    pub fn apply(&self, c: &mut Cue) {
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
    /// A closure to apply the delta time on a Cue. Use it with Iterator.map()
    pub fn applicator(&self) -> impl Fn(Cue) -> Cue {
        fn add(c: &mut Cue, d: &Duration) {
            c.begin += *d;
            c.end += *d;
        }
        fn sub(c: &mut Cue, d: &Duration) {
            c.begin -= *d;
            c.end -= *d;
        }
        fn zero(_: &mut Cue, _: &Duration) {}

        let (f, d): (fn(&mut Cue, &Duration), Duration) = match self {
            Delta::Add(d) => (add, *d),
            Delta::Sub(d) => (sub, *d),
            Delta::None => (zero, Duration::new(0, 0)),
        };

        move |mut c: Cue| {
            f(&mut c, &d);
            c
        }
    }
}
#[test]
fn delta_apply() {
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
#[test]
fn delta_applicator() {
    let c = Cue::new(None, Duration::new(5, 10), Duration::new(6, 20), Vec::new());

    let cc = Delta::None.applicator()(c.clone());
    assert_eq!(cc, c);

    let cc = Delta::Add(Duration::new(10, 0)).applicator()(c.clone());
    assert_eq!(
        cc,
        Cue::new(
            None,
            Duration::new(15, 10),
            Duration::new(16, 20),
            Vec::new()
        )
    );

    let cc = Delta::Sub(Duration::new(2, 0)).applicator()(c.clone());
    assert_eq!(
        cc,
        Cue::new(None, Duration::new(3, 10), Duration::new(4, 20), Vec::new())
    );
}
impl FromStr for Delta {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "" || s == "0" {
            return Ok(Delta::None);
        }

        let sign: char = s.chars().next().unwrap();
        let s = &s[1..];

        let begin = s.find(':');
        let min: f64 = match begin {
            Some(sep) => s[..sep]
                .parse::<u64>()
                .map_err(|err| format!("{} on {:?}", err, s))? as f64,
            None => 0.0,
        } * 60.0;
        let s = match begin {
            Some(sep) => &s[(sep + 1)..],
            None => s,
        };

        let f: f64 = s.parse().map_err(|err| format!("{} on {:?}", err, s))?;
        let d = Duration::from_secs_f64(min + f);

        Ok(match sign {
            '+' => Delta::Add(d),
            '-' => Delta::Sub(d),
            _ => {
                return Err(format!(
                    "Need a sign at begin to a Delta time ({:?}) or zero or an empty string",
                    s
                ))
            }
        })
    }
}
#[test]
fn delta_fromstr() {
    let add = Delta::Add(Duration::new(96, 125_000_000));
    assert_eq!("+96.125".parse::<Delta>().unwrap(), add);
    assert_eq!("+1:36.125".parse::<Delta>().unwrap(), add);

    let sub = Delta::Sub(Duration::new(96, 125_000_000));
    assert_eq!("-96.125".parse::<Delta>().unwrap(), sub);
    assert_eq!("-1:36.125".parse::<Delta>().unwrap(), sub);

    assert_eq!("".parse::<Delta>().unwrap(), Delta::None);
    assert_eq!("0".parse::<Delta>().unwrap(), Delta::None);
}

/// The crate supported formats for input or output stream.
#[derive(Debug, Copy, Clone)]
pub enum Format {
    WebVTT,
    Srt,
}
impl FromStr for Format {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "srt" {
            Ok(Format::Srt)
        } else if s == "webvtt" {
            Ok(Format::WebVTT)
        } else {
            Err(format!(
                "Unknown format for {:?} (possible value are: 'webvtt' and 'srt')",
                s
            ))
        }
    }
}

/// Convert cues from the input, apply delta duration and save it.
pub fn convert<R: Read, W: Write>(
    input_reader: R,
    input_format: Format,
    output_writer: W,
    output_format: Format,
    delta: Delta,
) -> io::Result<usize> {
    match input_format {
        Format::WebVTT => convert_output(
            WebVTTParser::new(input_reader)?,
            output_writer,
            output_format,
            delta,
        ),
        Format::Srt => convert_output(
            SrtParser::new(input_reader)?,
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
        .map(delta.applicator());

    let nb = match output_format {
        Format::WebVTT => webvtt_out,
        Format::Srt => srt_out,
    }(cues, output_writer)?;

    match error {
        Some(e) => Err(e),
        None => Ok(nb),
    }
}

/// A line by line reader that count readed lines.
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
        self.nb
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
