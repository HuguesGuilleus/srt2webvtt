// Copyright (c) 2020, Hugues GUILLEUS <ghugues@netc.fr>. All rights reserved.
// Use of this source code is governed by a BSD
// license that can be found in the LICENSE file.

use super::{Cue, LineNb};
use std::io;
use std::io::{ErrorKind, Read, Write};
use std::time::Duration;

pub struct WebVTTParser<R: Read> {
    lines: LineNb<R>,
    error: Option<io::Error>,
    id: Option<String>,
}
impl<R: Read> WebVTTParser<R> {
    pub fn parse(r: R) -> io::Result<Self> {
        let mut lines = LineNb::new(r);

        match lines.next() {
            None => Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                "WebVTT file need a `WEBVTT` line header",
            )),
            Some(Err(e)) => Err(e),
            Some(Ok(l)) if !l.starts_with("WEBVTT") && l.starts_with("\u{FEFF}WEBVTT") => {
                Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "WebVTT file need a `WEBVTT` line header",
                ))
            }
            _ => Ok(()),
        }?;

        Ok(Self {
            lines: lines,
            id: None,
            error: None,
        })
    }
    /// Read lines while the line is not empty and no error come.
    fn next_while_empty(&mut self) {
        loop {
            match self.lines.next() {
                None => return,
                Some(Err(e)) => {
                    self.error = Some(e);
                    return;
                }
                Some(Ok(l)) if l.len() == 0 => return,
                _ => {}
            }
        }
    }
    fn parse_cue(&mut self, first: &str, line: usize) -> io::Result<Cue> {
        let (size, begin) = parse_duration(first, line)?;
        let (_, end) = parse_duration(
            first[size..]
                .trim_start()
                .trim_start_matches("-->")
                .trim_start(),
            line,
        )?;

        let mut lines = vec![];
        loop {
            match self.lines.next() {
                Some(Err(e)) => return Err(e),
                None => break,
                Some(Ok(l)) if l.len() == 0 => break,
                Some(Ok(l)) => lines.push(l),
            }
        }

        let id = match std::mem::replace(&mut self.id, None) {
            Some(id) if id.chars().any(|c| !c.is_numeric()) => Some(id),
            _ => None,
        };

        Ok(Cue::new(id, begin, end, lines))
    }
}
impl<R: Read> Iterator for WebVTTParser<R> {
    type Item = io::Result<Cue>;
    fn next(&mut self) -> Option<io::Result<Cue>> {
        if self.error.is_some() {
            return None;
        }

        match &self.id {
            Some(id) => match self.lines.next() {
                Some(Ok(l)) if l.contains("-->") => {
                    match self.parse_cue(&l, self.lines.current()) {
                        Ok(c) => Some(Ok(c)),
                        Err(e) => {
                            self.error = Some(e);
                            None
                        }
                    }
                }
                Some(Err(e)) => {
                    self.error = Some(e);
                    None
                }
                _ => {
                    self.error = Some(io::Error::new(
                        ErrorKind::InvalidData,
                        format!("A alone text line {:?} (line: ?)", id),
                    ));
                    return None;
                }
            },
            None => match self.lines.next() {
                None => None,
                Some(Err(e)) => {
                    self.error = Some(e);
                    None
                }
                Some(Ok(l)) if l.len() == 0 => self.next(),
                Some(Ok(l))
                    if l.starts_with("REGION")
                        || l.starts_with("NOTE")
                        || l.starts_with("STYLE") =>
                {
                    self.next_while_empty();
                    self.next()
                }
                Some(Ok(l)) if l.len() == 0 => self.next(),
                Some(Ok(l)) if !l.contains("-->") => {
                    self.id = Some(l);
                    self.next()
                }
                Some(Ok(l)) => match self.parse_cue(&l, self.lines.current()) {
                    Ok(c) => Some(Ok(c)),
                    Err(e) => {
                        self.error = Some(e);
                        None
                    }
                },
            },
        }
    }
}
#[test]
fn parser() {
    let mut p = WebVTTParser::parse(
        "WEBVTT - A good webvtt file

REGION
id:editor-comments
regionanchor:0%,0%
viewportanchor:0%,0%

NOTE Hello World

NOTE
Lorem ipsum dolor sit amet, consectetur adipisicing
elit, sed do eiusmod tempor incididunt ut labore

STYLE
::cue(b) {
	color: red;
}

1
00:01.000 --> 00:04.000 line:63% position:72% align:start
Never drink liquid nitrogen.

identifier
00:05.000 --> 00:09.000
— It will perforate your stomach.
— You could die."
            .as_bytes(),
    )
    .unwrap();

    assert_eq!(
        p.next().unwrap().unwrap(),
        Cue::new(
            None,
            Duration::new(1, 0),
            Duration::new(4, 0),
            vec![String::from("Never drink liquid nitrogen.")],
        )
    );

    assert_eq!(
        p.next().unwrap().unwrap(),
        Cue::new(
            Some(String::from("identifier")),
            Duration::new(5, 0),
            Duration::new(9, 0),
            vec![
                String::from("— It will perforate your stomach."),
                String::from("— You could die."),
            ],
        )
    );
}

