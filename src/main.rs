// Copyright (c) 2020, Hugues GUILLEUS <ghugues@netc.fr>. All rights reserved.
// Use of this source code is governed by a BSD
// license that can be found in the LICENSE file.

mod lib;

fn main() -> Result<(), String> {
    let before = std::time::Instant::now();

    for f in std::env::args().skip(1) {
        println!("{}", f);
        lib::str2vtt_file(&f).map_err(|e| e.to_string())?;
    }

    println!("Duration: {:?}", before.elapsed());

    Ok(())
}
