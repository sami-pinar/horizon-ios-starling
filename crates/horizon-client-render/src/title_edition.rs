//! "ERKAM STYLE" under the classic title screen's logo.
//!
//! Vanilla's `textures/gui/title/edition.png` reads "JAVA EDITION". Horizon
//! never ships Mojang's art, so the banner is rebuilt at load time: the
//! letters below are drawn with the shades the player's own texture uses
//! (a face with darker sides and a light bottom edge, a dark extrusion under
//! it, a black outline and a soft rim), sampled from the texture itself. Only
//! vanilla 26.3's exact texture is rebuilt: a resource pack's edition.png
//! stays as its author drew it.

use std::collections::{HashMap, VecDeque};
use std::ops::Range;

/// The atlas sprite.
pub const SPRITE: &str = "gui/title/edition";
const WIDTH: usize = 512;
const HEIGHT: usize = 64;
/// FNV-1a over the size and RGBA pixels of 26.3's edition.png.
const VANILLA: u64 = 0x4249_0538_6dfb_181f;
/// Where the texture's letters put their faces, and how deep they are drawn.
const FACE_TOP: usize = 4;
const EXTRUSION: usize = 8;
const OUTLINE: usize = 3;
/// The face-to-face spacing between letters, and the extra gap between the words.
const LETTER_GAP: usize = 6;
const WORD_GAP: usize = 24;
/// A letter is a 5 x 7 grid of cells, each this many texels wide; the face is `FACE_HEIGHT` tall.
const CELL_WIDTH: usize = 8;
const FACE_HEIGHT: usize = 39;

/// The banner's two words, in a 5 x 7 pixel font: `#` is a lit cell.
const WORDS: [&str; 2] = ["ERKAM", "STYLE"];

fn letter(c: char) -> [&'static str; 7] {
    match c {
        'E' => ["#####", "#....", "#....", "####.", "#....", "#....", "#####"],
        'R' => ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"],
        'K' => ["#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"],
        'A' => [".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
        'M' => ["#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"],
        'S' => [".####", "#....", "#....", ".###.", "....#", "....#", "####."],
        'T' => ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."],
        'Y' => ["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."],
        'L' => ["#....", "#....", "#....", "#....", "#....", "#....", "#####"],
        _ => ["....."; 7],
    }
}

/// The banner rebuilt to read "ERKAM STYLE", or `None` when the texture is
/// not vanilla's (a resource pack's own).
pub fn rust_edition(width: u32, height: u32, pixels: &[u8]) -> Option<Vec<u8>> {
    (fingerprint(width, height, pixels) == VANILLA).then(|| compose(pixels))
}

fn fingerprint(width: u32, height: u32, pixels: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let bytes = width.to_le_bytes().into_iter().chain(height.to_le_bytes());
    for b in bytes.chain(pixels.iter().copied()) {
        hash = (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

type Rgba = [u8; 4];

fn lum(p: Rgba) -> u32 {
    (u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])) / 3
}

fn pixel(pixels: &[u8], x: usize, y: usize) -> Rgba {
    let i = (y * WIDTH + x) * 4;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

/// The texture's shades: the most common colour in each brightness band.
struct Palette {
    highlight: Rgba,
    face: Rgba,
    rim: Rgba,
    extrusion: Rgba,
    edge: Rgba,
    outline: Rgba,
    soft: Rgba,
}

impl Palette {
    fn sample(pixels: &[u8]) -> Self {
        let mut bands: [HashMap<Rgba, u32>; 7] = Default::default();
        for &p in pixels.as_chunks::<4>().0 {
            let band = match (p[3], lum(p)) {
                (0, _) => continue,
                (255, 215..) => 0,
                (255, 170..) => 1,
                (255, 100..) => 2,
                (255, 45..) => 3,
                (255, 20..) => 4,
                (255, _) => 5,
                _ => 6,
            };
            *bands[band].entry(p).or_default() += 1;
        }
        let mode = |band: &HashMap<Rgba, u32>, fallback: Rgba| {
            band.iter()
                .max_by_key(|&(p, n)| (*n, std::cmp::Reverse(*p)))
                .map_or(fallback, |(p, _)| *p)
        };
        let outline = mode(&bands[5], [0, 0, 0, 255]);
        let face = mode(&bands[1], [200, 200, 200, 255]);
        Self {
            highlight: mode(&bands[0], face),
            face,
            rim: mode(&bands[2], face),
            extrusion: mode(&bands[3], outline),
            edge: mode(&bands[4], outline),
            outline,
            soft: mode(&bands[6], outline),
        }
    }
}

/// Faces of the letters, centred in the texture.
fn glyph_faces() -> Vec<bool> {
    let letter_width = 5 * CELL_WIDTH;
    let letters: usize = WORDS.iter().map(|w| w.len()).sum();
    let total = letters * letter_width + (letters - WORDS.len()) * LETTER_GAP + (WORDS.len() - 1) * WORD_GAP;
    let mut left = (WIDTH - total) / 2;
    let mut face = vec![false; WIDTH * HEIGHT];
    for (w, word) in WORDS.iter().enumerate() {
        if w > 0 {
            left += WORD_GAP;
        }
        for (i, c) in word.chars().enumerate() {
            if i > 0 {
                left += LETTER_GAP;
            }
            for (row, cells) in letter(c).iter().enumerate() {
                let (y0, y1) = (row * FACE_HEIGHT / 7, (row + 1) * FACE_HEIGHT / 7);
                for (col, cell) in cells.chars().enumerate() {
                    if cell == '#' {
                        for y in FACE_TOP + y0..FACE_TOP + y1 {
                            for x in left + col * CELL_WIDTH..left + (col + 1) * CELL_WIDTH {
                                face[y * WIDTH + x] = true;
                            }
                        }
                    }
                }
            }
            left += letter_width;
        }
    }
    face
}

/// Draw faces within `columns` the way the texture's letters are drawn.
fn draw_glyphs(out: &mut [u8], face: &[bool], palette: &Palette, columns: Range<usize>) {
    let at = |grid: &[bool], x: isize, y: isize| {
        (columns.start as isize..columns.end as isize).contains(&x)
            && (0..HEIGHT as isize).contains(&y)
            && grid[y as usize * WIDTH + x as usize]
    };
    let mut body = face.to_vec();
    for y in 0..HEIGHT {
        for x in columns.clone() {
            let (xi, yi) = (x as isize, y as isize);
            body[y * WIDTH + x] |= (1..=EXTRUSION as isize).any(|k| at(face, xi, yi - k));
        }
    }
    let reach = OUTLINE as isize + 1;
    for y in 0..HEIGHT {
        for x in columns.clone() {
            let (xi, yi) = (x as isize, y as isize);
            let colour = if at(face, xi, yi) {
                if !at(face, xi, yi + 1) {
                    palette.highlight
                } else if !at(face, xi - 1, yi) || !at(face, xi + 1, yi) {
                    palette.rim
                } else {
                    palette.face
                }
            } else if at(&body, xi, yi) {
                if at(face, xi, yi - 1) {
                    palette.rim
                } else if !at(&body, xi - 1, yi) || !at(&body, xi + 1, yi) || !at(&body, xi, yi + 1)
                {
                    palette.edge
                } else {
                    palette.extrusion
                }
            } else {
                let distance = (-reach..=reach)
                    .flat_map(|dy| (-reach..=reach).map(move |dx| (dx, dy)))
                    .filter(|&(dx, dy)| at(&body, xi + dx, yi + dy))
                    .map(|(dx, dy)| dx.abs().max(dy.abs()))
                    .min();
                match distance {
                    Some(d) if d <= OUTLINE as isize => palette.outline,
                    Some(_) => palette.soft,
                    None => continue,
                }
            };
            put(out, x, y, colour);
        }
    }
}

/// Paint `p` over what is there unless that is more opaque, or as opaque and
/// brighter (a letter's body wins over a neighbour's outline).
fn put(out: &mut [u8], x: usize, y: usize, p: Rgba) {
    let old = pixel(out, x, y);
    if (p[3], lum(p)) > (old[3], lum(old)) {
        out[(y * WIDTH + x) * 4..][..4].copy_from_slice(&p);
    }
}

/// Close the pockets that no gap reaches from outside, as
/// the texture's letters fill their counters with the outline.
fn fill_counters(out: &mut [u8], outline: Rgba) {
    let open = |out: &[u8], x: usize, y: usize| pixel(out, x, y)[3] < 255;
    let mut seen = vec![false; WIDTH * HEIGHT];
    let mut queue = VecDeque::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let border = x == 0 || y == 0 || x == WIDTH - 1 || y == HEIGHT - 1;
            if border && open(out, x, y) {
                seen[y * WIDTH + x] = true;
                queue.push_back((x, y));
            }
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let (nx, ny) = (x as isize + dx, y as isize + dy);
            if !(0..WIDTH as isize).contains(&nx) || !(0..HEIGHT as isize).contains(&ny) {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            if !seen[ny * WIDTH + nx] && open(out, nx, ny) {
                seen[ny * WIDTH + nx] = true;
                queue.push_back((nx, ny));
            }
        }
    }
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if !seen[y * WIDTH + x] && open(out, x, y) {
                out[(y * WIDTH + x) * 4..][..4].copy_from_slice(&outline);
            }
        }
    }
}

