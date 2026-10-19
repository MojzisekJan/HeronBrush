//! Reading a compiler's portal file.
//!
//! Reads compiler `.prt` files containing convex portal polygons between visibility regions. Their
//! outlines help inspect room boundaries and visibility partitioning.
//!
//! Four dialects, all of them text and all of them ending in the same lines: a count of
//! points and then that many `(x y z)` triples, with the numbers that come before them
//! differing by compiler. The parentheses are separators like any other, which is what
//! lets one reader take all four.

use geometry_math::vec::Vec3d;

/// The portals a compiler wrote.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Portals {
    /// One convex polygon per portal, in the order they were written.
    pub polygons: Vec<Vec<Vec3d>>,
}

/// Why a portal file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortalError {
    /// The first line was not one of the formats this knows.
    UnknownFormat(String),
    /// The header ended before it said how many portals there are.
    ShortHeader,
    /// A portal line was missing, or held fewer points than it said it did.
    ShortPortal(usize),
}

impl std::fmt::Display for PortalError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortalError::UnknownFormat(code) => write!(out, "unknown portal format: {code}"),
            PortalError::ShortHeader => write!(out, "the header ends before the portal count"),
            PortalError::ShortPortal(at) => write!(out, "portal {at} is not all there"),
        }
    }
}

impl std::error::Error for PortalError {}

/// Reads a portal file.
///
/// `loadPortalFile`. The header says how many portals follow and how many numbers each
/// line carries before its points; everything after the count is the same in all four
/// dialects.
pub fn read(text: &str) -> Result<Portals, PortalError> {
    let mut lines = text.lines();
    let format = lines.next().unwrap_or_default().trim();

    // How many lines of the header come after the format, and how many numbers a portal
    // line carries before its points. Quake 3's PRT1 writes one more -- the number of
    // solid faces -- which is what the extra line after the header gives away.
    let skip = match format {
        "PRT1" | "PRT1-AM" => 2,
        "PRT2" => 3,
        other => return Err(PortalError::UnknownFormat(other.to_string())),
    };
    // The point count and the two leaf or cluster numbers, which every dialect writes
    // before the points themselves.
    let mut first_point = 3;

    let mut header = Vec::new();
    for _ in 0..skip {
        header.push(lines.next().ok_or(PortalError::ShortHeader)?);
    }
    // The last line of the header is the portal count in every dialect but `PRT1-AM`,
    // which puts the leaf count after it and is read the same way regardless: the count is
    // the line before the portals.
    let portals: usize = header
        .last()
        .and_then(|line| line.trim().parse().ok())
        .ok_or(PortalError::ShortHeader)?;

    let mut lines = lines.peekable();
    if format == "PRT1-AM" {
        // The leaf count, which is of no use here.
        lines.next();
    }

    // A Quake 3 `PRT1` writes the number of solid faces on its own line before the
    // portals; a line with one number on it and nothing that looks like a point is that.
    if format == "PRT1" {
        if let Some(next) = lines.peek() {
            if fields(next).len() == 1 {
                lines.next();
                // Its portal lines carry a third leaf number before the points.
                first_point += 1;
            }
        }
    }

    let mut polygons = Vec::with_capacity(portals);
    for at in 0..portals {
        let line = lines.next().ok_or(PortalError::ShortPortal(at))?;
        let numbers = fields(line);
        if numbers.len() < 3 {
            return Err(PortalError::ShortPortal(at));
        }
        let points: usize = numbers[0].parse().map_err(|_| PortalError::ShortPortal(at))?;

        let mut polygon = Vec::with_capacity(points);
        let mut cursor = first_point;
        for _ in 0..points {
            let number = |at: usize| -> Result<f64, PortalError> {
                numbers
                    .get(at)
                    .and_then(|word| word.parse().ok())
                    .ok_or(PortalError::ShortPortal(at))
            };
            polygon.push(Vec3d::new(
                number(cursor)?,
                number(cursor + 1)?,
                number(cursor + 2)?,
            ));
            cursor += 3;
        }
        polygons.push(polygon);
    }

    Ok(Portals { polygons })
}

/// The numbers on one line, with the parentheses treated as the separators they are.
fn fields(line: &str) -> Vec<&str> {
    line.split(['(', ')', ' ', '\t', '\r'])
        .filter(|word| !word.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quake_prt1_gives_one_polygon_per_portal() {
        let text = "PRT1\n4\n2\n\
                    4 0 1 (0 0 0 ) (0 0 64 ) (64 0 64 ) (64 0 0 )\n\
                    3 1 2 (0 0 0 ) (0 64 0 ) (64 0 0 )\n";
        let portals = read(text).expect("it reads");
        assert_eq!(portals.polygons.len(), 2);
        assert_eq!(portals.polygons[0].len(), 4);
        assert_eq!(portals.polygons[0][2], Vec3d::new(64.0, 0.0, 64.0));
        assert_eq!(portals.polygons[1].len(), 3);
    }

    #[test]
    fn a_quake_3_prt1_has_one_more_number_before_its_points() {
        // The line after the header holds the number of solid faces, which is what says
        // this is Quake 3's dialect of the same format.
        let text = "PRT1\n4\n1\n17\n\
                    3 1 2 3 (0 0 0 ) (0 64 0 ) (64 0 0 )\n";
        let portals = read(text).expect("it reads");
        assert_eq!(portals.polygons.len(), 1);
        assert_eq!(portals.polygons[0][1], Vec3d::new(0.0, 64.0, 0.0));
    }

    #[test]
    fn a_prt2_skips_its_extra_header_line() {
        let text = "PRT2\n4\n2\n1\n\
                    3 1 2 (0 0 0 ) (0 64 0 ) (64 0 0 )\n";
        let portals = read(text).expect("it reads");
        assert_eq!(portals.polygons.len(), 1);
        assert_eq!(portals.polygons[0].len(), 3);
    }

    #[test]
    fn something_that_is_not_a_portal_file_is_an_error() {
        assert_eq!(
            read("BSP2\n"),
            Err(PortalError::UnknownFormat("BSP2".to_string()))
        );
        assert!(matches!(read("PRT1\n4\n2\n"), Err(PortalError::ShortPortal(0))));
    }
}
