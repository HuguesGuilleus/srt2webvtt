// Copyright (c) 2020, Hugues GUILLEUS <ghugues@netc.fr>. All rights reserved.
// Use of this source code is governed by a BSD
// license that can be found in the LICENSE file.

use crate::lib::Cue;
use std::io;
use std::time::Duration;

pub fn write_head<W: io::Write>(w: &mut W) -> io::Result<()> {
    writeln!(w, "WEBVTT")?;
    writeln!(w, "")
}

pub fn write<W: io::Write>(w: &mut W, c: Cue) -> io::Result<()> {
    write_time(w, c.begin)?;
    write!(w, " --> ")?;
    write_time(w, c.end)?;
    write!(w, "\r\n")?;

    for t in c.text {
        writeln!(w, "{}", t)?;
    }
    writeln!(w, "")
}
fn write_time<W: io::Write>(w: &mut W, d: Duration) -> io::Result<()> {
    let s = d.as_secs();
    write!(
        w,
        "{:02}:{:02}:{:02}.{:03}",
        s / 3600,
        (s / 60) % 60,
        s % 60,
        d.subsec_millis()
    )
}