/// Give every edge the soft rim the texture's outlines have
/// .
fn soften_edges(out: &mut [u8], soft: Rgba) {
    let opaque = |out: &[u8], x: isize, y: isize| {
        (0..WIDTH as isize).contains(&x)
            && (0..HEIGHT as isize).contains(&y)
            && pixel(out, x as usize, y as usize)[3] == 255
    };
    let mut rim = Vec::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let (xi, yi) = (x as isize, y as isize);
            let near = (-1..=1).any(|dy| (-1..=1).any(|dx| opaque(out, xi + dx, yi + dy)));
            if pixel(out, x, y)[3] == 0 && near {
                rim.push((x, y));
            }
        }
    }
    for (x, y) in rim {
        out[(y * WIDTH + x) * 4..][..4].copy_from_slice(&soft);
    }
}

/// "ERKAM STYLE"; `pixels` is a 512x64 RGBA banner laid out as vanilla's, which lends its shades.
fn compose(pixels: &[u8]) -> Vec<u8> {
    let palette = Palette::sample(pixels);
    let mut out = vec![0; WIDTH * HEIGHT * 4];
    draw_glyphs(&mut out, &glyph_faces(), &palette, 0..WIDTH);
    fill_counters(&mut out, palette.outline);
    soften_edges(&mut out, palette.soft);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banner_is_drawn_inside_its_texture() {
        let faces = glyph_faces();
        let used = |x: usize| (0..HEIGHT).any(|y| faces[y * WIDTH + x]);
        let first = (0..WIDTH).find(|&x| used(x)).unwrap();
        let last = (0..WIDTH).rev().find(|&x| used(x)).unwrap();
        // Centred, with room left for the outline and the soft rim.
        assert!(first >= 4 && WIDTH - 1 - last >= 4, "{first}..{last}");
        assert!(first.abs_diff(WIDTH - 1 - last) <= 1);
    }

    #[test]
    fn only_vanilla_is_rebuilt() {
        let mut texture = vec![0; WIDTH * HEIGHT * 4];
        texture[3] = 255;
        assert!(rust_edition(512, 64, &texture).is_none());
    }
}
