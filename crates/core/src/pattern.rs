//! The Pattern library: known arrangements of live Cells to stamp into a
//! World.

/// A named, known arrangement of live Cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pattern {
    /// The smallest spaceship, moving one Cell diagonally every 4
    /// Generations.
    Glider,
    /// The lightweight spaceship, moving two Cells sideways every 4
    /// Generations.
    Lwss,
    /// A five-Cell methuselah that takes 1103 Generations to settle on an
    /// unbounded plane.
    RPentomino,
    /// The Gosper glider gun, firing a glider every 30 Generations.
    GosperGun,
}

impl Pattern {
    /// Every Pattern in the library.
    pub const ALL: [Pattern; 4] = [
        Pattern::Glider,
        Pattern::Lwss,
        Pattern::RPentomino,
        Pattern::GosperGun,
    ];

    /// The Pattern's name in kebab case, such as `"gosper-gun"`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Glider => "glider",
            Self::Lwss => "lwss",
            Self::RPentomino => "r-pentomino",
            Self::GosperGun => "gosper-gun",
        }
    }

    /// The Pattern called `name`, as [`Pattern::name`] writes it.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|pattern| pattern.name() == name)
    }

    /// The Pattern's rows, top to bottom, with `O` for a live Cell and `.`
    /// for a dead one.
    fn rows(self) -> &'static [&'static str] {
        match self {
            Self::Glider => &[".O.", "..O", "OOO"],
            Self::Lwss => &[".O..O", "O....", "O...O", "OOOO."],
            Self::RPentomino => &[".OO", "OO.", ".O."],
            Self::GosperGun => &[
                "........................O...........",
                "......................O.O...........",
                "............OO......OO............OO",
                "...........O...O....OO............OO",
                "OO........O.....O...OO..............",
                "OO........O...O.OO....O.O...........",
                "..........O.....O.......O...........",
                "...........O...O....................",
                "............OO......................",
            ],
        }
    }

    /// Width and height of the Pattern's bounding box in Cells.
    pub fn size(self) -> (usize, usize) {
        let rows = self.rows();
        (rows[0].len(), rows.len())
    }

    /// Every Cell of the bounding box as `(x, y, live)`, row by row.
    pub(crate) fn cells(self) -> impl Iterator<Item = (usize, usize, bool)> {
        self.rows().iter().enumerate().flat_map(|(y, row)| {
            row.bytes()
                .enumerate()
                .map(move |(x, cell)| (x, y, cell == b'O'))
        })
    }
}
