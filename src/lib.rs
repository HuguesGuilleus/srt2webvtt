// Copyright (c) 2020, Hugues GUILLEUS <ghugues@netc.fr>. All rights reserved.
// Use of this source code is governed by a BSD
// license that can be found in the LICENSE file.

mod webvtt;
use std::time::Duration;

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
fn shift_applicator() {
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
