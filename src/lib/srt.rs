// Copyright (c) 2020, Hugues GUILLEUS <ghugues@netc.fr>. All rights reserved.
// Use of this source code is governed by a BSD
// license that can be found in the LICENSE file.

use crate::lib::Cue;
use std::io;
use std::io::{BufRead, BufReader};
use std::iter::Iterator;
use std::time::Duration;

pub struct Srt<R: std::io::Read> {
    input: io::Lines<io::BufReader<R>>,
    end: bool,
    pub error: Option<io::Error>,
}
impl<R: std::io::Read> Srt<R> {
    pub fn parse(input: R) -> Srt<R> {
        Srt {
            input: BufReader::new(input).lines(),
            error: None,
            end: false,
        }
    }
}
impl<R: std::io::Read> Iterator for Srt<R> {
    type Item = Cue;
    fn next(&mut self) -> Option<Self::Item> {
        if self.end || self.error.is_some() {
            return None;
        }

        let mut lines: Vec<String> = Vec::new();
        loop {
            match self.input.next() {
                Some(l) => match l {
                    Ok(s) => {
                        if !s.is_empty() {
                            lines.push(String::from(s));
                        } else {
                            break;
                        }
                    }
                    Err(e) => {
                        self.error = Some(e);
                        return None;
                    }
                },
                None => {
                    self.end = true;
                    break;
                }
            };
        }

        if lines.len() < 3 {
            return self.next();
        }

        match parse_one(lines) {
            Ok(c) => Some(c),
            Err(e) => {
                self.error = Some(io::Error::new(io::ErrorKind::InvalidData, e));
                None
            }
        }
    }
}

fn parse_one(lines: Vec<String>) -> Result<Cue, String> {
    if lines.len() < 3 {
        return Err(format!("Cue {:?} is too short.", lines));
    } else if lines.len() > 4 {
        return Err(format!("Cue {:?} is too long.", lines));
    }

    if !only_digit(&lines[0]) {
        return Err(format!("Cue {:?} number contains invalid digit.", lines[0]));
    }

    let t = parse_time(&lines[1])?;

    Ok(Cue::new(t.0, t.1, (&lines[2..]).to_vec()))
}

fn parse_time(s: &str) -> Result<(Duration, Duration), String> {
    let split: Vec<&str> = s.split(" --> ").collect();
    if split.len() != 2 {
        return Err(format!("Invalid syntax time line on {:?}", s));
    }

    Ok((
        parse_duration(split[0].trim_end())?,
        parse_duration(split[1].trim_start())?,
    ))
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    let split: Vec<&str> = s.split(":").collect();
    if split.len() != 3 {
        return Err(format!("Invalid duration syntax in {:?}", s));
    }

    let second_part: Vec<&str> = split[2].split(",").collect();
    if second_part.len() != 2 {
        return Err(format!(
            "Invalid duration syntax (second and microsecond part) in {:?}",
            s
        ));
    }

    let hour: u64 = split[0]
        .parse()
        .map_err(|e| format!("Invalid integer format in {:?} (hour part): {}", s, e))?;
    let min: u64 = split[1]
        .parse()
        .map_err(|e| format!("Invalid integer format in {:?} (minute part): {}", s, e))?;
    let sec: u64 = second_part[0]
        .parse()
        .map_err(|e| format!("Invalid integer format in {:?} (second part): {}", s, e))?;
    let ms: u32 = second_part[1].parse().map_err(|e| {
        format!(
            "Invalid integer format in {:?} (microsecond part): {}",
            s, e
        )
    })?;
    if ms > 999 {
        return Err(format!(
            "Invalid duration in {:?}, microsecond greater than 999",
            s
        ));
    }

    Ok(Duration::new(hour * 3600 + min * 60 + sec, ms * 1_000_000))
}
#[test]
fn test_parse_duration_test() {
    debug_assert_eq!(
        Ok(Duration::new(3600 + 23 * 60 + 17, 486 * 1_000_000)),
        parse_duration("01:23:17,486")
    )
}

fn only_digit(s: &str) -> bool {
    s.chars().all(|c: char| '0' <= c && c <= '9')
}
#[test]
fn test_only_digit() {
    assert_eq!(only_digit("48é"), false);
    assert_eq!(only_digit("0123456789"), true);
}
