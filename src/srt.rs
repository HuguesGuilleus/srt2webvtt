// Copyright (c) 2020, Hugues GUILLEUS <ghugues@netc.fr>. All rights reserved.
// Use of this source code is governed by a BSD
// license that can be found in the LICENSE file.

use super::{Cue, LineNb};
use std::fmt::Display;
use std::io::{self, BufReader, ErrorKind, Read, Write};
use std::time::Duration;

pub struct SrtParser<R: Read> {
    lines: LineNb<BufReader<R>>,
    end: bool,
}
impl<R: Read> SrtParser<R> {
    pub fn new(r: R) -> io::Result<Self> {
        use std::io::BufRead;

        let mut input = BufReader::new(r);

        let first = input.fill_buf()?;
        if first.len() >= 3 && &first[..3] == [0xEF, 0xBB, 0xBF] {
            input.consume(3);
        }

        Ok(Self {
            lines: LineNb::new(input),
            end: false,
        })
    }
    /// Just after the id line is readed, parse the cue (time code and text content).
    fn next_cue(&mut self) -> io::Result<Cue> {
        match self.lines.next() {
            None => Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                format!(
                    "Expected time code to a new cue (line: {})",
                    self.lines.current()
                ),
            )),
            Some(Err(e)) => Err(e),
            Some(Ok(time_code)) => {
                let (begin, end) = parse_time(&time_code, self.lines.current())?;
                Ok(Cue::new(None, begin, end, self.next_text()?))
            }
        }
    }
    /// Return the text of a cue.
    fn next_text(&mut self) -> io::Result<Vec<String>> {
        let mut text = Vec::new();
        loop {
            match self.lines.next() {
                Some(Err(e)) => return Err(e),
                None => return Ok(text),
                Some(Ok(l)) if l.len() == 0 => return Ok(text),
                Some(Ok(l)) => text.push(l),
            }
        }
    }
}
impl<R: Read> Iterator for SrtParser<R> {
    type Item = io::Result<Cue>;
    fn next(&mut self) -> Option<io::Result<Cue>> {
        if self.end {
            return None;
        }

        match self.lines.next() {
            None => {
                self.end = true;
                None
            }
            Some(Err(e)) => {
                self.end = true;
                Some(Err(e))
            }
            Some(Ok(l)) if l.len() == 0 => self.next(),
            Some(Ok(id)) if id.chars().any(|c| !c.is_numeric()) => {
                self.end = true;
                Some(err_invalid("Unexpected line", &id, self.lines.current()))
            }
            Some(Ok(..)) => match self.next_cue() {
                Err(e) => {
                    self.end = true;
                    Some(Err(e))
                }
                Ok(c) => Some(Ok(c)),
            },
        }
    }
}
#[test]
fn srtparser() {
    use std::io::prelude::*;

    fn t(s: &[u8]) {
        let mut p = SrtParser::new(s).unwrap();
        assert_eq!(
            Cue::new(
                None,
                Duration::new(5, 542_000_000),
                Duration::new(7, 792_000_000),
                vec!["Hello".to_string(), "World".to_string()]
            ),
            p.next().unwrap().unwrap()
        );
    }

    let mut input: Vec<u8> = vec![0xEF, 0xBB, 0xBF];
    input
        .write(
            b"1
00:00:05,542 --> 00:00:07,792
Hello
World
",
        )
        .unwrap();

    t(&input[3..]);
    t(&input[..]);
}

fn parse_time(s: &str, line: usize) -> io::Result<(Duration, Duration)> {
    let split: Vec<&str> = s.split(" --> ").take(3).collect();
    if split.len() != 2 {
        return err_invalid("Invalide time code syntax", s, line);
    }

    Ok((
        parse_duration(split[0].trim_end(), line)?,
        parse_duration(split[1].trim_start(), line)?,
    ))
}
#[test]
fn parse_time_test() {
    fn dur(h: u64, m: u64, s: u64, ms: u32) -> Duration {
        Duration::new(h * 3600 + m * 60 + s, ms * 1_000_000)
    }
    assert_eq!(
        parse_time("17:35:29,942 --> 17:25:48,456", 0).unwrap(),
        (dur(17, 35, 29, 942), dur(17, 25, 48, 456))
    );
}

