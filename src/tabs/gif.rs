use std::collections::{HashMap, HashSet};

use iced::widget::{self, operation, scrollable};
use iced::Task as Command;
use serde::Deserialize;

use crate::app::{Message, Move, SPACING, SearchState};

const API_BASE: &str = "https://api.klipy.com/api/v1";
const GIF_COLUMNS: usize = 2;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GifEntry {
    pub id: String,
    pub url: String,
    pub gif_url: String,
    pub preview_url: String,
    pub slug: String,
    pub title: String,
}

pub struct GifTab {
    pub entries: Vec<GifEntry>,
    pub trending_cache: Option<Vec<GifEntry>>,
    pub frames: HashMap<usize, iced_gif::Frames>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub visible: HashSet<usize>,
    pub loading_frames: HashSet<usize>,
    pub scroll_id: widget::Id,
    pub saved_search: SearchState,
    pub loading: bool,
    pub error: Option<String>,
}

fn move_in(filtered: &[usize], selected: &mut usize, mv: Move) -> bool {
    if filtered.is_empty() {
        return false;
    }

    let pos = filtered.iter().position(|&i| i == *selected).unwrap_or(0);
    let col = pos % GIF_COLUMNS;
    let len = filtered.len();
    let next = match mv {
        Move::Up => pos.saturating_sub(GIF_COLUMNS),
        Move::Down => (pos + GIF_COLUMNS).min(len - 1),
        Move::Left => if col > 0 { pos - 1 } else { pos },
        Move::Right => if col + 1 < GIF_COLUMNS && pos + 1 < len { pos + 1 } else { pos },
    };

    let old_row = pos / GIF_COLUMNS;
    let new_row = next / GIF_COLUMNS;
    *selected = filtered[next];
    old_row != new_row
}

fn scroll_y(filtered: &[usize], selected: usize) -> f32 {
    let pos = filtered.iter().position(|&i| i == selected).unwrap_or(0);
    let row = pos / GIF_COLUMNS;
    let content_width = 500.0 - 42.0 - 24.0;
    let gaps = (GIF_COLUMNS - 1) as f32 * SPACING;
    let cell_width = (content_width - gaps) / GIF_COLUMNS as f32;
    let row_height = cell_width + SPACING;
    let y = row as f32 * row_height;
    (y - 120.0).max(0.0)
}

impl GifTab {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            trending_cache: None,
            frames: HashMap::new(),
            filtered: Vec::new(),
            selected: 0,
            visible: HashSet::new(),
            loading_frames: HashSet::new(),
            scroll_id: "gif-scroll".into(),
            saved_search: SearchState::new(),
            loading: true,
            error: None,
        }
    }

    pub fn set_entries(&mut self, entries: Vec<GifEntry>) {
        self.entries = entries;
        self.filtered = (0..self.entries.len()).collect();
        self.selected = self.filtered.first().copied().unwrap_or(0);
        self.loading = false;
        self.error = None;
        self.frames.clear();
        self.loading_frames.clear();
    }

    pub fn set_error(&mut self, msg: String) {
        self.entries.clear();
        self.filtered.clear();
        self.loading = false;
        self.error = Some(msg);
    }

    pub fn apply_search(&mut self, query: &str) -> bool {
        if query.trim().is_empty() {
            self.filtered = (0..self.entries.len()).collect();
            self.selected = self.filtered.first().copied().unwrap_or(0);
            return false;
        }
        // No local filtering — the API is the search engine.
        // Signal that we need an API search.
        true
    }

    pub fn move_selection(&mut self, mv: Move) -> bool {
        move_in(&self.filtered, &mut self.selected, mv)
    }

    pub fn scroll_to_selected(&self) -> Command<Message> {
        if self.filtered.is_empty() {
            return Command::none();
        }
        let y = scroll_y(&self.filtered, self.selected);
        operation::scroll_to(
            self.scroll_id.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y },
        )
    }

    pub fn copy_text(&self) -> Option<String> {
        let entry = self.entries.get(self.selected)?;
        if !self.filtered.contains(&self.selected) {
            return None;
        }
        Some(entry.gif_url.clone())
    }
}

