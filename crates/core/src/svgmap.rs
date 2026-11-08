//! Track outline from iRacing's official track map (the SVG layers iRacing shows on its
//! website): the racing line from the "active" layer, the start/finish line from the
//! "start-finish" layer and the turn numbers from the "turns" layer to tell the direction of
//! travel. The line is resampled by distance so that point i sits at lap fraction i / BINS,
//! like a recorded outline.

use crate::engine::trackmap::{normalise, TrackMap, BINS, MAP_VERSION};

type P = [f64; 2];
type M = [f64; 6];

const ID: M = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn mul(a: &M, b: &M) -> M {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}

fn apply(m: &M, p: P) -> P {
    [m[0] * p[0] + m[2] * p[1] + m[4], m[1] * p[0] + m[3] * p[1] + m[5]]
}

fn numbers(s: &str) -> Vec<f64> {
    let mut out = vec![];
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if c.is_ascii_digit() || c == '-' || c == '+' || c == '.' {
            let start = i;
            i += 1;
            let mut dot = c == '.';
            while i < b.len() {
                let d = b[i] as char;
                if d.is_ascii_digit() {
                    i += 1;
                } else if d == '.' && !dot {
                    dot = true;
                    i += 1;
                } else if (d == 'e' || d == 'E') && i + 1 < b.len() {
                    i += 1;
                    if b[i] == b'-' || b[i] == b'+' {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            if let Ok(v) = s[start..i].parse::<f64>() {
                out.push(v);
            }
        } else {
            i += 1;
        }
    }
    out
}

/// `transform="translate(..) matrix(..) scale(..) rotate(..)"`.
fn transform(s: &str) -> M {
    let mut m = ID;
    for part in s.split(')') {
        let Some((name, args)) = part.split_once('(') else { continue };
        let v = numbers(args);
        let t = match name.trim().trim_start_matches(',').trim() {
            "matrix" if v.len() == 6 => [v[0], v[1], v[2], v[3], v[4], v[5]],
            "translate" => [1.0, 0.0, 0.0, 1.0, v.first().copied().unwrap_or(0.0), v.get(1).copied().unwrap_or(0.0)],
            "scale" => {
                let sx = v.first().copied().unwrap_or(1.0);
                [sx, 0.0, 0.0, v.get(1).copied().unwrap_or(sx), 0.0, 0.0]
            }
            "rotate" => {
                let a = v.first().copied().unwrap_or(0.0).to_radians();
                let r = [a.cos(), a.sin(), -a.sin(), a.cos(), 0.0, 0.0];
                match (v.get(1), v.get(2)) {
                    (Some(&cx), Some(&cy)) => mul(&mul(&[1.0, 0.0, 0.0, 1.0, cx, cy], &r), &[1.0, 0.0, 0.0, 1.0, -cx, -cy]),
                    _ => r,
                }
            }
            _ => ID,
        };
        m = mul(&m, &t);
    }
    m
}

/// Subpaths of an SVG path, curves flattened.
fn path(d: &str) -> Vec<Vec<P>> {
    let mut subs: Vec<Vec<P>> = vec![];
    let mut cur: Vec<P> = vec![];
    let (mut pos, mut start, mut ctrl): (P, P, Option<P>) = ([0.0; 2], [0.0; 2], None);
    // Tokens: command letters and numbers.
    let mut toks: Vec<(char, Vec<f64>)> = vec![];
    let mut cmd = ' ';
    let mut buf = String::new();
    for c in d.chars() {
        if c.is_ascii_alphabetic() && c != 'e' && c != 'E' {
            if cmd != ' ' {
                toks.push((cmd, numbers(&buf)));
            }
            cmd = c;
            buf.clear();
        } else {
            buf.push(c);
        }
    }
    if cmd != ' ' {
        toks.push((cmd, numbers(&buf)));
    }
    let flush = |cur: &mut Vec<P>, subs: &mut Vec<Vec<P>>| {
        if cur.len() > 1 {
            subs.push(std::mem::take(cur));
        } else {
            cur.clear();
        }
    };
    let cubic = |cur: &mut Vec<P>, p0: P, c1: P, c2: P, p: P| {
        for k in 1..=12 {
            let t = k as f64 / 12.0;
            let u = 1.0 - t;
            cur.push([
                u * u * u * p0[0] + 3.0 * u * u * t * c1[0] + 3.0 * u * t * t * c2[0] + t * t * t * p[0],
                u * u * u * p0[1] + 3.0 * u * u * t * c1[1] + 3.0 * u * t * t * c2[1] + t * t * t * p[1],
            ]);
        }
    };
    for (c, v) in toks {
        let rel = c.is_ascii_lowercase();
        let off = |p: P, pos: P| if rel { [p[0] + pos[0], p[1] + pos[1]] } else { p };
        match c.to_ascii_uppercase() {
            'M' => {
                flush(&mut cur, &mut subs);
                for (i, xy) in v.as_chunks::<2>().0.iter().enumerate() {
                    pos = off([xy[0], xy[1]], pos);
                    if i == 0 {
                        start = pos;
                    }
                    cur.push(pos);
                }
                ctrl = None;
            }
            'L' | 'T' => {
                for xy in v.as_chunks::<2>().0.iter() {
                    pos = off([xy[0], xy[1]], pos);
                    cur.push(pos);
                }
                ctrl = None;
            }
            'H' => {
                for x in &v {
                    pos = [if rel { pos[0] + x } else { *x }, pos[1]];
                    cur.push(pos);
                }
                ctrl = None;
            }
            'V' => {
                for y in &v {
                    pos = [pos[0], if rel { pos[1] + y } else { *y }];
                    cur.push(pos);
                }
                ctrl = None;
            }
            'C' => {
                for w in v.as_chunks::<6>().0.iter() {
                    let (c1, c2, p) = (off([w[0], w[1]], pos), off([w[2], w[3]], pos), off([w[4], w[5]], pos));
                    cubic(&mut cur, pos, c1, c2, p);
                    ctrl = Some(c2);
                    pos = p;
                }
            }
            'S' => {
                for w in v.as_chunks::<4>().0.iter() {
                    let c1 = ctrl.map(|c| [2.0 * pos[0] - c[0], 2.0 * pos[1] - c[1]]).unwrap_or(pos);
                    let (c2, p) = (off([w[0], w[1]], pos), off([w[2], w[3]], pos));
                    cubic(&mut cur, pos, c1, c2, p);
                    ctrl = Some(c2);
                    pos = p;
                }
            }
            'Q' => {
                for w in v.as_chunks::<4>().0.iter() {
                    let (q, p) = (off([w[0], w[1]], pos), off([w[2], w[3]], pos));
                    let c1 = [pos[0] + 2.0 / 3.0 * (q[0] - pos[0]), pos[1] + 2.0 / 3.0 * (q[1] - pos[1])];
                    let c2 = [p[0] + 2.0 / 3.0 * (q[0] - p[0]), p[1] + 2.0 / 3.0 * (q[1] - p[1])];
                    cubic(&mut cur, pos, c1, c2, p);
                    ctrl = None;
                    pos = p;
                }
            }
            'A' => {
                // Arcs are rare in track maps: a straight line to the end point is close enough.
                for w in v.as_chunks::<7>().0.iter() {
                    pos = off([w[5], w[6]], pos);
                    cur.push(pos);
                }
                ctrl = None;
            }
            'Z' => {
                if cur.first() != Some(&pos) {
                    cur.push(start);
                }
                pos = start;
                flush(&mut cur, &mut subs);
                ctrl = None;
            }
            _ => {}
        }
    }
    flush(&mut cur, &mut subs);
    subs
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = tag;
    while let Some(i) = rest.find(name) {
        let before = rest[..i].chars().last();
        let after = rest[i + name.len()..].trim_start();
        if before.is_some_and(|c| c.is_whitespace()) && after.starts_with('=') {
            let v = after[1..].trim_start();
            let q = v.chars().next()?;
            if q == '"' || q == '\'' {
                let end = v[1..].find(q)?;
                return Some(&v[1..1 + end]);
            }
        }
        rest = &rest[i + name.len()..];
    }
    None
}

#[derive(Default)]
struct Svg {
    /// Polylines of every shape, in document coordinates.
    shapes: Vec<Vec<P>>,
    /// Text labels and where they are.
    texts: Vec<(String, P)>,
}

fn parse(svg: &str) -> Svg {
    let mut out = Svg::default();
    parse_into(svg, svg, ID, 0, &mut out);
    out
}

/// The symbol (or other element) with this id: its tag and its content.
fn element_by_id<'a>(doc: &'a str, id: &str) -> Option<(&'a str, &'a str)> {
    let at = doc.find(&format!("id=\"{id}\"")).or_else(|| doc.find(&format!("id='{id}'")))?;
    let start = doc[..at].rfind('<')?;
    let end_tag = start + doc[start..].find('>')?;
    let tag = &doc[start + 1..end_tag];
    let name: String = tag.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
    if tag.ends_with('/') {
        return Some((tag, ""));
    }
    let close = doc[end_tag..].find(&format!("</{name}")).map(|k| end_tag + k)?;
    Some((tag, &doc[end_tag + 1..close]))
}

/// Shapes and texts of `svg` (part of `doc`, where `<use>` looks symbols up), placed by `base`.
fn parse_into(doc: &str, svg: &str, base: M, depth: u32, out: &mut Svg) {
    let mut stack: Vec<M> = vec![base];
    let mut i = 0;
    while let Some(o) = svg[i..].find('<') {
        let s = i + o;
        let Some(e) = svg[s..].find('>') else { break };
        let tag = &svg[s + 1..s + e];
        i = s + e + 1;
        if let Some(close) = tag.strip_prefix('/') {
            if close.trim_start().starts_with("g") && stack.len() > 1 {
                stack.pop();
            }
            continue;
        }
        let name: String = tag.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        let selfclose = tag.ends_with('/');
        let m = mul(stack.last().unwrap(), &attr(tag, "transform").map(transform).unwrap_or(ID));
        let f = |k: &str| attr(tag, k).and_then(|v| numbers(v).first().copied());
        let tr = |pts: Vec<P>| pts.into_iter().map(|p| apply(&m, p)).collect::<Vec<_>>();
        match name.as_str() {
            "g" if !selfclose => stack.push(m),
            // Definitions are only drawn where a <use> places them.
            "defs" | "symbol" if !selfclose => {
                if let Some(k) = svg[i..].find(&format!("</{name}")) {
                    i += k;
                }
            }
            "use" if depth < 4 => {
                let href = attr(tag, "xlink:href").or_else(|| attr(tag, "href")).unwrap_or("");
                let Some((target, content)) = href.strip_prefix('#').and_then(|id| element_by_id(doc, id)) else { continue };
                // At (x, y), with the symbol's viewBox fitted into width × height (centred).
                let mut place = mul(&m, &[1.0, 0.0, 0.0, 1.0, f("x").unwrap_or(0.0), f("y").unwrap_or(0.0)]);
                let vb = attr(target, "viewBox").map(numbers).unwrap_or_default();
                if let ([vx, vy, vw, vh, ..], Some(w), Some(h)) = (vb.as_slice(), f("width"), f("height")) {
                    if *vw > 0.0 && *vh > 0.0 {
                        let k = (w / vw).min(h / vh);
                        place = mul(&place, &[k, 0.0, 0.0, k, (w - vw * k) / 2.0 - vx * k, (h - vh * k) / 2.0 - vy * k]);
                    }
                }
                if content.is_empty() {
                    // A single element: parse its own tag.
                    let one = format!("<{target}>");
                    parse_into(doc, &one, place, depth + 1, out);
                } else {
                    parse_into(doc, content, place, depth + 1, out);
                }
            }
            "path" => {
                for sub in path(attr(tag, "d").unwrap_or("")) {
                    out.shapes.push(tr(sub));
                }
            }
            "polygon" | "polyline" => {
                let mut pts: Vec<P> = numbers(attr(tag, "points").unwrap_or("")).as_chunks::<2>().0.to_vec();
                if name == "polygon" && pts.len() > 2 {
                    pts.push(pts[0]);
                }
                out.shapes.push(tr(pts));
            }
            "line" => out.shapes.push(tr(vec![[f("x1").unwrap_or(0.0), f("y1").unwrap_or(0.0)], [f("x2").unwrap_or(0.0), f("y2").unwrap_or(0.0)]])),
            "rect" => {
                let (x, y, w, h) = (f("x").unwrap_or(0.0), f("y").unwrap_or(0.0), f("width").unwrap_or(0.0), f("height").unwrap_or(0.0));
                out.shapes.push(tr(vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h], [x, y]]));
            }
            "circle" | "ellipse" => out.shapes.push(tr(vec![[f("cx").unwrap_or(0.0), f("cy").unwrap_or(0.0)]])),
            "text" if !selfclose => {
                // Text content up to </text>, without nested tags (tspans).
                let end = svg[i..].find("</text").map(|k| i + k).unwrap_or(i);
                let mut content = String::new();
                let mut inside = false;
                for c in svg[i..end].chars() {
                    match c {
                        '<' => inside = true,
                        '>' => inside = false,
                        c if !inside => content.push(c),
                        _ => {}
                    }
                }
                // Position: x/y on the text or its first tspan, plus the transform.
                let pos = [f("x").unwrap_or(0.0), f("y").unwrap_or(0.0)];
                let pos = if pos == [0.0, 0.0] {
                    let inner = &svg[i..end];
                    inner
                        .find("<tspan")
                        .map(|k| {
                            let t = &inner[k..inner[k..].find('>').map(|e| k + e).unwrap_or(inner.len())];
                            let g = |n: &str| attr(t, n).and_then(|v| numbers(v).first().copied()).unwrap_or(0.0);
                            [g("x"), g("y")]
                        })
                        .unwrap_or(pos)
                } else {
                    pos
                };
                out.texts.push((content.trim().to_string(), apply(&m, pos)));
                i = end;
            }
            _ => {}
        }
    }
}

