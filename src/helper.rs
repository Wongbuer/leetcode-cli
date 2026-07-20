//! A set of helper traits
pub use self::{
    digit::Digit,
    file::{code_path, load_script, test_cases_path},
    filter::{filter, squash},
    html::{HTML, RenderedDesc},
    image::{print_desc_with_images, print_images, render_markdown, supports_inline_images},
};

/// Convert i32 to specific digits string.
mod digit {
    /// Abstract Digit trait, fill the empty space to specific length.
    pub trait Digit<T> {
        fn digit(self, d: T) -> String;
    }

    impl Digit<i32> for i32 {
        fn digit(self, d: i32) -> String {
            let mut s = self.to_string();
            let space = " ".repeat((d as usize) - s.len());
            s.push_str(&space);

            s
        }
    }

    impl Digit<i32> for String {
        fn digit(self, d: i32) -> String {
            let mut s = self.clone();
            let space = " ".repeat((d as usize) - self.len());
            s.push_str(&space);

            s
        }
    }

    impl Digit<i32> for &'static str {
        fn digit(self, d: i32) -> String {
            let mut s = self.to_string();
            let space = " ".repeat((d as usize) - self.len());
            s.push_str(&space);

            s
        }
    }
}

/// Question filter tool
mod filter {
    use crate::cache::models::Problem;
    /// Abstract query filter
    ///
    /// ```sh
    ///     -q, --query <query>          Filter questions by conditions:
    ///                                  Uppercase means negative
    ///                                  e = easy     E = m+h
    ///                                  m = medium   M = e+h
    ///                                  h = hard     H = e+m
    ///                                  d = done     D = not done
    ///                                  l = locked   L = not locked
    ///                                  s = starred  S = not starred
    /// ```
    pub fn filter(ps: &mut Vec<Problem>, query: String) {
        for p in query.chars() {
            match p {
                'l' => ps.retain(|x| x.locked),
                'L' => ps.retain(|x| !x.locked),
                's' => ps.retain(|x| x.starred),
                'S' => ps.retain(|x| !x.starred),
                'e' => ps.retain(|x| x.level == 1),
                'E' => ps.retain(|x| x.level != 1),
                'm' => ps.retain(|x| x.level == 2),
                'M' => ps.retain(|x| x.level != 2),
                'h' => ps.retain(|x| x.level == 3),
                'H' => ps.retain(|x| x.level != 3),
                'd' => ps.retain(|x| x.status == "ac"),
                'D' => ps.retain(|x| x.status != "ac"),
                _ => {}
            }
        }
    }

    /// Squash questions and ids
    pub fn squash(ps: &mut Vec<Problem>, ids: Vec<String>) -> crate::Result<()> {
        use std::collections::HashMap;

        let mut map: HashMap<String, bool> = HashMap::new();
        ids.iter().for_each(|x| {
            map.insert(x.to_string(), true).unwrap_or_default();
        });

        ps.retain(|x| map.contains_key(&x.id.to_string()));
        Ok(())
    }
}

pub fn superscript(n: u8) -> String {
    match n {
        x if x >= 10 => format!("{}{}", superscript(n / 10), superscript(n % 10)),
        0 => "⁰".to_string(),
        1 => "¹".to_string(),
        2 => "²".to_string(),
        3 => "³".to_string(),
        4 => "⁴".to_string(),
        5 => "⁵".to_string(),
        6 => "⁶".to_string(),
        7 => "⁷".to_string(),
        8 => "⁸".to_string(),
        9 => "⁹".to_string(),
        _ => n.to_string(),
    }
}

pub fn subscript(n: u8) -> String {
    match n {
        x if x >= 10 => format!("{}{}", subscript(n / 10), subscript(n % 10)),
        0 => "₀".to_string(),
        1 => "₁".to_string(),
        2 => "₂".to_string(),
        3 => "₃".to_string(),
        4 => "₄".to_string(),
        5 => "₅".to_string(),
        6 => "₆".to_string(),
        7 => "₇".to_string(),
        8 => "₈".to_string(),
        9 => "₉".to_string(),
        _ => n.to_string(),
    }
}

/// Render html to command-line
mod html {
    use crate::helper::{subscript, superscript};
    use regex::Captures;
    use scraper::Html;

