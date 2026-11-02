//! Pixel snapping for the line-icon set (`assets/icons/v2`, and the files
//! that took its drawings over).
//!
//! resvg ignores `vector-effect: non-scaling-stroke`, and iced rasterises an
//! SVG at its physical size. So the line weight and the pixel alignment of an
//! icon are baked into its SVG bytes, once per physical size: every path is
//! rewritten into the physical-pixel coordinates of an `N × N` box, stroked
//! paths with their on-curve points on the line grid (so horizontal and
//! vertical lines cover exactly `W` pixel rows or columns) and filled paths
//! moved whole onto a pixel corner.
//!
//! The icons share one shape (see [`parse`]): a root `<svg>` whose viewBox is
//! the 19-unit icon frame of a 24-unit grid, an optional
//! `<g transform="matrix(…)">`, and `<path>` elements with `d`, `fill`,
//! `stroke` and `stroke-width`. A file outside that subset is not snapped;
//! [`plain`] then only sets its size and line weight.

use std::fmt::Write as _;

use crate::ui::style::ribbon_metrics::icon_metrics as m;

/// An icon in the supported subset, ready to snap at any size.
#[derive(Clone, Debug)]
pub struct LineIcon {
    /// The root's presentation attributes other than size, viewBox and
    /// stroke width (`fill`, `stroke`, caps, joins), in file order.
    root_attrs: Vec<(String, String)>,
    paths: Vec<IconPath>,
}

#[derive(Clone, Debug)]
struct IconPath {
    /// Subpaths in absolute frame units, after the group matrix; curves are
    /// cubic.
    subpaths: Vec<Vec<Seg>>,
    /// Every subpath ends with a close.
    closed: Vec<bool>,
    filled: bool,
    /// The path's own attributes other than `d`, in file order.
    attrs: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Seg {
    Move(P),
    Line(P),
    Cubic(P, P, P),
}

type P = (f64, f64);

/// Whether `source` is drawn on the icon frame, so the pipeline should draw
/// it (snapped, or by [`plain`] when it is outside the subset).
pub fn is_line_icon(source: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(source) else {
        return false;
    };
    let Some(root) = text.find("<svg") else {
        return false;
    };
    let head = &text[root..text[root..].find('>').map_or(text.len(), |end| root + end)];
    head.contains(m::ICON_FRAME_VIEWBOX)
}

/// Parse an icon in the supported subset: a root `<svg>` with the icon
/// frame as its viewBox; at most one `<g>`, with no attribute or one
/// `transform="matrix(…)"`; `<path>` elements with only `d`, `fill`,
/// `stroke` and `stroke-width`, whose data uses only `M L H V A C S Q Z`
/// (absolute and relative). Anything else is `Err` with the reason.
pub fn parse(source: &[u8]) -> Result<LineIcon, String> {
    let text = std::str::from_utf8(source).map_err(|_| "not UTF-8".to_string())?;
    let doc = roxmltree::Document::parse(text).map_err(|e| e.to_string())?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return Err("root is not <svg>".into());
    }
    let mut root_attrs = Vec::new();
    for attr in root.attributes() {
        match attr.name() {
            "viewBox" => {
                let numbers = numbers(attr.value())?;
                let frame = [
                    m::ICON_FRAME_MIN,
                    m::ICON_FRAME_MIN,
                    m::ICON_FRAME_SIZE,
                    m::ICON_FRAME_SIZE,
                ];
                if numbers.len() != 4
                    || numbers
                        .iter()
                        .zip(frame)
                        .any(|(a, b)| (a - f64::from(b)).abs() > 1e-9)
                {
                    return Err(format!("viewBox {:?} is not the icon frame", attr.value()));
                }
            }
            "width" | "height" | "stroke-width" => {}
            "fill" | "stroke" | "stroke-linecap" | "stroke-linejoin" => {
                root_attrs.push((attr.name().to_string(), attr.value().to_string()));
            }
            other => return Err(format!("root attribute {other:?}")),
        }
    }