fn parse_duration(s: &str, line: usize) -> io::Result<Duration> {
    let split: Vec<&str> = s.split(":").take(4).collect();
    if split.len() != 3 {
        return err_invalid("Invalid duration syntax", s, line);
    }

    let second_part: Vec<&str> = split[2].split(",").take(3).collect();
    if second_part.len() != 2 {
        return err_invalid(
            "Invalid duration syntax (second and microsecond part)",
            s,
            line,
        );
    }

    fn parse<T: std::str::FromStr>(s: &str, line: usize) -> io::Result<T>
    where
        <T as std::str::FromStr>::Err: Display,
    {
        s.parse().map_err(|e| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("{} in {:?} (line {})", e, s, line),
            )
        })
    }
    let hour: u64 = parse(split[0], line)?;
    let min: u64 = parse(split[1], line)?;
    let sec: u64 = parse(second_part[0], line)?;
    let ms: u32 = parse(second_part[1], line)?;
    if ms > 999 {
        return err_invalid("microsecond greater than 999 ", s, line);
    }

    Ok(Duration::new(hour * 3600 + min * 60 + sec, ms * 1_000_000))
}
#[test]
fn test_parse_duration_test() {
    debug_assert_eq!(
        Duration::new(3600 + 23 * 60 + 17, 486 * 1_000_000),
        parse_duration("01:23:17,486", 0).unwrap()
    );
}

/// Create a io::Result with an error where the error kind is InvalidData.
fn err_invalid<T>(because: &'static str, data: &str, line: usize) -> io::Result<T> {
    Err(io::Error::new(
        ErrorKind::InvalidData,
        format!("{} in {:?} (line {})", because, data, line),
    ))
}

/// Write all Cues from the input Iterator into the write W. Use SRT subtitle format.
/// Return the number fo writed cue.
pub fn out<I, W>(cues: I, mut w: W) -> Result<usize, std::io::Error>
where
    W: Write,
    I: std::iter::Iterator<Item = Cue>,
{
    let mut nb = 0;

    for c in cues {
        nb += 1;
        writeln!(w, "{}", nb)?;
        write_duration(&mut w, &c.begin)?;
        write!(w, " --> ")?;
        write_duration(&mut w, &c.end)?;
        writeln!(w, "")?;
        for l in c.text {
            writeln!(w, "{}", l)?;
        }
        writeln!(w, "")?;
    }

    Ok(nb)
}
#[test]
fn test_out() {
    fn dur(d: u64) -> Duration {
        Duration::new(d, 0)
    }

    let cues = vec![
        Cue::new(
            None,
            dur(1),
            dur(4),
            vec!["Never drink liquid nitrogen.".to_string()],
        ),
        Cue::new(
            None,
            dur(5),
            dur(9),
            vec![
                "— It will perforate your stomach.".to_string(),
                "— You could die.".to_string(),
            ],
        ),
    ];

    let mut output: Vec<u8> = Vec::new();
    assert_eq!(2, out(cues.into_iter(), &mut output).unwrap());
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "1
00:00:01,000 --> 00:00:04,000
Never drink liquid nitrogen.

2
00:00:05,000 --> 00:00:09,000
— It will perforate your stomach.
— You could die.

"
    );
}

/// Write one time code to a line.
fn write_duration<W: Write>(w: &mut W, d: &Duration) -> Result<(), io::Error> {
    let sec = d.as_secs();
    write!(
        w,
        "{:02}:{:02}:{:02},{:03}",
        sec / 3600,
        sec / 60 % 60,
        sec % 60,
        d.subsec_millis()
    )
}
#[test]
fn test_write_duration() {
    let d = Duration::new(2 * 3600 + 3 * 60 + 5, 84 * 1_000_000);
    let mut out: Vec<u8> = Vec::new();
    write_duration(&mut out, &d).unwrap();
    assert_eq!(std::str::from_utf8(&out).unwrap(), "02:03:05,084");
}