    /// Text + ordered image URLs extracted from a problem statement.
    #[derive(Debug, Default, Clone)]
    pub struct RenderedDesc {
        pub text: String,
        pub images: Vec<String>,
    }

    /// Html render plugin
    pub trait HTML {
        fn render(&self) -> String;
        fn render_with_images(&self) -> RenderedDesc;
    }

    impl HTML for String {
        fn render(&self) -> String {
            self.render_with_images().text
        }

        fn render_with_images(&self) -> RenderedDesc {
            // Match a full <img ...> / <img .../> tag and capture src in any attribute order.
            let img_re = regex::Regex::new(
                r#"(?is)<img\b[^>]*?\bsrc\s*=\s*(?:"([^"]+)"|'([^']+)'|([^\s>]+))[^>]*/?>"#,
            )
            .unwrap();
            let mut images = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for cap in img_re.captures_iter(self) {
                let src = cap
                    .get(1)
                    .or_else(|| cap.get(2))
                    .or_else(|| cap.get(3))
                    .map(|m| m.as_str().trim())
                    .unwrap_or("");
                if !src.is_empty() && seen.insert(src.to_string()) {
                    images.push(src.to_string());
                }
            }

            // Replace each full img tag with a text placeholder before stripping tags.
            let mut idx = 0usize;
            let mut seen2 = std::collections::HashSet::new();
            let with_placeholders = img_re.replace_all(self, |cap: &Captures| {
                let src = cap
                    .get(1)
                    .or_else(|| cap.get(2))
                    .or_else(|| cap.get(3))
                    .map(|m| m.as_str().trim())
                    .unwrap_or("");
                if src.is_empty() || !seen2.insert(src.to_string()) {
                    return String::new();
                }
                idx += 1;
                format!("\n[图片 {idx}]\n")
            });

            let sup_re = regex::Regex::new(r"<sup>(?P<num>[0-9]*)</sup>").unwrap();
            let sub_re = regex::Regex::new(r"<sub>(?P<num>[0-9]*)</sub>").unwrap();

            let res = sup_re.replace_all(&with_placeholders, |cap: &Captures| {
                let num: u8 = cap["num"].to_string().parse().unwrap_or(0);
                superscript(num)
            });

            let res = sub_re.replace_all(&res, |cap: &Captures| {
                let num: u8 = cap["num"].to_string().parse().unwrap_or(0);
                subscript(num)
            });

            let frag = Html::parse_fragment(&res);
            let text = frag
                .root_element()
                .text()
                .fold(String::new(), |acc, e| acc + e);

            RenderedDesc { text, images }
        }
    }
}

/// Inline terminal images (Kitty graphics protocol) with URL fallback.
mod image {
    use base64::{Engine, engine::general_purpose::STANDARD as B64};
    use std::io::{IsTerminal, Write};
    use std::path::PathBuf;
    use std::time::Duration;

    /// Ghostty / Kitty (and a few others) understand the Kitty graphics protocol.
    pub fn supports_inline_images() -> bool {
        if !std::io::stdout().is_terminal() {
            return false;
        }
        // Explicit opt-out.
        if std::env::var_os("LEETCODE_NO_IMAGES").is_some() {
            return false;
        }
        // Explicit opt-in for odd TERM values.
        if std::env::var_os("LEETCODE_FORCE_IMAGES").is_some() {
            return true;
        }

        let term = std::env::var("TERM").unwrap_or_default().to_lowercase();
        let program = std::env::var("TERM_PROGRAM").unwrap_or_default().to_lowercase();

        if program.contains("ghostty")
            || program.contains("kitty")
            || program.contains("wezterm")
            || program.contains("konsole")
        {
            return true;
        }
        if !std::env::var("KITTY_WINDOW_ID").unwrap_or_default().is_empty()
            || !std::env::var("WEZTERM_EXECUTABLE")
                .unwrap_or_default()
                .is_empty()
            || !std::env::var("GHOSTTY_RESOURCES_DIR")
                .unwrap_or_default()
                .is_empty()
        {
            return true;
        }
        // TERM hints (e.g. xterm-ghostty, xterm-kitty).
        term.contains("kitty") || term.contains("ghostty") || term.contains("wezterm")
    }

