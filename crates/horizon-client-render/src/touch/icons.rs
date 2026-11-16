//! Pixel icons for the touch buttons: 28 small pictures a player can put on any button (the editor's Icon
//! picker), drawn as 12 x 12 cells with a soft shadow, the way Minecraft draws its text.
use super::*;

/// Colours of the cells: `#` white, `o` grey, `k` dark, `g` gold, `G` light gold, `r` red, `b` blue, `n` brown,
/// `e` green, `c` cyan, `p` pink; anything else is clear.
fn cell_color(c: char) -> Option<(u8, u8, u8)> {
    Some(match c {
        '#' => (255, 255, 255),
        'o' => (170, 170, 178),
        'k' => (60, 60, 70),
        'g' => (255, 200, 60),
        'G' => (255, 255, 150),
        'r' => (230, 70, 60),
        'b' => (90, 150, 255),
        'n' => (150, 100, 60),
        'e' => (90, 200, 90),
        'c' => (110, 230, 230),
        'p' => (255, 130, 180),
        _ => return None,
    })
}

/// How many icons there are; a button's `icon_id` 0 is "none", 1..=ICON_COUNT are these.
pub const ICON_COUNT: u8 = 28;

/// The icons, in picker order: name and 12 rows of 12 cells.
const ICONS: [(&str, [&str; 12]); 28] = [
    (
        "Sword",
        [
            "..........o#",
            ".........o##",
            "........o##o",
            ".......o##o.",
            "......o##o..",
            ".g...o##o...",
            ".gg.o##o....",
            "..ggg#o.....",
            "...ggg......",
            "..n.gg......",
            ".nn.........",
            "nn..........",
        ],
    ),
    (
        "Pickaxe",
        [
            "..########..",
            ".###oooo###.",
            "###..nn..###",
            "##...nn...##",
            "#....nn....#",
            ".....nn.....",
            ".....nn.....",
            ".....nn.....",
            ".....nn.....",
            ".....nn.....",
            ".....nn.....",
            ".....nn.....",
        ],
    ),
    (
        "Bow",
        [
            "..nn........",
            "...nn.......",
            "....n#......",
            ".....n#.....",
            "......n#....",
            "..#####n####",
            "......n#..#.",
            ".....n#..#..",
            "....n#......",
            "...nn.......",
            "..nn........",
            "............",
        ],
    ),
    (
        "Shield",
        [
            ".##########.",
            ".#oooooooo#.",
            ".#o######o#.",
            ".#o#bbbb#o#.",
            ".#o#bbbb#o#.",
            ".#o#bbbb#o#.",
            ".#o#bbbb#o#.",
            "..#o#bb#o#..",
            "...#o##o#...",
            "....#oo#....",
            ".....##.....",
            "............",
        ],
    ),
    (
        "Heart",
        [
            "............",
            ".rrr....rrr.",
            "rrrrr..rrrrr",
            "r#rrrrrrrrrr",
            "r#rrrrrrrrrr",
            "rrrrrrrrrrrr",
            ".rrrrrrrrrr.",
            "..rrrrrrrr..",
            "...rrrrrr...",
            "....rrrr....",
            ".....rr.....",
            "............",
        ],
    ),
    (
        "Apple",
        [
            ".......ee...",
            "......ee....",
            "...rr.n.rr..",
            "..rrrrnrrrr.",
            ".rr#rrrrrrrr",
            ".r#rrrrrrrrr",
            ".rrrrrrrrrrr",
            ".rrrrrrrrrrr",
            "..rrrrrrrrr.",
            "..rrrrrrrrr.",
            "...rrr.rrr..",
            "............",
        ],
    ),
    (
        "Chest",
        [
            "............",
            ".nnnnnnnnnn.",
            "nnnnnnnnnnnn",
            "nonnnnnnnnon",
            "nnnnnggnnnnn",
            "kkkkkggkkkkk",
            "nnnnnggnnnnn",
            "nonnnnnnnnon",
            "nnnnnnnnnnnn",
            "nnnnnnnnnnnn",
            ".nnnnnnnnnn.",
            "............",
        ],
    ),
    (
        "Block",
        [
            "eeeeeeeeeeee",
            "eeeeeeeeeeee",
            "eneeeneeeene",
            "nnnnnnnnnnnn",
            "nnknnnnnnknn",
            "nnnnnnknnnnn",
            "nnnnnnnnnnnk",
            "nknnnnnnnnnn",
            "nnnnnnknnnnn",
            "nnnnknnnnnnk",
            "nknnnnnnnknn",
            "nnnnnnnnnnnn",
        ],
    ),
    (
        "Pick Block",
        [
            "........g...",
            "........g...",
            "......ggggg.",
            "........g...",
            "eeeeeeeeg...",
            "eeeeeeee....",
            "nennenne....",
            "nnnnnnnn....",
            "nnknnnkn....",
            "nnnnnknn....",
            "nknnnnnn....",
            "nnnnnnnn....",
        ],
    ),
    (
        "Drop",
        [
            "....#.......",
            "....#.......",
            "..#.#.#.....",
            "...###......",
            "....#.......",
            "............",
            "nnnnnnnnnnnn",
            "n#n#n#n#n#nn",
            "nnnnnnnnnnnn",
            "nnnnnnnnnnnn",
            "............",
            "............",
        ],
    ),
    (
        "Swap",
        [
            "............",
            "...##.......",
            "..###.......",
            ".##########.",
            "..###.......",
            "...##.......",
            ".......##...",
            ".......###..",
            ".##########.",
            ".......###..",
            ".......##...",
            "............",
        ],
    ),
    (
        "Eye",
        [
            "............",
            "....####....",
            "..##....##..",
            ".#...cc...#.",
            "#...cc##...#",
            "#..cc####..#",
            "#...cc##...#",
            ".#...cc...#.",
            "..##....##..",
            "....####....",
            "............",
            "............",
        ],
    ),
    (
        "Chat",
        [
            "............",
            ".##########.",
            "#oooooooooo#",
            "#o########o#",
            "#oooooooooo#",
            "#o######ooo#",
            "#oooooooooo#",
            ".####o######",
            "....#o#.....",
            ".....##.....",
            "............",
            "............",
        ],
    ),
    (
        "Map",
        [
            "............",
            ".nn########.",
            "n##oooooooo#",
            "n#o#.ee..#o#",
            "n#o#.eee.#o#",
            "n#o#..e.rr#o",
            "n#o#.#...rr#",
            "n#o#.#e..#o#",
            "n#oo.#.ee#o#",
            "n###########",
            ".nn.........",
            "............",
        ],
    ),
    (
        "Compass",
        [
            "....####....",
            "..##oooo##..",
            ".#o..r...o#.",
            ".#o..rr..o#.",
            "#o...rr...o#",
            "#o...##...o#",
            "#o..b##...o#",
            ".#o.bb...o#.",
            ".#o..b...o#.",
            "..##oooo##..",
            "....####....",
            "............",
        ],
    ),
    (
        "Star",
        [
            ".....gg.....",
            ".....gg.....",
            "....gggg....",
            "gggggggggggg",
            ".gggg##gggg.",
            "..gggggggg..",
            "...gggggg...",
            "...ggggggg..",
            "..ggg..ggg..",
            "..gg....gg..",
            ".g........g.",
            "............",
        ],
    ),
    (
        "Bolt",
        [
            ".......###..",
            "......###...",
            ".....###....",
            "....###.....",
            "...#######..",
            ".....###....",
            "....###.....",
            "...###......",
            "..###.......",
            "..##........",
            ".#..........",
            "............",
        ],
    ),
    (
        "Feather",
        [
            "........####",
            "......######",
            ".....#######",
            "....o#######",
            "...oo######.",
            "..ooo#####..",
            ".ooo#####...",
            ".oo#####....",
            "oo####......",
            "o###........",
            "#...........",
            "............",
        ],
    ),
    (
        "Flame",
        [
            ".....r......",
            "....rr......",
            "....rrr..r..",
            "...rrrr.rr..",
            "..rrrggrrr..",
            "..rrgggrrrr.",
            ".rrrgg#grrr.",
            ".rrgg###grr.",
            ".rrgg###grr.",
            "..rrgggggr..",
            "...rrrrrr...",
            "....rrrr....",
        ],
    ),
    (
        "Skull",
        [
            "............",
            "..########..",
            ".##########.",
            "############",
            "##kk####kk##",
            "##kk####kk##",
            "############",
            ".####kk####.",
            "..########..",
            "..#.#..#.#..",
            "..#.#..#.#..",
            "............",
        ],
    ),
    (
        "Bed",
        [
            "............",
            "............",
            "rrrrrr......",
            "r#rrrr#####.",
            "rrrrrr######",
            "############",
            "nnnnnnnnnnnn",
            "nnnnnnnnnnnn",
            "n..........n",
            "n..........n",
            "............",
            "............",
        ],
    ),
    (
        "Camera",
        [
            "............",
            "...######...",
            ".##########.",
            "#oooooooooo#",
            "#o..####..o#",
            "#o.##oo##.o#",
            "#o.#o##o#.o#",
            "#o.##oo##.o#",
            "#o..####..o#",
            "#oooooooooo#",
            ".##########.",
            "............",
        ],
    ),
    (
        "Gear",
        [
            ".....##.....",
            "..#..##..#..",
            "..########..",
            "...######...",
            ".###....###.",
            "####....####",
            "####....####",
            ".###....###.",
            "...######...",
            "..########..",
            "..#..##..#..",
            ".....##.....",
        ],
    ),
    (
        "Arrow Up",
        [
            "............",
            ".....##.....",
            "....####....",
            "...######...",
            "..########..",
            ".##########.",
            "....####....",
            "....####....",
            "....####....",
            "....####....",
            "............",
            "............",
        ],
    ),
    (
        "Arrow Down",
        [
            "............",
            "............",
            "....####....",
            "....####....",
            "....####....",
            "....####....",
            ".##########.",
            "..########..",
            "...######...",
            "....####....",
            ".....##.....",
            "............",
        ],
    ),
    (
        "Torch",
        [
            ".....g......",
            "....ggg.....",
            "....gGg.....",
            "....ggg.....",
            ".....rr.....",
            ".....nn.....",
            ".....nn.....",
            ".....nn.....",
            ".....nn.....",
            ".....nn.....",
            ".....nn.....",
            ".....nn.....",
        ],
    ),
    (
        "Potion",
        [
            "....nnnn....",
            "....#oo#....",
            ".....##.....",
            ".....##.....",
            "....####....",
            "...#pppp#...",
            "..#pppppp#..",
            "..#pp##ppp#.",
            "..#pppppp#..",
            "..#pppppp#..",
            "...######...",
            "............",
        ],
    ),
    (
        "Home",
        [
            ".....##.....",
            "....####....",
            "...######...",
            "..########..",
            ".##########.",
            "############",
            ".nnnnnnnnnn.",
            ".nnn#nnnn#n.",
            ".nnn#nggn#n.",
            ".nnnnnggnnn.",
            ".nnnnnnnnnn.",
            "............",
        ],
    ),
];