/// Parse the duration of the line line. Return the string readed length and the Duration.
fn parse_duration(s: &str, line: usize) -> io::Result<(usize, Duration)> {
    let len = match s.find('.') {
        None => {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("Not found '.' for duration milliseconds (line {})", line),
            ));
        }
        Some(l) => l,
    };

    let millis: u32 = match s.get(len + 1..len + 4).map(|s| s.parse::<u32>()) {
        Some(Ok(n)) => n * 1_000_000,
        None => {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Need 3 digit after the dot for milliseconds (Parse duration, line {})",
                    line
                ),
            ))
        }
        Some(Err(err)) => {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("{} on {:?} (Parse duration, line {})", err, s, line),
            ))
        }
    };

    let hhmmss = s[..len].split(':');
    match hhmmss.clone().count() {
        2 | 3 => {}
        _ => return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!(
                "Wrong duration format (expected hh:mm:ss.ttt or mm:ss.ttt) on {:?} (Parse duration, line {})",
                s, line
            ),
        )),
    }
    let mut secs: u64 = 0;
    for ss in hhmmss {
        secs = secs * 60
            + ss.parse::<u64>().map_err(|err| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    format!("{} on {:?} (Parse duration, line {})", err, s, line),
                )
            })?;
    }

    Ok((len + 4, Duration::new(secs, millis)))
}
#[test]
fn test_parse_duration() {
    assert_eq!(
        (9, Duration::new(13 * 60 + 16, 500_000_000)),
        parse_duration("13:16.500", 0).unwrap()
    );
    assert_eq!(
        (14, Duration::new(7892 * 3600 + 13 * 60 + 16, 500_000_000)),
        parse_duration("7892:13:16.500", 0).unwrap()
    );
}

/// Write all Cues from the input Iterator into the write W. Return the number fo writed cue.
pub fn out<I, W>(cues: I, mut w: W) -> Result<usize, std::io::Error>
where
    W: Write,
    I: std::iter::Iterator<Item = Cue>,
{
    w.write(b"WEBVTT\n\n")?;

    let mut nb = 0;
    for c in cues {
        if let Some(id) = c.id {
            write!(w, "{}\n", id)?;
        }
        write_duration(&mut w, &c.begin)?;
        w.write(b" --> ")?;
        write_duration(&mut w, &c.end)?;
        w.write(b"\n")?;
        for l in c.text {
            write!(w, "{}\n", l)?;
        }
        w.write(b"\n")?;
        nb += 1;
    }

    Ok(nb)
}
#[test]
fn test_out() {
    let mut output: Vec<u8> = Vec::new();

    fn dur(d: u64) -> Duration {
        Duration::new(d, 0)
    }

    assert_eq!(
        out(
            vec![
                Cue::new(None, dur(0), dur(05), vec![String::from("Hello World")]),
                Cue::new(
                    Some("Yolo".to_string()),
                    dur(5),
                    dur(10),
                    vec![
                        String::from("J'espère que tous le monde va bien."),
                        String::from("On va commencer."),
                    ],
                ),
            ]
            .into_iter(),
            &mut output,
        )
        .unwrap(),
        2
    );

    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        "WEBVTT

00:00.000 --> 00:05.000
Hello World

Yolo
00:05.000 --> 00:10.000
J'espère que tous le monde va bien.
On va commencer.

"
    );
}

fn write_duration<W: Write>(w: &mut W, d: &Duration) -> Result<(), std::io::Error> {
    let sec = d.as_secs();
    let min = sec / 3600;
    if min == 0 {
        write!(
            w,
            "{:02}:{:02}.{:03}",
            sec / 60 % 60,
            sec % 60,
            d.subsec_millis()
        )
    } else {
        write!(
            w,
            "{:02}:{:02}:{:02}.{:03}",
            min,
            sec / 60 % 60,
            sec % 60,
            d.subsec_millis()
        )
    }
}
#[test]
fn test_write_duration() {
    let mut out: Vec<u8> = Vec::new();
    let d = Duration::new(3 * 60 + 5, 84 * 1_000_000);
    write_duration(&mut out, &d).unwrap();
    assert_eq!(std::str::from_utf8(&out).unwrap(), "03:05.084");

    let mut out: Vec<u8> = Vec::new();
    let d = Duration::new(2 * 3600 + 3 * 60 + 5, 84 * 1_000_000);
    write_duration(&mut out, &d).unwrap();
    assert_eq!(std::str::from_utf8(&out).unwrap(), "02:03:05.084");
}