    fn cache_dir() -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        let dir = home.join(".leetcode").join("images");
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    }

    fn cache_path_for(url: &str) -> Option<PathBuf> {
        let dir = cache_dir()?;
        let digest = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut h = DefaultHasher::new();
            url.hash(&mut h);
            format!("{:016x}", h.finish())
        };
        let ext = url
            .rsplit('.')
            .next()
            .and_then(|e| {
                let e = e.split('?').next().unwrap_or(e).to_lowercase();
                match e.as_str() {
                    "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => Some(e),
                    _ => None,
                }
            })
            .unwrap_or_else(|| "img".into());
        Some(dir.join(format!("{digest}.{ext}")))
    }

    fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
        if let Some(path) = cache_path_for(url) {
            if path.exists() {
                if let Ok(bytes) = std::fs::read(&path) {
                    if !bytes.is_empty() {
                        return Some(bytes);
                    }
                }
            }
        }

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("leetcode-cli")
            .build()
            .ok()?;
        let bytes = client
            .get(url)
            .send()
            .ok()?
            .error_for_status()
            .ok()?
            .bytes()
            .ok()?;
        let bytes = bytes.to_vec();
        if let Some(path) = cache_path_for(url) {
            let _ = std::fs::write(path, &bytes);
        }
        Some(bytes)
    }

    /// Terminal size in cells. Prefers `$COLUMNS`/`$LINES`, then ioctl, else 80x24.
    fn term_cells() -> (u32, u32) {
        let cols = std::env::var("COLUMNS")
            .ok()
            .and_then(|s| s.parse().ok());
        let rows = std::env::var("LINES")
            .ok()
            .and_then(|s| s.parse().ok());
        if let (Some(c), Some(r)) = (cols, rows) {
            if c > 0 && r > 0 {
                return (c, r);
            }
        }

        #[cfg(unix)]
        {
            // TIOCGWINSZ without pulling in extra crates.
            #[repr(C)]
            struct Winsize {
                ws_row: u16,
                ws_col: u16,
                ws_xpixel: u16,
                ws_ypixel: u16,
            }
            // macOS / Linux share the same request number for TIOCGWINSZ on our targets.
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            const TIOCGWINSZ: u64 = 0x40087468;
            #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
            const TIOCGWINSZ: u64 = 0x5413;

            unsafe {
                let mut ws = Winsize {
                    ws_row: 0,
                    ws_col: 0,
                    ws_xpixel: 0,
                    ws_ypixel: 0,
                };
                if libc::ioctl(1, TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 {
                    return (
                        cols.unwrap_or(ws.ws_col as u32).max(ws.ws_col as u32),
                        rows.unwrap_or(ws.ws_row as u32).max(ws.ws_row as u32),
                    );
                }
            }
        }

        (cols.unwrap_or(80), rows.unwrap_or(24))
    }

    /// Decode any common format, downscale to fit the terminal, re-encode as PNG.
    /// Returns (png_bytes, display_cols, display_rows) for Kitty placement.
    fn fit_to_terminal(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
        let img = image::load_from_memory(bytes).ok()?;
        let (term_cols, term_rows) = term_cells();
        // Leave a small margin; cap height so one figure never eats the whole screen.
        let max_cols = term_cols.saturating_sub(2).clamp(20, 120);
        let max_rows = (term_rows.saturating_mul(6) / 10).clamp(8, 40); // ~60% of height

        // Approximate cell metrics (px). Slightly conservative so we don't overflow.
        let px_per_col = 12u32;
        let px_per_row = 24u32;
        let max_px_w = max_cols.saturating_mul(px_per_col);
        let max_px_h = max_rows.saturating_mul(px_per_row);

        let img = if img.width() > max_px_w || img.height() > max_px_h {
            img.resize(max_px_w, max_px_h, image::imageops::FilterType::Triangle)
        } else {
            img
        };

        let disp_cols = (img.width().saturating_add(px_per_col - 1) / px_per_col)
            .clamp(1, max_cols);
        let disp_rows = (img.height().saturating_add(px_per_row - 1) / px_per_row)
            .clamp(1, max_rows);

        let mut out = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut out);
            img.write_to(&mut cursor, image::ImageFormat::Png).ok()?;
        }
        Some((out, disp_cols, disp_rows))
    }

    /// Emit one image via the Kitty graphics protocol, sized to terminal cells.
    fn print_kitty_image(bytes: &[u8], id: u32) -> bool {
        let Some((png, cols, rows)) = fit_to_terminal(bytes) else {
            return false;
        };
        let mut out = std::io::stdout().lock();
        // f=100: raw PNG; c/r reserve cell grid so following text doesn't overlap.
        let encoded = B64.encode(&png);
        let chunks: Vec<&[u8]> = encoded.as_bytes().chunks(4096).collect();
        if chunks.is_empty() {
            return false;
        }
        for (i, chunk) in chunks.iter().enumerate() {
            let first = i == 0;
            let last = i + 1 == chunks.len();
            let m = if last { 0 } else { 1 };
            let piece = std::str::from_utf8(chunk).unwrap_or("");
            if first {
                // a=T transmit+display, q=2 quiet
                let _ = write!(
                    out,
                    "\x1b_Ga=T,f=100,c={cols},r={rows},q=2,m={m},i={id};{piece}\x1b\\"
                );
            } else {
                let _ = write!(out, "\x1b_Gm={m};{piece}\x1b\\");
            }
        }
        let _ = writeln!(out);
        let _ = out.flush();
        true
    }

    /// Match a standalone `[图片 N]` placeholder line.
    fn placeholder_index(line: &str) -> Option<usize> {
        let t = line.trim();
        let rest = t.strip_prefix("[图片 ")?.strip_suffix(']')?;
        rest.parse::<usize>().ok().filter(|n| *n > 0)
    }

    /// Print a rendered description with images at their placeholders.
    ///
    /// - Kitty/Ghostty: true image right under `[图片 N]`, plus URL on that line
    /// - Other terminals: `[图片 N] <url>` at the same position (no end dump)
    /// - GFM pipe tables are prettified with box-drawing characters
    pub fn print_desc_with_images(text: &str, images: &[String]) {
        let inline = supports_inline_images();
        let pretty = pretty_tables(text);
        for line in pretty.lines() {
            if let Some(n) = placeholder_index(line) {
                let url = images.get(n - 1).map(String::as_str).unwrap_or("");
                if url.is_empty() {
                    println!("{line}");
                    continue;
                }
                println!("[图片 {n}] {url}");
                if inline {
                    if let Some(bytes) = fetch_bytes(url) {
                        let _ = print_kitty_image(&bytes, (n as u32) + 1000);
                    }
                }
            } else {
                println!("{line}");
            }
        }
    }

    /// Print images only (legacy / append-at-end). Prefer `print_desc_with_images`.
    pub fn print_images(images: &[String]) {
        if images.is_empty() {
            return;
        }
        let inline = supports_inline_images();
        println!();
        for (i, url) in images.iter().enumerate() {
            let n = i + 1;
            println!("[图片 {n}] {url}");
            if inline {
                if let Some(bytes) = fetch_bytes(url) {
                    let _ = print_kitty_image(&bytes, (n as u32) + 1000);
                }
            }
        }
    }

    /// Display width that respects CJK / wide glyphs.
    fn disp_width(s: &str) -> usize {
        use unicode_width::UnicodeWidthStr;
        UnicodeWidthStr::width(s)
    }

    fn pad_cell(s: &str, width: usize) -> String {
        let w = disp_width(s);
        if w >= width {
            s.to_string()
        } else {
            format!("{s}{}", " ".repeat(width - w))
        }
    }

    fn is_md_separator_row(line: &str) -> bool {
        let t = line.trim();
        if !t.contains('|') {
            return false;
        }
        // cells are only dashes/colons/spaces
        t.trim_matches('|')
            .split('|')
            .all(|c| {
                let c = c.trim();
                !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
            })
    }

    fn is_md_table_row(line: &str) -> bool {
        let t = line.trim();
        t.starts_with('|') && t.matches('|').count() >= 2
    }

    fn split_md_row(line: &str) -> Vec<String> {
        let t = line.trim().trim_matches('|');
        t.split('|')
            .map(|c| c.trim().to_string())
            .collect()
    }

    /// Convert contiguous GFM pipe-table blocks into box-drawing tables.
    fn pretty_tables(text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let mut out = String::new();
        let mut i = 0usize;
        while i < lines.len() {
            // Need header + separator at minimum.
            if i + 1 < lines.len()
                && is_md_table_row(lines[i])
                && is_md_separator_row(lines[i + 1])
            {
                let mut block = vec![lines[i]];
                i += 1; // separator
                i += 1;
                while i < lines.len() && is_md_table_row(lines[i]) && !is_md_separator_row(lines[i])
                {
                    block.push(lines[i]);
                    i += 1;
                }
                out.push_str(&render_box_table(&block));
                out.push('\n');
                continue;
            }
            out.push_str(lines[i]);
            out.push('\n');
            i += 1;
        }
        // trim final extra newline to match line-by-line printing
        if out.ends_with('\n') {
            out.pop();
        }
        out
    }

    fn render_box_table(rows_raw: &[&str]) -> String {
        if rows_raw.is_empty() {
            return String::new();
        }
        let rows: Vec<Vec<String>> = rows_raw.iter().map(|r| split_md_row(r)).collect();
        let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if cols == 0 {
            return rows_raw.join("\n");
        }
        // Normalize row length.
        let rows: Vec<Vec<String>> = rows
            .into_iter()
            .map(|mut r| {
                while r.len() < cols {
                    r.push(String::new());
                }
                if r.len() > cols {
                    r.truncate(cols);
                }
                r
            })
            .collect();

        let mut widths = vec![0usize; cols];
        for r in &rows {
            for (j, cell) in r.iter().enumerate() {
                widths[j] = widths[j].max(disp_width(cell)).max(1);
            }
        }

        // Cap total width roughly to terminal so wide discuss tables don't explode.
        let (term_cols, _) = term_cells();
        let max_total = term_cols.saturating_sub(4) as usize;
        let min_each = 4usize;
        let mut total: usize = widths.iter().sum::<usize>() + 3 * cols + 1;
        if total > max_total && cols > 0 {
            // shrink widest columns first until we fit or hit min_each
            while total > max_total {
                if let Some((idx, _)) = widths
                    .iter()
                    .enumerate()
                    .filter(|(_, w)| **w > min_each)
                    .max_by_key(|(_, w)| *w)
                {
                    widths[idx] -= 1;
                    total -= 1;
                } else {
                    break;
                }
            }
        }

        let trunc = |s: &str, w: usize| -> String {
            use unicode_width::UnicodeWidthChar;
            if disp_width(s) <= w {
                return s.to_string();
            }
            let mut out = String::new();
            let mut used = 0usize;
            for ch in s.chars() {
                let cw = UnicodeWidthChar::width(ch).unwrap_or(1);
                if used + cw + 1 > w {
                    break;
                }
                out.push(ch);
                used += cw;
            }
            out.push('…');
            out
        };

        let hline = |left: char, mid: char, right: char, fill: char| -> String {
            let mut s = String::new();
            s.push(left);
            for (j, w) in widths.iter().enumerate() {
                s.push_str(&fill.to_string().repeat(w + 2));
                s.push(if j + 1 == cols { right } else { mid });
            }
            s
        };

        let mut out = String::new();
        out.push_str(&hline('┌', '┬', '┐', '─'));
        out.push('\n');
        for (ri, row) in rows.iter().enumerate() {
            out.push('│');
            for (j, cell) in row.iter().enumerate() {
                let cell = trunc(cell, widths[j]);
                out.push(' ');
                out.push_str(&pad_cell(&cell, widths[j]));
                out.push(' ');
                out.push('│');
            }
            out.push('\n');
            if ri == 0 {
                out.push_str(&hline('├', '┼', '┤', '─'));
                out.push('\n');
            }
        }
        out.push_str(&hline('└', '┴', '┘', '─'));
        out
    }

    /// Render a markdown discuss post: text + image urls (from `![](url)`).
    pub fn render_markdown(md: &str) -> super::html::RenderedDesc {
        let img_re = regex::Regex::new(r#"!\[[^\]]*\]\(([^)\s]+)(?:\s+"[^"]*")?\)"#).unwrap();
        let mut images = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for cap in img_re.captures_iter(md) {
            let src = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            // strip optional leetcode {:width=...} suffix artifacts if any
            let src = src.split('{').next().unwrap_or(src).trim();
            if !src.is_empty() && seen.insert(src.to_string()) {
                images.push(src.to_string());
            }
        }

        let mut idx = 0usize;
        let mut seen2 = std::collections::HashSet::new();
        let with_ph = img_re.replace_all(md, |cap: &regex::Captures| {
            let src = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            let src = src.split('{').next().unwrap_or(src).trim();
            if src.is_empty() || !seen2.insert(src.to_string()) {
                return String::new();
            }
            idx += 1;
            format!("\n[图片 {idx}]\n")
        });

        // Light markdown cleanup for terminal readability.
        let mut text = with_ph.to_string();
        // strip {:width=N} artifacts that follow images in leetcode md
        let artifact = regex::Regex::new(r"\{:[^}]*\}").unwrap();
        text = artifact.replace_all(&text, "").to_string();
        // links [text](url) -> text (url)
        let link_re = regex::Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
        text = link_re
            .replace_all(&text, |c: &regex::Captures| {
                format!("{} ({})", &c[1], &c[2])
            })
            .to_string();
        // bold/italic markers
        for pat in ["**", "__", "*", "_", "~~", "`"] {
            text = text.replace(pat, "");
        }
        // headings
        let heading = regex::Regex::new(r"(?m)^#{1,6}\s*").unwrap();
        text = heading.replace_all(&text, "").to_string();
        // blockquote
        let bq = regex::Regex::new(r"(?m)^>\s?").unwrap();
        text = bq.replace_all(&text, "").to_string();

        super::html::RenderedDesc { text, images }
    }
}

