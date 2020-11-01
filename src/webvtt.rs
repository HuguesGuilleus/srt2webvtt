// Copyright (c) 2020, Hugues GUILLEUS <ghugues@netc.fr>. All rights reserved.
// Use of this source code is governed by a BSD
// license that can be found in the LICENSE file.

use super::Cue;
use std::io::Write;
use std::time::Duration;

/// Write all Cues from the input Iterator into the write W. Return the number fo writed cue.
pub fn out<I, W>(cues: I, mut w: W) -> Result<usize, std::io::Error>
where
    W: Write,
    I: std::iter::Iterator<Item = Cue>,
{
    w.write(b"WEBVTT\n\n")?;

    let mut nb = 0;
    for c in cues {
        write_duration(&mut w, &c.begin)?;
        w.write(b" --> ")?;
        write_duration(&mut w, &c.end)?;
        w.write(b"\n")?;
        for l in c.text {
            w.write(l.as_bytes())?;
            w.write(b"\n")?;
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
                Cue::new(dur(0), dur(05), vec![String::from("Hello World")]),
                Cue::new(
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
