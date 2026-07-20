//! Solution command — list / read leetcode.cn solution articles.
use crate::{Error, Result, cache::Cache, plugins::LeetCode};
use clap::Args;
use colored::Colorize;
use serde_json::Value;
use std::io::{self, Write};

/// Solution command arguments
#[derive(Args)]
#[command(group = clap::ArgGroup::new("target").args(&["id", "url"]).required(true))]
pub struct SolutionArgs {
    /// Problem frontend id (e.g. 3)
    #[arg(value_parser = clap::value_parser!(i32))]
    pub id: Option<i32>,

    /// Select N-th solution from the list (1-based). If omitted, print list and prompt.
    #[arg(value_parser = clap::value_parser!(usize))]
    pub index: Option<usize>,

    /// Open a solution directly by URL or slug
    ///
    /// Examples:
    ///   leetcode so --url https://leetcode.cn/problems/.../solutions/1959540/xia-biao-.../
    ///   leetcode so --url xia-biao-zong-suan-cuo-qing-kan-zhe-by-e-iaks
    #[arg(long = "url", short = 'u', value_name = "URL_OR_SLUG")]
    pub url: Option<String>,

    /// How many solutions to list (default 15)
    #[arg(long, short = 'n', default_value_t = 15)]
    pub limit: i32,

    /// Skip language filter from `[code].lang` — show all languages
    #[arg(long = "all")]
    pub all_langs: bool,

    /// Override language tag filter (e.g. java / python3 / cpp)
    #[arg(long = "lang", short = 'l')]
    pub lang: Option<String>,

    /// Sort order: hot | default (default: hot)
    #[arg(long, short = 'o', default_value = "hot")]
    pub order: String,
}

impl SolutionArgs {
    /// `solution` handler
    pub async fn run(&self) -> Result<()> {
        // Direct URL / slug path — skip listing.
        if let Some(ref raw) = self.url {
            let slug = extract_solution_slug(raw).ok_or_else(|| {
                Error::Anyhow(anyhow::anyhow!(
                    "cannot parse solution slug from `{}`\n  try: leetcode so --url <slug-or-url>",
                    raw
                ))
            })?;
            return show_solution(&slug, preferred_lang(self)).await;
        }

        let id = self.id.ok_or(Error::NoneError)?;
        let cache = Cache::new()?;
        let problem = cache.get_problem(id)?;
        let conf = &cache.0.conf;

        let order = match self.order.to_ascii_lowercase().as_str() {
            "default" | "d" => "DEFAULT",
            _ => "HOT",
        };

        let tags = if self.all_langs {
            Vec::new()
        } else {
            let lang = self
                .lang
                .clone()
                .unwrap_or_else(|| conf.code.lang.clone());
            solution_lang_tags(&lang)
        };

        let lc = LeetCode::new()?;
        let resp = lc
            .list_solution_articles(&problem.slug, 0, self.limit.max(1), order, &tags)
            .await?;
        let json: Value = resp.json().await?;

        if let Some(errs) = json.get("errors") {
            return Err(Error::Anyhow(anyhow::anyhow!(
                "solution list API error: {}",
                errs
            )));
        }

        let root = json
            .pointer("/data/questionSolutionArticles")
            .cloned()
            .unwrap_or(Value::Null);
        let total = root
            .get("totalNum")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let edges = root
            .get("edges")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if edges.is_empty() {
            let filter_hint = if tags.is_empty() {
                String::new()
            } else {
                format!(" (lang filter: {})", tags.join(","))
            };
            println!(
                "no solutions found for #{} {}{}",
                id,
                problem.name,
                filter_hint
            );
            if !tags.is_empty() {
                println!(
                    "{} try `leetcode so {} --all` to drop language filter",
                    "hint:".dimmed(),
                    id
                );
            }
            return Ok(());
        }

        let items: Vec<SolutionItem> = edges
            .iter()
            .filter_map(|e| SolutionItem::from_node(e.get("node")?))
            .collect();

        let lang_label = if tags.is_empty() {
            "all".to_string()
        } else {
            tags.join(",")
        };
        println!(
            "\n{} #{} {}  {} {}  {} {}\n",
            "题解".bold(),
            id,
            problem.name.bold(),
            "lang:".dimmed(),
            lang_label.cyan(),
            "total:".dimmed(),
            total
        );

        for (i, it) in items.iter().enumerate() {
            let n = format!("{:>2}.", i + 1).yellow().bold();
            let author = it.author.green();
            let views = format_count(it.views);
            let votes = format_count(it.upvotes);
            let video = if it.has_video {
                " 🎬".to_string()
            } else {
                String::new()
            };
            let official = if it.by_leetcode {
                " [官方]".bright_blue().to_string()
            } else {
                String::new()
            };
            println!(
                "{} {}{}  {}{}",
                n,
                author,
                official,
                it.title,
                video
            );
            println!(
                "     {} {}  {} {}  {}",
                "👍".dimmed(),
                votes.dimmed(),
                "👁".dimmed(),
                views.dimmed(),
                it.slug.dimmed()
            );
        }

        // Resolve selection: explicit index, or interactive prompt.
        let pick = if let Some(idx) = self.index {
            idx
        } else {
            print!(
                "\n{} enter number 1-{} (q to quit): ",
                "select:".cyan().bold(),
                items.len()
            );
            let _ = io::stdout().flush();
            let mut buf = String::new();
            io::stdin().read_line(&mut buf)?;
            let s = buf.trim();
            if s.is_empty() || s.eq_ignore_ascii_case("q") || s.eq_ignore_ascii_case("quit") {
                return Ok(());
            }
            s.parse::<usize>().map_err(|_| {
                Error::Anyhow(anyhow::anyhow!("invalid selection `{}`", s))
            })?
        };

        if pick == 0 || pick > items.len() {
            return Err(Error::Anyhow(anyhow::anyhow!(
                "selection {} out of range 1-{}",
                pick,
                items.len()
            )));
        }

        let chosen = &items[pick - 1];
        show_solution(&chosen.slug, preferred_lang(self)).await
    }
}

