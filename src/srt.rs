// Copyright (c) 2020, Hugues GUILLEUS <ghugues@netc.fr>. All rights reserved.
// Use of this source code is governed by a BSD
// license that can be found in the LICENSE file.

use std::fmt::Display;
use std::io;
use std::io::ErrorKind;
use std::time::Duration;

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
        parse_time("x --> 17:35:29.942 --> 17:25:48.456", 0).unwrap(),
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