fn length(pts: &[P]) -> f64 {
    pts.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum()
}

/// Arc position (0..len) of the point on the closed line nearest to `p`.
fn project(line: &[P], cum: &[f64], p: P) -> f64 {
    nearest(line, cum, p).1
}

/// How far `p` is from the line, and the distance along the line of the nearest point.
fn nearest(line: &[P], cum: &[f64], p: P) -> (f64, f64) {
    let mut best = (f64::MAX, 0.0);
    for (k, w) in line.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let l2 = dx * dx + dy * dy;
        let t = if l2 > 0.0 { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
        let q = [a[0] + dx * t, a[1] + dy * t];
        let d = (p[0] - q[0]).hypot(p[1] - q[1]);
        if d < best.0 {
            best = (d, cum[k] + t * l2.sqrt());
        }
    }
    best
}

/// The start/finish layer: where the line crosses the track (distance along the racing line)
/// and, when the layer has iRacing's direction arrow, whether travel follows the line's
/// own order. The marker is the shape lying on the track; the arrow sits beside it.
fn start_line(line: &[P], cum: &[f64], shapes: &[Vec<P>]) -> Option<(f64, Option<bool>)> {
    let total = *cum.last()?;
    let mut found: Vec<(f64, f64, &Vec<P>)> = shapes
        .iter()
        .filter(|s| !s.is_empty())
        .map(|s| {
            let (d, at_s) = nearest(line, cum, mean(s));
            (d, at_s, s)
        })
        .collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (_, s0, _) = *found.first()?;
    // The arrow: its tip is the corner furthest from its middle.
    let forward = found.iter().skip(1).find(|f| f.2.len() >= 3).and_then(|&(_, s, arrow)| {
        let c = mean(arrow);
        let tip = arrow.iter().copied().max_by(|a, b| (a[0] - c[0]).hypot(a[1] - c[1]).total_cmp(&(b[0] - c[0]).hypot(b[1] - c[1])))?;
        let v = [tip[0] - c[0], tip[1] - c[1]];
        let step = total * 0.005;
        let (a, b) = (at(line, cum, s - step), at(line, cum, s + step));
        let t = [b[0] - a[0], b[1] - a[1]];
        let cos = (v[0] * t[0] + v[1] * t[1]) / (v[0].hypot(v[1]) * t[0].hypot(t[1])).max(1e-9);
        // Only an arrow pointing along the track says anything.
        (cos.abs() > 0.5).then_some(cos > 0.0)
    });
    Some((s0, forward))
}