/// The name of icon `id` (1-based), or "Default" for 0.
pub fn icon_name(id: u8) -> &'static str {
    match id {
        0 => "Default",
        id => ICONS.get(usize::from(id) - 1).map_or("Default", |icon| icon.0),
    }
}

/// Draws icon `id` (1-based) centred in `rect`, as large as fits in whole-or-half GUI pixels, with a shadow.
/// `alpha` is 0..=255.
pub fn draw_icon(b: &mut HudBatch, a: &HudAtlas, id: u8, rect: [f32; 4], alpha: u8) {
    let Some((_, rows)) = usize::from(id).checked_sub(1).and_then(|i| ICONS.get(i)) else {
        return;
    };
    let side = rect[2].min(rect[3]);
    let u = ((side * 0.64 / 12.0) * 2.0).floor() / 2.0;
    let u = u.max(1.0);
    let x0 = (rect[0] + (rect[2] - 12.0 * u) / 2.0).round();
    let y0 = (rect[1] + (rect[3] - 12.0 * u) / 2.0).round();
    let shadow = (u * 0.5).max(1.0).round();
    for pass in [true, false] {
        for (y, row) in rows.iter().enumerate() {
            let cells: Vec<char> = row.chars().collect();
            let mut x = 0;
            while x < cells.len() {
                let Some(rgb) = cell_color(cells[x]) else {
                    x += 1;
                    continue;
                };
                let start = x;
                while x < cells.len() && cells[x] == cells[start] {
                    x += 1;
                }
                let (color, dx) = if pass {
                    (rgba(10, 10, 14, (u16::from(alpha) * 150 / 255) as u8), shadow)
                } else {
                    (rgba(rgb.0, rgb.1, rgb.2, alpha), 0.0)
                };
                b.fill(
                    a,
                    x0 + start as f32 * u + dx,
                    y0 + y as f32 * u + dx,
                    (x - start) as f32 * u,
                    u,
                    color,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_twelve_by_twelve() {
        for (name, rows) in ICONS {
            assert!(rows.iter().all(|row| row.chars().count() == 12), "{name}");
        }
        assert_eq!(usize::from(ICON_COUNT), ICONS.len());
    }
}