struct SolutionItem {
    title: String,
    slug: String,
    author: String,
    views: i64,
    upvotes: i64,
    has_video: bool,
    by_leetcode: bool,
}

impl SolutionItem {
    fn from_node(n: &Value) -> Option<Self> {
        let title = n.get("title")?.as_str()?.to_string();
        let slug = n.get("slug")?.as_str()?.to_string();
        let author = n
            .pointer("/author/profile/realName")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .or_else(|| n.pointer("/author/username").and_then(|v| v.as_str()))
            .unwrap_or("?")
            .to_string();
        let views = n
            .pointer("/topic/viewCount")
            .and_then(|v| v.as_i64())
            .or_else(|| n.get("hitCount").and_then(|v| v.as_i64()))
            .unwrap_or(0);
        let upvotes = n
            .get("reactionsV2")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter(|r| r.get("reactionType").and_then(|t| t.as_str()) == Some("UPVOTE"))
                    .filter_map(|r| r.get("count").and_then(|c| c.as_i64()))
                    .sum()
            })
            .unwrap_or(0);
        let has_video = n.get("hasVideo").and_then(|v| v.as_bool()).unwrap_or(false);
        let by_leetcode = n
            .get("byLeetcode")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        Some(Self {
            title,
            slug,
            author,
            views,
            upvotes,
            has_video,
            by_leetcode,
        })
    }
}