mod file {
    /// Convert file suffix from language type
    pub fn suffix(l: &str) -> crate::Result<&'static str> {
        match l {
            "bash" => Ok("sh"),
            "c" => Ok("c"),
            "cpp" => Ok("cpp"),
            "csharp" => Ok("cs"),
            "elixir" => Ok("ex"),
            "golang" => Ok("go"),
            "java" => Ok("java"),
            "javascript" => Ok("js"),
            "kotlin" => Ok("kt"),
            "mysql" => Ok("sql"),
            "php" => Ok("php"),
            "python" => Ok("py"),
            "python3" => Ok("py"),
            "ruby" => Ok("rb"),
            "rust" => Ok("rs"),
            "scala" => Ok("scala"),
            "swift" => Ok("swift"),
            "typescript" => Ok("ts"),
            _ => Ok("c"),
        }
    }

    use crate::{Error, cache::models::Problem};

    /// Generate test cases path by fid
    pub fn test_cases_path(problem: &Problem) -> crate::Result<String> {
        let conf = crate::config::Config::locate()?;
        // Use the basename of `pick` only — if the code template nests with `/`,
        // keep all *.tests.dat files flat under storage.tests.
        let pick_base = conf
            .code
            .pick
            .rsplit('/')
            .next()
            .unwrap_or(&conf.code.pick);
        let mut path = format!("{}/{}.tests.dat", conf.storage.tests()?, pick_base);

        path = path.replace("${fid}", &problem.fid.to_string());
        path = path.replace("${slug}", &problem.slug.to_string());
        Ok(path)
    }

    /// Generate code path by fid
    pub fn code_path(problem: &Problem, l: Option<String>) -> crate::Result<String> {
        let conf = crate::config::Config::locate()?;
        let mut lang = conf.code.lang;
        if l.is_some() {
            lang = l.ok_or(Error::NoneError)?;
        }

        let mut path = format!(
            "{}/{}.{}",
            conf.storage.code()?,
            conf.code.pick,
            suffix(&lang)?,
        );

        path = path.replace("${fid}", &problem.fid.to_string());
        path = path.replace("${slug}", &problem.slug.to_string());

        Ok(path)
    }

    /// Load python scripts
    pub fn load_script(module: &str) -> crate::Result<String> {
        use std::fs::File;
        use std::io::Read;
        let conf = crate::config::Config::locate()?;
        let mut script = "".to_string();
        File::open(format!("{}/{}.py", conf.storage.scripts()?, module))?
            .read_to_string(&mut script)?;

        Ok(script)
    }
}