    let mut matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut paths = Vec::new();
    let mut groups = 0;
    for child in root.children() {
        if child.is_text() && child.text().is_some_and(|t| t.trim().is_empty())
            || child.is_comment()
        {
            continue;
        }
        if !child.is_element() {
            return Err("unexpected content in <svg>".into());
        }
        match child.tag_name().name() {
            "g" => {
                groups += 1;
                if groups > 1 {
                    return Err("more than one <g>".into());
                }
                for attr in child.attributes() {
                    if attr.name() != "transform" {
                        return Err(format!("<g> attribute {:?}", attr.name()));
                    }
                    matrix = parse_matrix(attr.value())?;
                }
                for path in child.children() {
                    if path.is_text() && path.text().is_some_and(|t| t.trim().is_empty())
                        || path.is_comment()
                    {
                        continue;
                    }
                    if !path.is_element() || path.tag_name().name() != "path" {
                        return Err("<g> holds something other than <path>".into());
                    }
                    paths.push(path_of(path, matrix)?);
                }
            }
            "path" => paths.push(path_of(child, matrix)?),
            other => return Err(format!("element <{other}>")),
        }
    }
    Ok(LineIcon { root_attrs, paths })
}

fn numbers(text: &str) -> Result<Vec<f64>, String> {
    text.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<f64>().map_err(|_| format!("number {s:?}")))
        .collect()
}

fn parse_matrix(text: &str) -> Result<[f64; 6], String> {
    let inner = text
        .trim()
        .strip_prefix("matrix(")
        .and_then(|rest| rest.strip_suffix(')'))
        .ok_or_else(|| format!("transform {text:?} is not one matrix(…)"))?;
    let n = numbers(inner)?;
    <[f64; 6]>::try_from(n).map_err(|_| format!("matrix {text:?} needs six numbers"))
}

fn path_of(node: roxmltree::Node, matrix: [f64; 6]) -> Result<IconPath, String> {
    let mut d = None;
    let mut attrs = Vec::new();
    for attr in node.attributes() {
        match attr.name() {
            "d" => d = Some(attr.value()),
            "fill" | "stroke" | "stroke-width" => {
                attrs.push((attr.name().to_string(), attr.value().to_string()))
            }
            other => return Err(format!("<path> attribute {other:?}")),
        }
    }
    let d = d.ok_or("<path> without d")?;
    if let Some(bad) = d
        .chars()
        .find(|c| c.is_ascii_alphabetic() && !"MmLlHhVvAaCcSsQqZzEe".contains(*c))
    {
        return Err(format!("path command {bad:?}"));
    }
    let filled = attrs
        .iter()
        .any(|(name, value)| name == "fill" && value != "none");
    let map = |x: f64, y: f64| -> P {
        let [a, b, c, dd, e, f] = matrix;
        (a * x + c * y + e, b * x + dd * y + f)
    };

    let mut subpaths: Vec<Vec<Seg>> = Vec::new();
    let mut closed = Vec::new();
    let mut last = (0.0, 0.0);
    for segment in svgtypes::SimplifyingPathParser::from(d) {
        let segment = segment.map_err(|e| format!("path data: {e}"))?;
        use svgtypes::SimplePathSegment as S;
        match segment {
            S::MoveTo { x, y } => {
                subpaths.push(vec![Seg::Move(map(x, y))]);
                closed.push(false);
                last = (x, y);
            }
            S::LineTo { x, y } => {
                current(&mut subpaths)?.push(Seg::Line(map(x, y)));
                last = (x, y);
            }
            S::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                current(&mut subpaths)?.push(Seg::Cubic(map(x1, y1), map(x2, y2), map(x, y)));
                last = (x, y);
            }
            S::Quadratic { x1, y1, x, y } => {
                // Raised to a cubic, so every curve has one control point
                // per end.
                let c1 = (
                    last.0 + (x1 - last.0) * 2.0 / 3.0,
                    last.1 + (y1 - last.1) * 2.0 / 3.0,
                );
                let c2 = (x + (x1 - x) * 2.0 / 3.0, y + (y1 - y) * 2.0 / 3.0);
                current(&mut subpaths)?.push(Seg::Cubic(
                    map(c1.0, c1.1),
                    map(c2.0, c2.1),
                    map(x, y),
                ));
                last = (x, y);
            }
            S::ClosePath => {
                if let Some(flag) = closed.last_mut() {
                    *flag = true;
                }
                // The parser starts the next subpath with its own MoveTo.
            }
        }
    }
    Ok(IconPath {
        subpaths,
        closed,
        filled,
        attrs,
    })
}