async fn show_solution(slug: &str, prefer_lang: Option<String>) -> Result<()> {
    let lc = LeetCode::new()?;
    let resp = lc.get_solution_article(slug).await?;
    let json: Value = resp.json().await?;

    if let Some(errs) = json.get("errors") {
        return Err(Error::Anyhow(anyhow::anyhow!(
            "solution detail API error: {}",
            errs
        )));
    }

    let art = json
        .pointer("/data/solutionArticle")
        .cloned()
        .unwrap_or(Value::Null);
    if art.is_null() {
        return Err(Error::Anyhow(anyhow::anyhow!(
            "solution `{}` not found",
            slug
        )));
    }

    let title = art
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("(no title)");
    let author = art
        .pointer("/author/profile/realName")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| art.pointer("/author/username").and_then(|v| v.as_str()))
        .unwrap_or("?");
    let mut content = art
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let qslug = art
        .pointer("/question/questionTitleSlug")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let identifier = art
        .get("identifier")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Prefer configured language in multi-tab code blocks (` ```java [sol-Java] `).
    if let Some(ref lang) = prefer_lang {
        content = filter_multilang_fences(&content, lang);
    }

    println!(
        "\n{}  {}\n{} {}\n",
        title.bold().underline(),
        format!("by {}", author).green(),
        "slug:".dimmed(),
        slug.dimmed()
    );

    let rendered = crate::helper::render_markdown(&content);
    crate::helper::print_desc_with_images(&rendered.text, &rendered.images);

    let base = crate::config::Config::locate()
        .map(|c| c.sys.urls.base)
        .unwrap_or_else(|_| "https://leetcode.cn".into());
    let base = base.trim_end_matches('/');
    let link = if !qslug.is_empty() && !identifier.is_empty() {
        format!("{base}/problems/{qslug}/solutions/{identifier}/{slug}/")
    } else if !qslug.is_empty() {
        format!("{base}/problems/{qslug}/solutions/")
    } else {
        format!("{base}/")
    };
    println!("\n{} {}", "链接:".dimmed(), link);

    Ok(())
}

fn preferred_lang(args: &SolutionArgs) -> Option<String> {
    if args.all_langs {
        return None;
    }
    if let Some(ref l) = args.lang {
        return Some(l.clone());
    }
    crate::config::Config::locate()
        .ok()
        .map(|c| c.code.lang)
}

/// Map config `code.lang` → solution tag slugs used by leetcode.cn.
fn solution_lang_tags(lang: &str) -> Vec<String> {
    let l = lang.to_ascii_lowercase();
    match l.as_str() {
        // python submissions often tag either python or python3
        "python" | "python3" | "python2" => vec!["python3".into(), "python".into()],
        "golang" | "go" => vec!["golang".into()],
        "csharp" | "c#" => vec!["csharp".into()],
        "javascript" | "js" => vec!["javascript".into()],
        "typescript" | "ts" => vec!["typescript".into()],
        "cpp" | "c++" => vec!["cpp".into()],
        other => vec![other.to_string()],
    }
}

/// Pull solution slug from a full URL or bare slug.
///
/// Accepts:
/// - `https://leetcode.cn/problems/<q>/solutions/<id>/<slug>/`
/// - `https://leetcode.cn/problems/<q>/solution/<slug>/` (legacy)
/// - bare slug `xia-biao-zong-suan-cuo-qing-kan-zhe-by-e-iaks`
fn extract_solution_slug(input: &str) -> Option<String> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }

    // /solutions/<numeric-or-id>/<slug>
    if let Some(idx) = s.find("/solutions/") {
        let rest = &s[idx + "/solutions/".len()..];
        let parts: Vec<&str> = rest
            .split(|c| c == '/' || c == '?' || c == '#' || c == ' ')
            .filter(|p| !p.is_empty())
            .collect();
        // parts[0] may be numeric id, parts[1] is slug; or single slug
        if parts.len() >= 2 {
            return Some(parts[1].to_string());
        }
        if parts.len() == 1 && !parts[0].chars().all(|c| c.is_ascii_digit()) {
            return Some(parts[0].to_string());
        }
    }

    // legacy /solution/<slug>
    if let Some(idx) = s.find("/solution/") {
        let rest = &s[idx + "/solution/".len()..];
        let slug = rest
            .split(|c| c == '/' || c == '?' || c == '#' || c == ' ')
            .next()
            .unwrap_or("")
            .trim();
        if !slug.is_empty() {
            return Some(slug.to_string());
        }
    }

    // bare slug
    let bare = s
        .trim_matches(|c| c == '/' || c == '?' || c == '#')
        .split(|c| c == '/' || c == '?' || c == '#' || c == ' ')
        .next()
        .unwrap_or("")
        .trim();
    if !bare.is_empty()
        && bare
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && bare.contains('-')
    {
        return Some(bare.to_string());
    }
    None
}

