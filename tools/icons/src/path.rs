//! The `d` attribute: SVG's path grammar, read into absolute commands.
//!
//! The whole grammar, not the part Lucide happens to use today. Every letter is
//! here — `M L H V C S Q T A Z` and each one's relative twin — because a parser
//! that silently ignores an unknown letter turns a version bump into an icon
//! that draws most of itself, and the missing bit is a stroke nobody sees go.
//! An unreadable path is an error the vendoring run stops on.
//!
//! What comes out is absolute and reduced: relative coordinates resolved,
//! horizontal and vertical shorthands expanded, smooth shorthands given their
//! reflected control point, arcs turned into cubics by [`crate::geometry`]. The
//! compositor reads none of this — it reads the blob.

use crate::geometry::{Arc, Command, Point, arc};

/// Reads one `d` attribute.
pub fn parse(d: &str) -> Result<Vec<Command>, String> {
    let mut scanner = Scanner {
        bytes: d.as_bytes(),
        at: 0,
    };
    let mut pen = Pen::default();
    let mut commands = Vec::new();
    let mut letter = None;

    loop {
        scanner.skip_separators();
        let Some(next) = scanner.peek() else { break };
        if next.is_ascii_alphabetic() {
            scanner.at += 1;
            letter = Some(next);
        } else if letter.is_none() {
            return Err(format!(
                "path starts with '{}', not a command",
                next as char
            ));
        }
        let Some(current) = letter else { break };
        step(current, &mut scanner, &mut pen, &mut commands)?;
        // A repeated coordinate set continues the same command, except after a
        // move, where the grammar says the rest are lines.
        letter = Some(match current {
            b'M' => b'L',
            b'm' => b'l',
            other => other,
        });
    }
    Ok(commands)
}

/// Where the pen is, and what it needs to remember to read the next command.
#[derive(Debug, Default, Clone, Copy)]
struct Pen {
    /// The point the next segment starts from.
    current: Point,
    /// Where the open contour began, which `Z` returns to.
    start: Point,
    /// The second control point of the last cubic, for `S` to reflect.
    cubic: Option<Point>,
    /// The control point of the last quadratic, for `T` to reflect.
    quad: Option<Point>,
}

impl Pen {
    /// A coordinate pair, made absolute if the command was a relative one.
    fn resolve(&self, relative: bool, point: Point) -> Point {
        if relative {
            (self.current.0 + point.0, self.current.1 + point.1)
        } else {
            point
        }
    }

    /// The control point a smooth shorthand starts from: the last one mirrored
    /// through the current point, or the current point when there is none.
    fn reflected(&self, previous: Option<Point>) -> Point {
        match previous {
            Some(point) => (
                2.0 * self.current.0 - point.0,
                2.0 * self.current.1 - point.1,
            ),
            None => self.current,
        }
    }
}

/// Reads one command's worth of numbers and appends what it draws.
fn step(
    letter: u8,
    scanner: &mut Scanner<'_>,
    pen: &mut Pen,
    commands: &mut Vec<Command>,
) -> Result<(), String> {
    let relative = letter.is_ascii_lowercase();
    let (cubic, quad) = (pen.cubic, pen.quad);
    pen.cubic = None;
    pen.quad = None;

    match letter.to_ascii_uppercase() {
        b'M' => {
            pen.current = pen.resolve(relative, scanner.point()?);
            pen.start = pen.current;
            commands.push(Command::Move(pen.current));
        }
        b'L' => {
            pen.current = pen.resolve(relative, scanner.point()?);
            commands.push(Command::Line(pen.current));
        }
        b'H' => {
            let x = scanner.number()?;
            pen.current.0 = if relative { pen.current.0 + x } else { x };
            commands.push(Command::Line(pen.current));
        }
        b'V' => {
            let y = scanner.number()?;
            pen.current.1 = if relative { pen.current.1 + y } else { y };
            commands.push(Command::Line(pen.current));
        }
        b'C' | b'S' => {
            let first = if letter.eq_ignore_ascii_case(&b'C') {
                pen.resolve(relative, scanner.point()?)
            } else {
                pen.reflected(cubic)
            };
            let second = pen.resolve(relative, scanner.point()?);
            pen.current = pen.resolve(relative, scanner.point()?);
            pen.cubic = Some(second);
            commands.push(Command::Cubic(first, second, pen.current));
        }
        b'Q' | b'T' => {
            let control = if letter.eq_ignore_ascii_case(&b'Q') {
                pen.resolve(relative, scanner.point()?)
            } else {
                pen.reflected(quad)
            };
            pen.current = pen.resolve(relative, scanner.point()?);
            pen.quad = Some(control);
            commands.push(Command::Quad(control, pen.current));
        }
        b'A' => {
            let radii = scanner.point()?;
            let rotation = scanner.number()?;
            let large = scanner.flag()?;
            let sweep = scanner.flag()?;
            let to = pen.resolve(relative, scanner.point()?);
            let shape = Arc {
                radii,
                rotation,
                large,
                sweep,
            };
            commands.extend(arc(pen.current, to, shape));
            pen.current = to;
        }
        b'Z' => {
            pen.current = pen.start;
            commands.push(Command::Close);
        }
        other => return Err(format!("'{}' is not a path command", other as char)),
    }
    Ok(())
}

/// A cursor over the `d` string's bytes.
struct Scanner<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Scanner<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    /// Whitespace and commas both separate numbers, and either may be absent
    /// where a sign or a decimal point already ends the previous one.
    fn skip_separators(&mut self) {
        while matches!(self.peek(), Some(b) if b.is_ascii_whitespace() || b == b',') {
            self.at += 1;
        }
    }

    fn point(&mut self) -> Result<Point, String> {
        Ok((self.number()?, self.number()?))
    }

    /// One number, in any of the spellings the grammar allows: `1`, `-1`,
    /// `.5`, `1e-3`, and `1.5.5` as two numbers rather than one.
    fn number(&mut self) -> Result<f64, String> {
        self.skip_separators();
        let from = self.at;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.at += 1;
        }
        let mut seen_point = false;
        while let Some(byte) = self.peek() {
            match byte {
                b'0'..=b'9' => self.at += 1,
                b'.' if !seen_point => {
                    seen_point = true;
                    self.at += 1;
                }
                b'e' | b'E' => {
                    self.at += 1;
                    if matches!(self.peek(), Some(b'+' | b'-')) {
                        self.at += 1;
                    }
                }
                _ => break,
            }
        }
        let text = std::str::from_utf8(&self.bytes[from..self.at])
            .map_err(|_| "path data is not UTF-8".to_string())?;
        text.parse()
            .map_err(|_| format!("'{text}' is not a number, at byte {from}"))
    }

    /// An arc flag: one digit, `0` or `1`, and never more — `a1 1 0 010 5` is
    /// two flags and a coordinate, not the number ten.
    fn flag(&mut self) -> Result<bool, String> {
        self.skip_separators();
        match self.peek() {
            Some(b'0') => {
                self.at += 1;
                Ok(false)
            }
            Some(b'1') => {
                self.at += 1;
                Ok(true)
            }
            found => Err(format!(
                "an arc flag is 0 or 1, found {:?} at byte {}",
                found.map(char::from),
                self.at
            )),
        }
    }
}