fn current(subpaths: &mut [Vec<Seg>]) -> Result<&mut Vec<Seg>, String> {
    subpaths
        .last_mut()
        .ok_or_else(|| "path data does not start with M".to_string())
}

/// Physical line weight, in pixels, at a window scale factor: 1 below
/// 150 %, [`icon_metrics::LINE_WEIGHT_150`](m::LINE_WEIGHT_150) from 150 %
/// up to 175 %, 2 from 175 %.
pub fn line_weight(scale_factor: f32) -> u32 {
    // The scale arrives in thousandths (see `icons::set_scale_factor`);
    // compare in those so 1.75 is not lost to rounding.
    let thousandths = (scale_factor * 1000.0).round() as u32;
    if thousandths >= (m::DOUBLE_LINE_FROM * 1000.0).round() as u32 {
        2
    } else if thousandths >= (m::LINE_150_FROM * 1000.0).round() as u32 {
        m::LINE_WEIGHT_150
    } else {
        1
    }
}

/// Physical size of an icon `size` logical pixels square at `scale_factor`.
pub fn physical_size(size: f32, scale_factor: f32) -> u32 {
    (size * scale_factor).round().max(1.0) as u32
}

impl LineIcon {
    /// The icon as an `n × n` SVG with `w`-pixel lines on the pixel grid.
    pub fn snap(&self, n: u32, w: u32) -> String {
        let k = f64::from(n) / m::ICON_FRAME_SIZE as f64;
        let to_px = |(x, y): P| -> P {
            (
                (x - m::ICON_FRAME_MIN as f64) * k,
                (y - m::ICON_FRAME_MIN as f64) * k,
            )
        };
        let grid = |v: f64| -> f64 {
            if w % 2 == 1 {
                v.floor() + 0.5
            } else {
                v.round()
            }
        };

        let mut out = String::with_capacity(256 + self.paths.len() * 96);
        let _ = write!(
            out,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{n}" height="{n}" viewBox="0 0 {n} {n}""#
        );
        for (name, value) in &self.root_attrs {
            let _ = write!(out, r#" {name}="{value}""#);
        }
        let _ = write!(out, r#" stroke-width="{w}">"#);

        for path in &self.paths {
            let mut d = String::new();
            for (subpath, &closed) in path.subpaths.iter().zip(&path.closed) {
                if path.filled {
                    // Whole subpath moves so its box's top-left corner is on
                    // a pixel corner; it keeps its size.
                    let points = subpath.iter().flat_map(|seg| match *seg {
                        Seg::Move(p) | Seg::Line(p) => [Some(p), None, None],
                        Seg::Cubic(a, b, c) => [Some(a), Some(b), Some(c)],
                    });
                    let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
                    for p in points.flatten().map(to_px) {
                        x0 = x0.min(p.0);
                        y0 = y0.min(p.1);
                    }
                    let (dx, dy) = (x0.round() - x0, y0.round() - y0);
                    let t = |p: P| {
                        let p = to_px(p);
                        (p.0 + dx, p.1 + dy)
                    };
                    for seg in subpath {
                        match *seg {
                            Seg::Move(p) => push_cmd(&mut d, 'M', &[t(p)]),
                            Seg::Line(p) => push_cmd(&mut d, 'L', &[t(p)]),
                            Seg::Cubic(a, b, c) => push_cmd(&mut d, 'C', &[t(a), t(b), t(c)]),
                        }
                    }
                } else {
                    // On-curve points to the line grid; each control point
                    // moves with the on-curve point it belongs to.
                    let mut start = (0.0, 0.0);
                    let mut snapped_start = (0.0, 0.0);
                    for seg in subpath {
                        match *seg {
                            Seg::Move(p) | Seg::Line(p) => {
                                let q = to_px(p);
                                let s = (grid(q.0), grid(q.1));
                                let cmd = if matches!(seg, Seg::Move(_)) {
                                    'M'
                                } else {
                                    'L'
                                };
                                push_cmd(&mut d, cmd, &[s]);
                                start = q;
                                snapped_start = s;
                            }
                            Seg::Cubic(a, b, c) => {
                                let (a, b, e) = (to_px(a), to_px(b), to_px(c));
                                let se = (grid(e.0), grid(e.1));
                                let c1 = (
                                    a.0 + snapped_start.0 - start.0,
                                    a.1 + snapped_start.1 - start.1,
                                );
                                let c2 = (b.0 + se.0 - e.0, b.1 + se.1 - e.1);
                                push_cmd(&mut d, 'C', &[c1, c2, se]);
                                start = e;
                                snapped_start = se;
                            }
                        }
                    }
                }
                if closed {
                    d.push('Z');
                }
            }
            let _ = write!(out, r#"<path d="{d}""#);
            for (name, value) in &path.attrs {
                if name != "stroke-width" {
                    let _ = write!(out, r#" {name}="{value}""#);
                }
            }
            out.push_str("/>");
        }
        out.push_str("</svg>");
        out
    }
}

fn push_cmd(d: &mut String, cmd: char, points: &[P]) {
    d.push(cmd);
    for (i, &(x, y)) in points.iter().enumerate() {
        if i > 0 {
            d.push(' ');
        }
        push_num(d, x);
        d.push(' ');
        push_num(d, y);
    }
}

/// A coordinate to three decimals, without trailing zeros.
fn push_num(d: &mut String, v: f64) {
    let v = (v * 1000.0).round() / 1000.0;
    let v = if v == 0.0 { 0.0 } else { v };
    let _ = write!(d, "{v}");
}

/// A file outside the subset, unsnapped: its size set to `n` and every
/// `stroke-width` to `w × 19 / n` frame units, so its lines are still
/// `w` physical pixels wide.
pub fn plain(source: &[u8], n: u32, w: u32) -> Vec<u8> {
    let text = String::from_utf8_lossy(source);
    let stroke = f64::from(w) * m::ICON_FRAME_SIZE as f64 / f64::from(n);
    let stroke = format!("{}", (stroke * 10_000.0).round() / 10_000.0);
    let root_end = text
        .find("<svg")
        .and_then(|start| text[start..].find('>').map(|end| start + end))
        .unwrap_or(0);
    let (head, body) = text.split_at(root_end);
    let size = n.to_string();
    let head = set_attrs(head, "width", &size);
    let head = set_attrs(&head, "height", &size);
    let mut out = set_attrs(&head, "stroke-width", &stroke);
    out.push_str(&set_attrs(body, "stroke-width", &stroke));
    out.into_bytes()
}

/// `text` with the value of every attribute `name` (a whole attribute name,
/// not the tail of a longer one) set to `value`.
fn set_attrs(text: &str, name: &str, value: &str) -> String {
    let needle = format!("{name}=\"");
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(&needle) {
        let whole = rest[..at]
            .chars()
            .next_back()
            .is_none_or(char::is_whitespace);
        let after = &rest[at + needle.len()..];
        let Some(end) = after.find('"') else { break };
        out.push_str(&rest[..at + needle.len()]);
        out.push_str(if whole { value } else { &after[..end] });
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon(paths: &str) -> LineIcon {
        let source = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="2.5 2.5 19 19" fill="none" stroke="#ffffff" stroke-width="1.1875" stroke-linecap="round" stroke-linejoin="round"><g>{paths}</g></svg>"##
        );
        parse(source.as_bytes()).expect("test icon parses")
    }

    /// The numbers of the first `d` attribute of `svg`.
    fn first_d(svg: &str) -> Vec<f64> {
        let start = svg.find(" d=\"").unwrap() + 4;
        let end = start + svg[start..].find('"').unwrap();
        svg[start..end]
            .split(|c: char| c.is_ascii_alphabetic() || c == ' ')
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect()
    }

    #[test]
    fn vertical_line_lands_on_pixel_centre_with_one_px_and_edge_with_two() {
        let line = icon(r#"<path d="M4 5V19"/>"#);
        for n in [16, 20, 24, 32, 48] {
            let one = first_d(&line.snap(n, 1));
            assert_eq!(one[0], one[2], "N={n}: stays vertical");
            assert_eq!(one[0].fract(), 0.5, "N={n}, W=1: x on a pixel centre");
            assert_eq!(one[1].fract(), 0.5);
            let two = first_d(&line.snap(n, 2));
            assert_eq!(two[0].fract(), 0.0, "N={n}, W=2: x on a pixel edge");
            assert_eq!(two[0], two[2]);
            // Within half a pixel of where the frame puts it.
            let exact = (4.0 - 2.5) * f64::from(n) / 19.0;
            assert!((one[0] - exact).abs() <= 1.0 && (two[0] - exact).abs() <= 0.5);
        }
    }

    #[test]
    fn stroke_width_is_the_line_weight() {
        let line = icon(r#"<path d="M4 5V19"/>"#);
        assert!(line.snap(24, 1).contains(r#"stroke-width="1""#));
        assert!(line.snap(32, 2).contains(r#"stroke-width="2""#));
        assert!(line.snap(32, 2).contains(r#"viewBox="0 0 32 32""#));
        assert!(!line.snap(32, 2).contains("<g"));
    }

    #[test]
    fn filled_square_keeps_its_size_and_sits_on_a_pixel_corner() {
        let square = icon(r##"<path d="M5.3 6.7h3v3h-3z" fill="#4a9eff" stroke="none"/>"##);
        for n in [16, 20, 24, 28, 32, 40, 48] {
            let d = first_d(&square.snap(n, 1));
            let xs: Vec<f64> = d.iter().step_by(2).copied().collect();
            let ys: Vec<f64> = d.iter().skip(1).step_by(2).copied().collect();
            let x0 = xs.iter().copied().fold(f64::INFINITY, f64::min);
            let y0 = ys.iter().copied().fold(f64::INFINITY, f64::min);
            let x1 = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let y1 = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            assert_eq!(
                (x0.fract(), y0.fract()),
                (0.0, 0.0),
                "N={n}: corner on a pixel"
            );
            let side = 3.0 * f64::from(n) / 19.0;
            assert!((x1 - x0 - side).abs() < 0.002, "N={n}: width kept");
            assert!((y1 - y0 - side).abs() < 0.002, "N={n}: height kept");
        }
    }

    #[test]
    fn arc_keeps_its_end_points_on_the_grid() {
        let arc = icon(r#"<path d="M4 18A8 8 0 0 1 20 18"/>"#);
        for (n, w) in [(16, 1), (24, 1), (28, 2), (32, 2), (48, 2)] {
            let svg = arc.snap(n, w);
            let d = first_d(&svg);
            assert!(svg.contains('C'), "the arc becomes curves");
            // Start point, then each curve's end point (every 6th number
            // after the start).
            let mut ends = vec![(d[0], d[1])];
            ends.extend(d[2..].chunks(6).map(|c| (c[4], c[5])));
            let on_grid = |v: f64| {
                if w == 1 {
                    v.fract() == 0.5
                } else {
                    v.fract() == 0.0
                }
            };
            for (x, y) in ends {
                assert!(on_grid(x) && on_grid(y), "N={n}: ({x}, {y}) on the grid");
            }
        }
    }

    #[test]
    fn group_matrix_is_applied() {
        let source = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="2.5 2.5 19 19" fill="none" stroke="#ffffff" stroke-width="1.1875"><g transform="matrix(0.5 0 0 0.5 6.25 6.25)"><path d="M2.5 2.5H21.5"/></g></svg>"##;
        let d = first_d(&parse(source).unwrap().snap(19, 1));
        // 2.5 → 7.5 → 5 px, 21.5 → 17 → 14.5 px.
        assert_eq!(d, vec![5.5, 5.5, 14.5, 5.5]);
    }

    #[test]
    fn outside_the_subset_is_refused() {
        for bad in [
            r#"<svg viewBox="0 0 24 24"><path d="M0 0H1"/></svg>"#,
            r#"<svg viewBox="2.5 2.5 19 19"><circle r="3"/></svg>"#,
            r#"<svg viewBox="2.5 2.5 19 19"><g transform="translate(1 1)"><path d="M0 0H1"/></g></svg>"#,
            r#"<svg viewBox="2.5 2.5 19 19"><path d="M0 0T4 4"/></svg>"#,
            r#"<svg viewBox="2.5 2.5 19 19"><path d="M0 0H1" opacity="0.5"/></svg>"#,
            r#"<svg viewBox="2.5 2.5 19 19"><g><g><path d="M0 0H1"/></g></g></svg>"#,
        ] {
            assert!(parse(bad.as_bytes()).is_err(), "{bad}");
        }
    }

    #[test]
    fn plain_sets_size_and_compensated_stroke() {
        let source = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="2.5 2.5 19 19" stroke-width="1.1875"><circle r="3" stroke-width="2"/></svg>"##;
        let out = String::from_utf8(plain(source, 38, 2)).unwrap();
        assert!(out.contains(r#"width="38" height="38""#), "{out}");
        assert_eq!(out.matches(r#"stroke-width="1""#).count(), 2, "{out}");
        assert!(out.contains("<circle r=\"3\""));
    }

    /// Every file under `assets/icons` the pipeline handles.
    fn line_icon_files() -> Vec<(String, Vec<u8>)> {
        fn walk(dir: &std::path::Path, out: &mut Vec<(String, Vec<u8>)>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "svg") {
                    let bytes = std::fs::read(&path).unwrap();
                    if is_line_icon(&bytes) {
                        out.push((path.display().to_string(), bytes));
                    }
                }
            }
        }
        let mut out = Vec::new();
        walk(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons")),
            &mut out,
        );
        out
    }

    #[test]
    fn every_line_icon_file_stays_in_the_supported_subset() {
        let files = line_icon_files();
        assert!(files.len() > 250, "only {} line icons found", files.len());
        let outside: Vec<String> = files
            .iter()
            .filter_map(|(path, bytes)| parse(bytes).err().map(|e| format!("{path}: {e}")))
            .collect();
        assert!(outside.is_empty(), "{outside:#?}");
    }

    #[test]
    fn line_weight_follows_the_table() {
        for (scale, w) in [
            (1.0, 1),
            (1.25, 1),
            (1.333, 1),
            (1.5, m::LINE_WEIGHT_150),
            (1.6, m::LINE_WEIGHT_150),
            (1.75, 2),
            (2.0, 2),
            (3.0, 2),
        ] {
            assert_eq!(line_weight(scale), w, "scale {scale}");
        }
        for (size, scale, n) in [
            (16.0, 1.0, 16),
            (16.0, 1.25, 20),
            (16.0, 1.5, 24),
            (16.0, 1.75, 28),
            (16.0, 2.0, 32),
            (32.0, 1.25, 40),
            (32.0, 1.75, 56),
        ] {
            assert_eq!(physical_size(size, scale), n);
        }
    }
}
