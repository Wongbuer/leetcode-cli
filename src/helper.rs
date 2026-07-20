//! A set of helper traits
pub use self::{
    digit::Digit,
    file::{code_path, load_script, test_cases_path},
    filter::{filter, squash},
    html::{HTML, RenderedDesc},
    image::{print_images, render_markdown, supports_inline_images},
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

    /// Kitty graphics protocol reliably decodes PNG (`f=100`). Convert other
    /// formats (JPEG/WebP/GIF) to PNG first so Ghostty/Kitty actually show the image.
    fn to_png_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
        // Already PNG — send as-is.
        if bytes.starts_with(&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']) {
            return Some(bytes.to_vec());
        }
        let img = image::load_from_memory(bytes).ok()?;
        let mut out = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut out);
        img.write_to(&mut cursor, image::ImageFormat::Png).ok()?;
        Some(out)
    }

    /// Emit one image via the Kitty graphics protocol.
    fn print_kitty_image(bytes: &[u8], id: u32) -> bool {
        let Some(png) = to_png_bytes(bytes) else {
            return false;
        };
        let mut out = std::io::stdout().lock();
        // f=100: payload is raw PNG file bytes; terminal decodes.
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
                    "\x1b_Ga=T,f=100,q=2,m={m},i={id};{piece}\x1b\\"
                );
            } else {
                let _ = write!(out, "\x1b_Gm={m};{piece}\x1b\\");
            }
        }
        let _ = writeln!(out);
        let _ = out.flush();
        true
    }

    /// Print images for a problem description.
    ///
    /// - Terminals with Kitty graphics support: inline true image + URL
    /// - Otherwise: URL only (no ASCII/block approximation)
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
                    if !print_kitty_image(&bytes, (n as u32) + 1000) {
                        // keep URL line already printed; silent if decode fails
                    }
                }
            }
        }
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
