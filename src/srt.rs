// Copyright (c) 2020, Hugues GUILLEUS <ghugues@netc.fr>. All rights reserved.
// Use of this source code is governed by a BSD
// license that can be found in the LICENSE file.

use super::{Cue, LineNb};
use std::fmt::Display;
use std::io::{self, BufReader, ErrorKind, Read};
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
            "1
00:00:05,542 --> 00:00:07,792
Hello
World
"
            .as_bytes(),
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