/// The middle of a shape's points.
fn mean(pts: &[P]) -> P {
    let n = pts.len().max(1) as f64;
    [pts.iter().map(|p| p[0]).sum::<f64>() / n, pts.iter().map(|p| p[1]).sum::<f64>() / n]
}

fn cumulative(line: &[P]) -> Vec<f64> {
    let mut cum = vec![0.0];
    for w in line.windows(2) {
        cum.push(cum.last().unwrap() + (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]));
    }
    cum
}

/// The point `s` along a line that is not a loop (clamped to its ends).
fn at_open(line: &[P], cum: &[f64], s: f64) -> P {
    let total = *cum.last().unwrap();
    if s >= total {
        return *line.last().unwrap();
    }
    at(line, cum, s.max(0.0))
}

/// The nearest point of `line` to `p` that is more than `gap` away from `s` along it (the
/// loop's other side): its distance from `p` and its position along the line.
fn nearest_elsewhere(line: &[P], cum: &[f64], p: P, s: f64, gap: f64) -> (f64, f64) {
    let total = *cum.last().unwrap();
    let mut best = (f64::MAX, 0.0);
    for (k, w) in line.windows(2).enumerate() {
        let mid = (cum[k] + cum[k + 1]) / 2.0;
        let apart = (mid - s).rem_euclid(total);
        if apart.min(total - apart) < gap {
            continue;
        }
        let (a, b) = (w[0], w[1]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let l2 = dx * dx + dy * dy;
        let t = if l2 > 0.0 { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
        let q = [a[0] + dx * t, a[1] + dy * t];
        let d = (p[0] - q[0]).hypot(p[1] - q[1]);
        if d < best.0 {
            best = (d, cum[k] + t * l2.sqrt());
        }
    }
    best
}

/// What the active layer's line turned out to be.
enum Shape {
    /// The track itself.
    Line,
    /// A ribbon (the track drawn as a thin filled shape: along one edge and back along the
    /// other), reduced to its centre line; and whether that is a loop.
    Ribbon(Vec<P>, bool),
    /// Partly a ribbon and partly not: no reliable line (the recorded outline is used).
    Mixed,
}

/// Whether the closed `line` outlines a ribbon, and its centre line if so.
fn ribbon_centre(line: &[P]) -> Shape {
    ribbon(line).unwrap_or(Shape::Line)
}

fn ribbon(line: &[P]) -> Option<Shape> {
    let cum = cumulative(line);
    let total = *cum.last()?;
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for p in line {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let size = (hi[0] - lo[0]).max(hi[1] - lo[1]);
    if total <= 0.0 || size <= 0.0 {
        return None;
    }
    // On a ribbon every point has the other edge right beside it, and the two edges mirror
    // each other about the ends: s and its partner add up to the same (twice an end).
    const N: usize = 240;
    let (mut near, mut cx, mut cy) = (0, 0.0, 0.0);
    for i in 0..N {
        let s = total * i as f64 / N as f64;
        let (d, other) = nearest_elsewhere(line, &cum, at(line, &cum, s), s, total * 0.04);
        if d < size * 0.03 {
            near += 1;
        }
        let a = std::f64::consts::TAU * (s + other) / total;
        cx += a.cos();
        cy += a.sin();
    }
    let mirrored = cx.hypot(cy) / N as f64;
    if near >= N * 9 / 10 && mirrored >= 0.8 {
        // A ribbon: carry on below.
    } else if near >= N / 2 && mirrored >= 0.4 {
        // Edges mirrored over much of the lap, but not all of it.
        return Some(Shape::Mixed);
    } else {
        // Narrow ovals and hairpins have the line close by too, but nothing mirrored.
        return Some(Shape::Line);
    }
    let end = (cy.atan2(cx) / std::f64::consts::TAU).rem_euclid(1.0) * total / 2.0;
    // One edge from that end to the other, the other edge back; the centre is midway.
    const M: usize = 1500;
    let half = total / 2.0;
    let back: Vec<P> = (0..=M).map(|k| at(line, &cum, end + half + half * k as f64 / M as f64)).collect();
    let back_cum = cumulative(&back);
    let centre: Vec<P> = (0..=M)
        .map(|k| {
            let a = at(line, &cum, end + half * k as f64 / M as f64);
            let (_, s) = nearest(&back, &back_cum, a);
            let b = at_open(&back, &back_cum, s);
            [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
        })
        .collect();
    let length = *cumulative(&centre).last()?;
    let (first, last) = (centre[0], centre[M]);
    // A circuit drawn as one stroke has its two ends at (or near) the same place.
    let is_loop = (first[0] - last[0]).hypot(first[1] - last[1]) < length * 0.1;
    Some(Shape::Ribbon(centre, is_loop))
}

fn at(line: &[P], cum: &[f64], s: f64) -> P {
    let total = *cum.last().unwrap();
    let s = s.rem_euclid(total);
    let k = cum.partition_point(|&c| c <= s).clamp(1, line.len() - 1);
    let (a, b) = (line[k - 1], line[k]);
    let seg = cum[k] - cum[k - 1];
    let t = if seg > 0.0 { (s - cum[k - 1]) / seg } else { 0.0 };
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

/// Turn numbers and where they are drawn. Only labels that are just a number count ("7",
/// "T7", "Turn 7"), not corner names with a number in them ("1º Curva di Lesmo"); a number
/// drawn as separate digits side by side ("1" "1" for 11) is read as one.
fn turn_numbers(texts: Vec<(String, P)>) -> Vec<(u32, P)> {
    let mut nums: Vec<(String, P)> = texts
        .into_iter()
        .filter_map(|(t, p)| {
            let l = t.trim().to_ascii_lowercase();
            let d = l.strip_prefix("turn").or_else(|| l.strip_prefix('t')).unwrap_or(&l).trim();
            (!d.is_empty() && d.len() <= 3 && d.chars().all(|c| c.is_ascii_digit())).then(|| (d.to_string(), p))
        })
        .collect();
    nums.sort_by(|a, b| a.1[0].total_cmp(&b.1[0]));
    let mut out: Vec<(u32, P)> = vec![];
    let mut used = vec![false; nums.len()];
    for i in 0..nums.len() {
        if used[i] {
            continue;
        }
        used[i] = true;
        let (mut text, p) = nums[i].clone();
        let mut last = p;
        // Single digits on the same baseline just to the right continue the number.
        while text.len() < 3 {
            let next = (0..nums.len())
                .find(|&j| !used[j] && nums[j].0.len() == 1 && (nums[j].1[1] - last[1]).abs() < 6.0 && nums[j].1[0] > last[0] && nums[j].1[0] - last[0] < 30.0);
            let Some(j) = next else { break };
            used[j] = true;
            text.push_str(&nums[j].0);
            last = nums[j].1;
        }
        if let Ok(n) = text.parse() {
            out.push((n, p));
        }
    }
    out
}

/// Build the outline from the SVG layers. `t1_hint`: lap fraction of turn 1 when known (e.g.
/// from Lovely Track Data), the most reliable way to tell the direction of travel.
pub fn build(track_id: i64, name: &str, active: &str, start_finish: Option<&str>, turns: Option<&str>, t1_hint: Option<f32>) -> Option<TrackMap> {
    let mut line = parse(active).shapes.into_iter().max_by(|a, b| length(a).total_cmp(&length(b)))?;
    if line.len() < 4 {
        return None;
    }
    if line.first() != line.last() {
        line.push(line[0]);
    }
    // Some maps draw the track as a filled ribbon (one edge out, the other back): use its
    // centre line, which for a hillclimb is not a loop.
    let mut open = false;
    match ribbon_centre(&line) {
        Shape::Line => {}
        Shape::Ribbon(centre, is_loop) => {
            line = centre;
            open = !is_loop;
            if !open && line.first() != line.last() {
                line.push(line[0]);
            }
        }
        Shape::Mixed => return None,
    }
    let cum = cumulative(&line);
    let total = *cum.last().unwrap();
    if total <= 0.0 {
        return None;
    }
    let sf_shapes = start_finish.map(|s| parse(s).shapes).unwrap_or_default();
    let sf = start_line(&line, &cum, &sf_shapes);
    if open {
        // Point to point: the lap runs from the start to the other end. The layer marks the
        // start and the finish, with an arrow by the start (the shape furthest off the track).
        let off = |sh: &Vec<P>| nearest(&line, &cum, mean(sh)).0;
        let start = if sf_shapes.iter().filter(|sh| !sh.is_empty()).count() >= 3 {
            sf_shapes.iter().filter(|sh| !sh.is_empty()).max_by(|a, b| off(a).total_cmp(&off(b))).map(|a| nearest(&line, &cum, mean(a)).1)
        } else {
            sf.map(|(s, _)| s)
        };
        let from_end = start.is_some_and(|s| s > total / 2.0);
        let pts: Vec<[f64; 2]> = (0..BINS)
            .map(|i| {
                let d = total * i as f64 / (BINS - 1) as f64;
                let p = at_open(&line, &cum, if from_end { total - d } else { d });
                [p[0], -p[1]]
            })
            .collect();
        return Some(TrackMap { track_id, name: name.to_string(), points: normalise(&pts), source: "iracing".into(), version: MAP_VERSION });
    }
    let (s0, arrow) = sf.unwrap_or((0.0, None));
    // Turn labels: the lowest number is turn 1.
    let mut labels = turns.map(|t| turn_numbers(parse(t).texts)).unwrap_or_default();
    labels.sort_by_key(|l| l.0);
    let ahead = |p: P| (project(&line, &cum, p) - s0).rem_euclid(total) / total;
    // The turn numbers: each turn and the next vote on which way the numbers run round the
    // line (a stray or doubled label is outvoted).
    let (mut votes_for, mut votes_against) = (0u32, 0u32);
    for w in labels.windows(2) {
        if w[1].0 == w[0].0 + 1 {
            let step = (ahead(w[1].1) - ahead(w[0].1)).rem_euclid(1.0);
            if step < 0.5 {
                votes_for += 1;
            } else {
                votes_against += 1;
            }
        }
    }
    let one = |n: u32| {
        let mut it = labels.iter().filter(|l| l.0 == n);
        match (it.next(), it.next()) {
            (Some(l), None) => Some(l.1),
            _ => None,
        }
    };
    // The turn numbers, else turn 1's place from Lovely Track Data (its first corner is not
    // always iRacing's turn 1, as at Spa's classic pits), else iRacing's direction arrow.
    let clear = votes_for.abs_diff(votes_against) >= 3 && votes_for.min(votes_against) * 3 <= votes_for.max(votes_against);
    let forward = match (t1_hint, labels.first()) {
        // A clear majority of the turns.
        _ if clear => votes_for > votes_against,
        // Exactly one turn 1 and one turn 2: turn 2 comes shortly after turn 1.
        _ if one(1).is_some() && one(2).is_some() => (ahead(one(2).unwrap()) - ahead(one(1).unwrap())).rem_euclid(1.0) < 0.5,
        (Some(h), Some(t1)) => {
            let f = ahead(t1.1);
            ((f - h as f64).abs()) <= ((1.0 - f) - h as f64).abs()
        }
        // Doubled or stray numbers: the vote.
        _ if votes_for != votes_against => votes_for > votes_against,
        (_, Some(t1)) => ahead(t1.1) < 0.5,
        _ => arrow.unwrap_or(true),
    };
    let dir = if forward { 1.0 } else { -1.0 };
    // SVG y points down; `normalise` expects y up.
    let pts: Vec<[f64; 2]> = (0..BINS)
        .map(|i| {
            let p = at(&line, &cum, s0 + dir * total * i as f64 / BINS as f64);
            [p[0], -p[1]]
        })
        .collect();
    Some(TrackMap { track_id, name: name.to_string(), points: normalise(&pts), source: "iracing".into(), version: MAP_VERSION })
}

/// Where the published index of iRacing's track maps lives (track id → map folder and
/// layer files), so maps work without an iRacing login. Maintainers refresh it from the
/// `iracing-track-maps.json` a logged-in copy of the app writes (see `trim_index`).
const INDEX_URL: &str = "https://raw.githubusercontent.com/ShamzOut2/Apex_Overlays/main/data/iracing-track-maps.json";

/// The shared library of recorded track outlines (see data/trackmaps in the repository).
const LIBRARY_URL: &str = "https://raw.githubusercontent.com/ShamzOut2/Apex_Overlays/main/data/trackmaps";

/// The public catalogue of iRacing's track maps kept by the racing-track-maps-vector project
/// (every track and layout, read from iRacing's site, with a copy of each SVG layer): where
/// tracks newer than the index built into the app are looked up.
const CATALOGUE_URL: &str = "https://raw.githubusercontent.com/meowmachine/racing-track-maps-vector/main/from-iracing/iracing-tracks-metadata.json";

/// That project's copy of the SVG layers, used when iRacing's own server does not answer.
const MIRROR_URL: &str = "https://raw.githubusercontent.com/meowmachine/racing-track-maps-vector/main/from-iracing";

/// The index of every track map, as published when this version was built (see
/// `data/README.md`), so the official maps work offline from GitHub and without a login.
const BUNDLED_INDEX: &str = include_str!("../../../data/iracing-track-maps.json");

/// The map layers the outline is built from.
const LAYERS: [&str; 3] = ["active", "start-finish", "turns"];

/// A folder path made safe for a URL (its folders have spaces, e.g. "Dirt Oval").
fn url_path(path: &str) -> String {
    let mut out = String::new();
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The racing-track-maps-vector catalogue turned into the index format (track id → map
/// folder, layer files and the mirror folder of the same files).
pub fn index_from_catalogue(catalogue: &serde_json::Value) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    let tracks = catalogue.get("tracks").and_then(|t| t.as_array()).map(|v| v.as_slice()).unwrap_or_default();
    for config in tracks.iter().flat_map(|t| t.get("configurations").and_then(|c| c.as_array()).map(|v| v.as_slice()).unwrap_or_default()) {
        let id = config.get("track_id").and_then(|v| v.as_i64().or_else(|| v.as_str()?.parse().ok()));
        let folder = config.get("svg_folder_url").and_then(|v| v.as_str()).unwrap_or("").trim_end_matches('/');
        let (Some(id), Some(urls)) = (id, config.get("svg_urls").and_then(|u| u.as_object())) else { continue };
        if folder.is_empty() {
            continue;
        }
        let mut layers = serde_json::Map::new();
        for k in LAYERS {
            let Some(url) = urls.get(k).and_then(|u| u.as_str()) else { continue };
            // A file in the map folder by name, anything else by its full address.
            let file = url.strip_prefix(folder).map(|f| f.trim_start_matches('/')).filter(|f| !f.contains('/')).unwrap_or(url);
            layers.insert(k.to_string(), file.into());
        }
        if !layers.contains_key("active") {
            continue;
        }
        let mut entry = serde_json::json!({ "track_map": format!("{folder}/"), "track_map_layers": layers });
        if let Some(local) = config.get("svg_local_path").and_then(|v| v.as_str()).filter(|p| !p.is_empty() && !p.contains("..")) {
            entry["mirror"] = format!("{MIRROR_URL}/{}/", url_path(local.trim_matches('/'))).into();
        }
        out.insert(id.to_string(), entry);
    }
    serde_json::Value::Object(out)
}

/// The part of the Data API's track/assets that locates each track map: the SVG files
/// themselves are public, only this index needs a login.
pub fn trim_index(assets: &serde_json::Value) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    for (id, e) in assets.as_object().into_iter().flatten() {
        let (Some(base), Some(layers)) = (e.get("track_map").and_then(|b| b.as_str()), e.get("track_map_layers")) else { continue };
        if base.is_empty() || !layers.is_object() {
            continue;
        }
        out.insert(id.clone(), serde_json::json!({ "track_map": base, "track_map_layers": layers }));
    }
    serde_json::Value::Object(out)
}

/// Downloads iRacing's track map for the current track and caches the outline per track.
/// The index of maps comes from the Data API when the iRacing login is set up (League
/// builder → Import from iRacing), otherwise from the copy published with Apex Overlays.
pub fn spawn(
    hub: std::sync::Arc<crate::hub::Hub>,
    config: crate::config::ConfigStore,
    data_dir: std::path::PathBuf,
    track: std::sync::Arc<std::sync::Mutex<(i64, String)>>,
) {
    use crate::hub::Command;
    use serde_json::Value;
    use std::time::{Duration, Instant, SystemTime};
    let _ = std::thread::Builder::new().name("trackmaps".into()).spawn(move || {
        // Built outlines, per track (v2: rebuilt with the ribbon, symbol and turn-number fixes).
        let dir = data_dir.join("trackmaps").join("iracing-v2");
        let assets_file = data_dir.join("iracing").join("track-assets.json");
        let index_file = data_dir.join("iracing").join("published-track-maps.json");
        let catalogue_file = data_dir.join("iracing").join("track-map-catalogue.json");
        let bundled: Value = serde_json::from_str(BUNDLED_INDEX).unwrap_or_default();
        let agent = crate::net::agent();
        let mut client: Option<crate::iracing_api::Client> = None;
        let mut done: Option<i64> = None;
        let mut library_done: Option<i64> = None;
        let library_dir = data_dir.join("trackmaps").join("library");
        let mut retry_at = Instant::now();
        let read = |p: &std::path::Path| std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok());
        let published = assets_file.with_file_name("iracing-track-maps.json");
        loop {
            std::thread::sleep(Duration::from_secs(3));
            // With the iRacing login: write the trimmed index for publishing as soon as the app
            // runs (no session needed), from the cached asset list or a fresh one.
            let api = config.read(|c| c.iracing_api.clone());
            if api.configured() && !published.exists() && Instant::now() >= retry_at {
                let assets = match read(&assets_file) {
                    Some(v) => Some(v),
                    None => {
                        let c = client.get_or_insert_with(|| crate::iracing_api::Client::new(api.clone()));
                        match c.get("track/assets", &[]) {
                            Ok(v) => {
                                if let Some(d) = assets_file.parent() {
                                    let _ = std::fs::create_dir_all(d);
                                }
                                let _ = std::fs::write(&assets_file, v.to_string());
                                Some(v)
                            }
                            Err(e) => {
                                log::warn!("track maps: {e}");
                                client = None;
                                retry_at = Instant::now() + Duration::from_secs(600);
                                None
                            }
                        }
                    }
                };
                if let Some(a) = assets {
                    let _ = std::fs::write(&published, serde_json::to_string_pretty(&trim_index(&a)).unwrap_or_default());
                    log::info!("wrote {}", published.display());
                }
            }
            let (id, name) = track.lock().unwrap().clone();
            if id == 0 {
                continue;
            }
            // Shared map library: an outline for tracks the player has not recorded yet.
            if library_done != Some(id) {
                library_done = Some(id);
                if !data_dir.join("trackmaps").join(format!("{id}.json")).exists() {
                    let cached = library_dir.join(format!("{id}.json"));
                    let map = read(&cached).and_then(|v| serde_json::from_value::<TrackMap>(v).ok()).or_else(|| {
                        // The library's index (cached a day) says which tracks it has.
                        let index_path = library_dir.join("index.json");
                        let fresh = std::fs::metadata(&index_path)
                            .and_then(|m| m.modified())
                            .ok()
                            .and_then(|t| SystemTime::now().duration_since(t).ok())
                            .is_some_and(|age| age < Duration::from_secs(86_400));
                        let index = read(&index_path).filter(|_| fresh).or_else(|| {
                            let v = agent.get(&format!("{LIBRARY_URL}/index.json")).call().ok()?.into_json::<Value>().ok()?;
                            let _ = std::fs::create_dir_all(&library_dir);
                            let _ = std::fs::write(&index_path, v.to_string());
                            Some(v)
                        })?;
                        index.pointer(&format!("/tracks/{id}"))?;
                        let m = agent.get(&format!("{LIBRARY_URL}/{id}.json")).call().ok()?.into_json::<TrackMap>().ok()?;
                        (m.track_id == id && m.points.len() >= 50).then(|| {
                            let _ = std::fs::write(&cached, serde_json::to_string(&m).unwrap_or_default());
                            m
                        })
                    });
                    if let Some(m) = map {
                        log::info!("track map for {id} from the shared library");
                        hub.command(Command::SetLibraryMap(TrackMap { source: "library".into(), ..m }));
                    }
                }
            }
            if done == Some(id) {
                continue;
            }
            let cache = dir.join(format!("{id}.json"));
            if let Some(m) = read(&cache).and_then(|v| serde_json::from_value::<TrackMap>(v).ok()) {
                hub.command(Command::SetOfficialMap(m));
                done = Some(id);
                continue;
            }
            if Instant::now() < retry_at {
                continue;
            }
            let api = config.read(|c| c.iracing_api.clone());
            let key = id.to_string();
            // Track map index (all tracks): cached for a week, refreshed when the track is missing.
            let fresh = |p: &std::path::Path| {
                std::fs::metadata(p)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| SystemTime::now().duration_since(t).ok())
                    .is_some_and(|age| age < Duration::from_secs(7 * 86_400))
            };
            // Where this track's map is: the iRacing login's own list when set up, else the
            // index built into the app, else the published index and the public catalogue
            // (for tracks newer than this version).
            let mut entry: Option<Value> = None;
            if api.configured() {
                let mut assets = read(&assets_file).filter(|a| fresh(&assets_file) && a.get(&key).is_some());
                if assets.is_none() {
                    let c = client.get_or_insert_with(|| crate::iracing_api::Client::new(api.clone()));
                    match c.get("track/assets", &[]) {
                        Ok(v) => {
                            if let Some(d) = assets_file.parent() {
                                let _ = std::fs::create_dir_all(d);
                            }
                            let _ = std::fs::write(&assets_file, v.to_string());
                            // The trimmed index, ready to publish for users without a login.
                            let _ = std::fs::write(
                                assets_file.with_file_name("iracing-track-maps.json"),
                                serde_json::to_string_pretty(&trim_index(&v)).unwrap_or_default(),
                            );
                            assets = Some(v);
                        }
                        Err(e) => {
                            // The other sources below still work.
                            log::warn!("track maps: {e}");
                            client = None;
                        }
                    }
                }
                entry = assets.and_then(|a| a.get(&key).cloned());
            }
            if entry.is_none() {
                entry = bundled.get(&key).cloned();
            }
            let mut unreachable = false;
            if entry.is_none() {
                // Newer than this version: the published index, then the public catalogue
                // (each kept a week).
                let fetched = |file: &std::path::Path, url: &str, convert: fn(&Value) -> Value| -> Option<Value> {
                    if let Some(v) = read(file).filter(|_| fresh(file)) {
                        return Some(v);
                    }
                    let text = agent.get(url).call().ok()?.into_string().ok()?;
                    let v = convert(&serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')).ok()?);
                    if let Some(d) = file.parent() {
                        let _ = std::fs::create_dir_all(d);
                    }
                    let _ = std::fs::write(file, v.to_string());
                    Some(v)
                };
                let published = fetched(&index_file, INDEX_URL, |v| v.clone());
                entry = published.as_ref().and_then(|v| v.get(&key).cloned());
                if entry.is_none() {
                    let catalogue = fetched(&catalogue_file, CATALOGUE_URL, index_from_catalogue);
                    unreachable = published.is_none() && catalogue.is_none();
                    entry = catalogue.and_then(|v| v.get(&key).cloned());
                }
            }
            let Some(entry) = entry else {
                if unreachable {
                    // No network: try again later.
                    retry_at = Instant::now() + Duration::from_secs(3600);
                } else {
                    // Not in any index (yet): the recorded outline is used.
                    log::info!("track {id} is not in any track map index");
                    done = Some(id);
                }
                continue;
            };
            let base = entry.get("track_map").and_then(|b| b.as_str()).unwrap_or("").to_string();
            let mirror = entry.get("mirror").and_then(|b| b.as_str()).map(str::to_string);
            let layers = entry.get("track_map_layers").cloned();
            let get = |url: &str| agent.get(url).call().ok()?.into_string().ok();
            let layer = |k: &str| -> Option<String> {
                let file = layers.as_ref()?.get(k)?.as_str()?;
                let url = if file.starts_with("http") { file.to_string() } else { format!("{}/{}", base.trim_end_matches('/'), file.trim_start_matches('/')) };
                // iRacing's own server first, then the copy of the same file.
                get(&url).or_else(|| {
                    let name = url.rsplit('/').next()?;
                    get(&format!("{}/{name}", mirror.as_deref()?.trim_end_matches('/')))
                })
            };
            let Some(active) = layer("active") else {
                // Neither iRacing's server nor the copy answered: try again in a while.
                log::info!("could not download the track map for track {id}");
                retry_at = Instant::now() + Duration::from_secs(600);
                continue;
            };
            // Turn 1 from Lovely Track Data (cached by the corners downloader), if known.
            let hint = read(&data_dir.join("corners").join(format!("{id}.v2.json")))
                .and_then(|v| serde_json::from_value::<crate::corners::TrackData>(v).ok())
                .and_then(|d| d.corners.iter().find(|c| c.number == 1).map(|c| c.start));
            match build(id, &name, &active, layer("start-finish").as_deref(), layer("turns").as_deref(), hint) {
                Some(m) => {
                    let _ = std::fs::create_dir_all(&dir);
                    let _ = std::fs::write(&cache, serde_json::to_string(&m).unwrap_or_default());
                    hub.command(Command::SetOfficialMap(m));
                }
                None => log::warn!("could not read the iRacing track map for track {id}"),
            }
            done = Some(id);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every layout in the index builds a usable outline. Opt-in (needs the SVG files):
    /// APEX_TRACK_SVGS=<dir with <trackId>/<layer>.svg> cargo test -p apex-core every_indexed_track -- --ignored
    /// Writes the outlines to <dir>/out/<trackId>.json for a look.
    #[test]
    #[ignore]
    fn every_indexed_track_builds() {
        let Ok(dir) = std::env::var("APEX_TRACK_SVGS") else { return };
        let dir = std::path::Path::new(&dir);
        let index: serde_json::Value = serde_json::from_str(BUNDLED_INDEX).unwrap();
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let mut bad = vec![];
        for id in index.as_object().unwrap().keys() {
            let layer = |k: &str| std::fs::read_to_string(dir.join(id).join(format!("{k}.svg"))).ok();
            let Some(active) = layer("active") else {
                bad.push(format!("{id}: no active layer"));
                continue;
            };
            match build(id.parse().unwrap(), id, &active, layer("start-finish").as_deref(), layer("turns").as_deref(), None) {
                Some(m) if m.points.len() == BINS => std::fs::write(out.join(format!("{id}.json")), serde_json::to_string(&m).unwrap()).unwrap(),
                Some(m) => bad.push(format!("{id}: {} points", m.points.len())),
                None => bad.push(format!("{id}: no outline")),
            }
        }
        // A few maps are deliberately refused (partly drawn as a ribbon: Monza Combined): those
        // tracks use the recorded outline.
        let total = index.as_object().unwrap().len();
        println!("{} of {total} without an official outline: {bad:?}", bad.len());
        assert!(bad.len() * 100 <= total, "{} of {total} failed: {bad:#?}", bad.len());
    }

    // A 200 x 100 rectangle; start/finish halfway along the top; turn 1 at the top-right corner,
    // so cars run clockwise on screen (left to right along the top).
    const ACTIVE: &str = r#"<svg viewBox="0 0 300 200"><g transform="translate(50 50)"><path d="M0,0 H200 V100 h-200 Z" fill="none"/></g></svg>"#;
    const SF: &str = r#"<svg><g transform="translate(50 50)"><line x1="100" y1="-5" x2="100" y2="5"/></g></svg>"#;
    const TURNS: &str =
        r#"<svg><text transform="matrix(1 0 0 1 255 45)">1</text><text x="255" y="155">2</text><text><tspan x="45" y="155">3</tspan></text></svg>"#;

    fn screen(m: &TrackMap, i: usize) -> [f32; 2] {
        m.points[i]
    }

    #[test]
    fn trims_the_asset_index() {
        let v = serde_json::json!({
            "1": {"track_id": 1, "track_map": "https://x/tracks/a/1/", "track_map_layers": {"active": "active.svg", "turns": "turns.svg"}, "detail_copy": "long text"},
            "2": {"track_id": 2, "track_map": "", "track_map_layers": {}},
            "3": {"track_id": 3}
        });
        let t = trim_index(&v);
        assert_eq!(t, serde_json::json!({"1": {"track_map": "https://x/tracks/a/1/", "track_map_layers": {"active": "active.svg", "turns": "turns.svg"}}}));
    }

    #[test]
    fn parses_shapes_and_text() {
        let s = parse(TURNS);
        assert_eq!(s.texts.len(), 3);
        assert_eq!(s.texts[0], ("1".to_string(), [255.0, 45.0]));
        assert_eq!(s.texts[2].1, [45.0, 155.0]);
        let a = parse(ACTIVE);
        assert_eq!(a.shapes.len(), 1);
        assert_eq!(a.shapes[0][1], [250.0, 50.0]);
        assert_eq!(path("M0 0c10 0 10 10 10 10s0 10-10 10z")[0].last(), Some(&[0.0, 0.0]));
    }

    #[test]
    fn builds_outline_from_layers() {
        let m = build(1, "Test", ACTIVE, Some(SF), Some(TURNS), None).unwrap();
        assert_eq!(m.points.len(), BINS);
        // Starts at the start/finish line (top middle) and heads right.
        let (p0, p1) = (screen(&m, 0), screen(&m, 20));
        assert!((p0[0] - 0.5).abs() < 0.01, "{p0:?}");
        assert!(p1[0] > p0[0] + 0.05 && (p1[1] - p0[1]).abs() < 0.01, "{p0:?} {p1:?}");
        // A quarter of the lap (150 of 600) in: past turn 1, on the right-hand side.
        let q = screen(&m, BINS / 4);
        assert!(q[0] > 0.95, "{q:?}");
        // Turns numbered the other way round (turn 1 bottom left): the map runs anticlockwise.
        let rev = r#"<svg><text x="45" y="155">1</text><text x="255" y="155">2</text><text x="255" y="45">3</text></svg>"#;
        let r = build(1, "Test", ACTIVE, Some(SF), Some(rev), None).unwrap();
        assert!(screen(&r, 20)[0] < screen(&r, 0)[0]);
        // With only turn 1 labelled, a Lovely hint decides: turn 1 two thirds round the lap
        // only fits clockwise.
        let only1 = r#"<svg><text x="45" y="155">1</text></svg>"#;
        let h = build(1, "Test", ACTIVE, Some(SF), Some(only1), Some(0.66)).unwrap();
        assert!(screen(&h, 20)[0] > screen(&h, 0)[0]);
        // The turn numbers outrank the hint (its first corner is not always iRacing's turn 1).
        let n = build(1, "Test", ACTIVE, Some(SF), Some(rev), Some(0.66)).unwrap();
        assert!(screen(&n, 20)[0] < screen(&n, 0)[0]);
    }

    #[test]
    fn start_line_drawn_as_a_symbol() {
        // Newer maps define the line and the direction arrow once and place them with <use>:
        // the line at the top middle, the arrow beside the track.
        let sf = r##"<svg viewBox="0 0 300 200"><defs>
            <symbol id="Start_Finish_Line" viewBox="0 0 10 70"><rect width="10" height="70"/></symbol>
            <symbol id="Track_Direction" viewBox="0 0 100 40"><polygon points="100 20 0 0 23 20 0 40 100 20"/></symbol></defs>
            <use width="100" height="40" transform="translate(160 30) scale(.4)" xlink:href="#Track_Direction"/>
            <use width="10" height="70" transform="translate(147.5 32.5) scale(.5)" xlink:href="#Start_Finish_Line"/></svg>"##;
        let m = build(1, "Test", ACTIVE, Some(sf), Some(TURNS), None).unwrap();
        assert!((screen(&m, 0)[0] - 0.5).abs() < 0.01, "{:?}", screen(&m, 0));
    }

    #[test]
    fn turn_numbers_only_count_numbers() {
        let t = |s: &str, x: f64| (s.to_string(), [x, 10.0]);
        let mut n = turn_numbers(vec![t("1º Curva", 0.0), t("2", 40.0), t("T3", 80.0), t("Turn 4", 120.0), t("1", 200.0), t("1", 220.0), t("Lesmo", 300.0)]);
        n.sort_by_key(|l| l.0);
        // "1º Curva" is a name; "1" "1" side by side is 11.
        assert_eq!(n.iter().map(|l| l.0).collect::<Vec<_>>(), vec![2, 3, 4, 11]);
        assert_eq!(n[3].1, [200.0, 10.0]);
    }

    #[test]
    fn ribbon_maps_become_one_lap() {
        // The test rectangle drawn as a 6-wide ribbon: out along one edge from the top middle
        // and back along the other (as some iRacing maps draw the track).
        let centre: Vec<P> = vec![[150.0, 50.0], [250.0, 50.0], [250.0, 150.0], [50.0, 150.0], [50.0, 50.0], [149.0, 50.0]];
        let side = |k: f64| -> Vec<P> {
            centre
                .windows(2)
                .flat_map(|w| {
                    let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
                    let l = dx.hypot(dy);
                    let (nx, ny) = (-dy / l * k, dx / l * k);
                    (0..20).map(move |i| [w[0][0] + dx * i as f64 / 20.0 + nx, w[0][1] + dy * i as f64 / 20.0 + ny])
                })
                .collect()
        };
        let mut pts = side(3.0);
        pts.extend(side(-3.0).into_iter().rev());
        let d = pts.iter().enumerate().map(|(i, p)| format!("{}{},{}", if i == 0 { "M" } else { "L" }, p[0], p[1])).collect::<String>() + "Z";
        let ribbon = format!(r#"<svg><path d="{d}"/></svg>"#);
        let sf = r#"<svg><line x1="150" y1="45" x2="150" y2="55"/></svg>"#;
        let m = build(1, "Test", &ribbon, Some(sf), Some(r#"<svg><text x="255" y="45">1</text><text x="255" y="155">2</text></svg>"#), None).unwrap();
        // Half a lap in is the bottom middle (on a doubled outline it would be back at the start).
        let h = screen(&m, BINS / 2);
        assert!((h[0] - 0.5).abs() < 0.03 && h[1] > 0.6, "{h:?}");
        assert!((screen(&m, BINS / 4)[0] - 1.0).abs() < 0.03, "{:?}", screen(&m, BINS / 4));
    }

    #[test]
    fn index_from_the_public_catalogue() {
        let c = serde_json::json!({"tracks": [{"configurations": [
            {"track_id": 580, "svg_folder_url": "https://members-assets.iracing.com/public/track-maps/tracks_adelaide/580-adelaide",
             "svg_local_path": "Dirt Oval/x/580-adelaide",
             "svg_urls": {"active": "https://members-assets.iracing.com/public/track-maps/tracks_adelaide/580-adelaide/active.svg",
                          "turns": "https://members-assets.iracing.com/public/track-maps/tracks_adelaide/580-adelaide/turns.svg",
                          "background": "https://members-assets.iracing.com/public/track-maps/tracks_adelaide/580-adelaide/background.svg"}},
            {"track_id": 1, "svg_folder_url": "https://x/1", "svg_urls": {"turns": "https://x/1/turns.svg"}}
        ]}]});
        let i = index_from_catalogue(&c);
        assert_eq!(i.as_object().unwrap().len(), 1, "no active layer, no entry");
        let e = &i["580"];
        assert_eq!(e["track_map"], "https://members-assets.iracing.com/public/track-maps/tracks_adelaide/580-adelaide/");
        assert_eq!(e["track_map_layers"], serde_json::json!({"active": "active.svg", "turns": "turns.svg"}));
        assert_eq!(e["mirror"], format!("{MIRROR_URL}/Dirt%20Oval/x/580-adelaide/"));
        // The index built into the app parses and covers every layout.
        let bundled: serde_json::Value = serde_json::from_str(BUNDLED_INDEX).unwrap();
        assert!(bundled.as_object().unwrap().len() >= 400);
    }
}