fn format_count(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Keep only the preferred language from leetcode multi-tab code fences.
///
/// Multi-lang solutions mark tabs with headers like `java [sol-Java]` or
/// `Java [sol1-Java]`. Consecutive sol-fences form a group; only the preferred
/// language is kept. Plain fences without `[sol` are left untouched.
fn filter_multilang_fences(md: &str, prefer: &str) -> String {
    let prefer_aliases = lang_aliases(prefer);
    let fence = "```";
    let pat = format!(
        r"(?m)^{f}[^\n]*\r?\n[\s\S]*?^{f}\s*$",
        f = regex::escape(fence)
    );
    let fence_re = regex::Regex::new(&pat).expect("fence regex");

    let mut out = String::new();
    let mut last = 0usize;
    let fences: Vec<(usize, usize, String)> = fence_re
        .find_iter(md)
        .map(|m| (m.start(), m.end(), m.as_str().to_string()))
        .collect();

    let mut i = 0;
    while i < fences.len() {
        let (start, _, _) = fences[i];
        out.push_str(&md[last..start]);

        let header = fences[i].2.lines().next().unwrap_or("");
        // Official posts use `[sol1-Java]`; community posts use `[sol-Java]`.
        let is_sol = header.contains("[sol");
        if !is_sol {
            out.push_str(&fences[i].2);
            last = fences[i].1;
            i += 1;
            continue;
        }

        // Collect consecutive sol fences separated only by whitespace.
        let group_start = i;
        let mut j = i + 1;
        while j < fences.len() {
            let between = &md[fences[j - 1].1..fences[j].0];
            if !between.chars().all(|c| c.is_whitespace()) {
                break;
            }
            let h = fences[j].2.lines().next().unwrap_or("");
            if !h.contains("[sol") {
                break;
            }
            j += 1;
        }

        let group = &fences[group_start..j];
        let chosen = group
            .iter()
            .find(|(_, _, body)| {
                let h = body.lines().next().unwrap_or("");
                let lang = fence_lang(h);
                prefer_aliases.iter().any(|a| a == &lang)
            })
            .or_else(|| group.first());

        if let Some((_, _, body)) = chosen {
            // Drop the [sol-Label] suffix from the fence header.
            let mut lines = body.lines();
            if let Some(h) = lines.next() {
                let cleaned = h.split_whitespace().next().unwrap_or(fence);
                out.push_str(cleaned);
                out.push('\n');
                for line in lines {
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }

        last = fences[j - 1].1;
        i = j;
    }
    out.push_str(&md[last..]);
    out
}

/// Extract the language token from a fence header line.
///
/// Examples:
/// - "```java [sol-Java]" → "java"
/// - "```C++ [sol1-C++]"  → "c++"
/// - "```Python [sol1-Python3]" → "python"
fn fence_lang(header: &str) -> String {
    let h = header.trim();
    let rest = h.strip_prefix("```").unwrap_or(h);
    // first token before space / [sol...
    let token = rest
        .split(|c: char| c.is_whitespace() || c == '[')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    normalize_lang(&token)
}

/// Canonical language aliases for matching fence headers / config lang.
fn lang_aliases(lang: &str) -> Vec<String> {
    let n = normalize_lang(lang);
    match n.as_str() {
        "python" => vec!["python".into(), "python3".into(), "python2".into(), "py".into()],
        "java" => vec!["java".into()],
        "cpp" => vec!["cpp".into(), "c++".into(), "cplusplus".into()],
        "c" => vec!["c".into()],
        "golang" => vec!["golang".into(), "go".into()],
        "javascript" => vec!["javascript".into(), "js".into()],
        "typescript" => vec!["typescript".into(), "ts".into()],
        "rust" => vec!["rust".into(), "rs".into()],
        "csharp" => vec!["csharp".into(), "c#".into(), "cs".into()],
        "kotlin" => vec!["kotlin".into(), "kt".into()],
        "swift" => vec!["swift".into()],
        other => vec![other.to_string()],
    }
}

fn normalize_lang(lang: &str) -> String {
    match lang.to_ascii_lowercase().as_str() {
        "python3" | "python2" | "py" => "python".into(),
        "c++" | "cplusplus" => "cpp".into(),
        "go" => "golang".into(),
        "js" => "javascript".into(),
        "ts" => "typescript".into(),
        "c#" | "cs" => "csharp".into(),
        "kt" => "kotlin".into(),
        "rs" => "rust".into(),
        other => other.to_string(),
    }
}