// ── Minimal Klipy API response types ──

#[derive(Deserialize)]
struct KlipyResponse {
    #[serde(default)]
    result: bool,
    data: Option<KlipyData>,
}

#[derive(Deserialize)]
struct KlipyData {
    #[serde(default)]
    data: Vec<KlipyItem>,
}

#[derive(Deserialize)]
struct KlipyItem {
    #[serde(default)]
    id: serde_json::Value,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    title: String,
    // Ad placements have no `file`.
    file: Option<KlipyFile>,
}

#[derive(Deserialize)]
struct KlipyFile {
    hd: Option<KlipyFormats>,
    md: Option<KlipyFormats>,
    sm: Option<KlipyFormats>,
    xs: Option<KlipyFormats>,
}

#[derive(Deserialize)]
struct KlipyFormats {
    gif: Option<KlipyMedia>,
}

#[derive(Deserialize)]
struct KlipyMedia {
    url: String,
}

// ── HTTP helpers ──

fn api_key() -> Result<String, String> {
    crate::env::klipy_api_key()
        .filter(|k| !k.is_empty())
        .ok_or_else(|| "KLIPY_API_KEY not set (add it to .env before building, or export it)".to_string())
}

fn format_gif_url(f: &Option<KlipyFormats>) -> Option<String> {
    f.as_ref()?.gif.as_ref().map(|m| m.url.clone()).filter(|u| !u.is_empty())
}

async fn get(path: &str, params: &[(&str, &str)]) -> Result<Vec<GifEntry>, String> {
    let key = api_key()?;
    let url = reqwest::Url::parse_with_params(&format!("{API_BASE}/{key}/gifs{path}"), params)
        .map_err(|e| format!("invalid URL: {e}"))?;

    let resp = reqwest::get(url)
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Klipy API returned {status}: {body}"));
    }

    let body: KlipyResponse = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse failed: {e}"))?;

    if !body.result {
        return Err("Klipy API returned an unsuccessful result".to_string());
    }

    Ok(body
        .data
        .map(|d| d.data)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|g| {
            let file = g.file?;
            let gif_url = format_gif_url(&file.md).or_else(|| format_gif_url(&file.hd))?;
            let preview_url = format_gif_url(&file.sm)
                .or_else(|| format_gif_url(&file.xs))
                .unwrap_or_else(|| gif_url.clone());
            let id = match g.id {
                serde_json::Value::String(s) => s,
                v => v.to_string(),
            };
            Some(GifEntry {
                id,
                url: format!("https://klipy.com/gifs/{}", g.slug),
                gif_url,
                preview_url,
                slug: g.slug,
                title: g.title,
            })
        })
        .collect())
}

pub async fn download_image(url: String) -> Vec<u8> {
    let Ok(resp) = reqwest::get(&url).await else {
        return Vec::new();
    };
    resp.bytes().await.map(|b| b.to_vec()).unwrap_or_default()
}

pub async fn fetch_trending_gifs() -> Result<Vec<GifEntry>, String> {
    let result = get("/trending", &[("per_page", "50"), ("page", "1")]).await;
    if let Err(ref e) = result {
        tracing::error!("Klipy trending failed: {e}");
    }
    result
}

pub async fn search_gifs(query: String) -> Result<Vec<GifEntry>, String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return fetch_trending_gifs().await;
    }
    let result = get(
        "/search",
        &[
            ("q", &q),
            ("per_page", "50"),
            ("page", "1"),
            ("rating", "pg"),
            ("locale", "en"),
        ],
    )
    .await;
    if let Err(ref e) = result {
        tracing::error!("Klipy search failed: {e}");
    }
    result
}
