#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{
    self, pos2, vec2, Align, Align2, Button, CentralPanel, Color32, CornerRadius, FontId, Frame,
    Layout, Margin, RichText, ScrollArea, Sense, Stroke, TextEdit, Ui, Vec2,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Constants
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
const CODEX_DESIGN: &str = "codex-status";
const WEATHER_DESIGN: &str = "weather-static";
const DISCORD_DESIGN: &str = "discord-voice";
const BTC_DESIGN: &str = "btc-fees";
const MEDIA_DESIGN: &str = "media-player";

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsPage {
    Library,
    Rotation,
    Updates,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Color Palette — Premium Dark
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
fn bg() -> Color32 {
    Color32::from_rgb(11, 13, 16)
}
fn surface() -> Color32 {
    Color32::from_rgb(17, 20, 25)
}
fn surface_el() -> Color32 {
    Color32::from_rgb(24, 28, 34)
}
fn surface_3() -> Color32 {
    Color32::from_rgb(34, 39, 47)
}
fn accent() -> Color32 {
    Color32::from_rgb(108, 92, 231)
}
fn text_primary() -> Color32 {
    Color32::from_rgb(241, 245, 249)
}
fn text_muted() -> Color32 {
    Color32::from_rgb(100, 116, 139)
}
fn success() -> Color32 {
    Color32::from_rgb(16, 185, 129)
}
fn border_color() -> Color32 {
    Color32::from_rgb(30, 41, 59)
}
fn border_subtle() -> Color32 {
    Color32::from_rgb(24, 32, 45)
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t.clamp(0.0, 1.0)) as u8
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgb(lerp_u8(a.r(), b.r(), t), lerp_u8(a.g(), b.g(), t), lerp_u8(a.b(), b.b(), t))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Widget Definition
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[derive(Clone)]
struct WidgetDef {
    id: &'static str,
    title: &'static str,
    subtitle: &'static str,
    description: &'static str,
    accent: Color32,
}

fn widget_defs() -> [WidgetDef; 5] {
    [
        WidgetDef {
            id: CODEX_DESIGN,
            title: "Codex Status",
            subtitle: "API Quota Monitor",
            description: "Displays real-time Codex API quota and project status directly on \
                          your taskbar with a compact status capsule. Tracks active projects, \
                          usage metrics, and connection state at a glance.",
            accent: Color32::from_rgb(56, 189, 248),
        },
        WidgetDef {
            id: WEATHER_DESIGN,
            title: "Static Weather",
            subtitle: "Weather Capsule",
            description: "Shows current weather conditions and temperature for your selected \
                          city in a beautiful taskbar capsule. Displays location, condition \
                          summary, and temperature in your preferred unit.",
            accent: Color32::from_rgb(245, 158, 11),
        },
        WidgetDef {
            id: DISCORD_DESIGN,
            title: "Discord Voice",
            subtitle: "Live Voice Avatars",
            description: "Displays users in your selected Discord voice channel. Speaking users \
                          get a bright green frame while inactive users are dimmed, matching the \
                          voice activity feel from Discord.",
            accent: Color32::from_rgb(34, 197, 94),
        },
        WidgetDef {
            id: BTC_DESIGN,
            title: "Crypto Fees",
            subtitle: "ETH Fee Capsule",
            description: "A dark crypto-style taskbar card matching the supplied BTC/Fee visual. \
                          The current first version is a static design surface for the selected \
                          layout.",
            accent: Color32::from_rgb(198, 105, 255),
        },
        WidgetDef {
            id: MEDIA_DESIGN,
            title: "Media Player",
            subtitle: "Now Playing Control",
            description: "A compact audio widget matching the supplied sound visual. It can toggle \
                          the current Windows media session when a playable session is available.",
            accent: Color32::from_rgb(125, 200, 226),
        },
    ]
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Persistence — JSON Settings
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WidgetSettings {
    active_design: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    refresh_interval_secs: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    widget_offset_px: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rotation_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rotation_interval_secs: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rotation_designs: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    codex_api_endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    codex_project_filter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    weather_city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    weather_temp_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    discord_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    discord_background_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    media_dark_mode: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    discord_client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    discord_client_secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    discord_redirect_uri: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateStatus {
    state: Option<String>,
    current_version: Option<String>,
    latest_version: Option<String>,
    update_available: Option<bool>,
    message: Option<String>,
    updated_at_unix: Option<i64>,
}

fn read_settings(app_dir: &Path) -> WidgetSettings {
    let path = app_dir.join("widget-settings.json");
    fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_else(|| WidgetSettings {
            active_design: CODEX_DESIGN.to_owned(),
            ..Default::default()
        })
}

fn write_settings(app_dir: &Path, s: &WidgetSettings) -> std::io::Result<()> {
    let data = serde_json::to_string_pretty(s)
        .unwrap_or_else(|_| format!("{{\"activeDesign\":\"{}\"}}", CODEX_DESIGN));
    fs::write(app_dir.join("widget-settings.json"), format!("{data}\n"))
}

fn read_update_status(app_dir: &Path) -> UpdateStatus {
    fs::read_to_string(app_dir.join("update-status.json"))
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

fn run_loader_command(app_dir: &Path, arg: &str) -> std::io::Result<()> {
    let exe = app_dir.join("TaskbarStats.exe");
    Command::new(exe)
        .current_dir(app_dir)
        .arg(arg)
        .spawn()
        .map(|_| ())
}

fn normalize_design(id: &str) -> &str {
    match id {
        BTC_DESIGN => BTC_DESIGN,
        MEDIA_DESIGN => MEDIA_DESIGN,
        DISCORD_DESIGN => DISCORD_DESIGN,
        WEATHER_DESIGN => WEATHER_DESIGN,
        _ => CODEX_DESIGN,
    }
}

fn normalize_rotation_designs(saved: Option<Vec<String>>) -> Vec<String> {
    let mut designs = Vec::new();
    for id in saved.unwrap_or_else(|| {
        vec![
            CODEX_DESIGN.to_owned(),
            WEATHER_DESIGN.to_owned(),
            DISCORD_DESIGN.to_owned(),
            BTC_DESIGN.to_owned(),
            MEDIA_DESIGN.to_owned(),
        ]
    }) {
        let normalized = normalize_design(&id).to_owned();
        if !designs.iter().any(|existing| existing == &normalized) {
            designs.push(normalized);
        }
    }

    if designs.is_empty() {
        designs.push(CODEX_DESIGN.to_owned());
    }

    designs
}

fn widget_def_by_id(id: &str) -> WidgetDef {
    let defs = widget_defs();
    defs.iter()
        .find(|def| def.id == id)
        .cloned()
        .unwrap_or_else(|| defs[0].clone())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Application State
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
struct SettingsApp {
    app_dir: PathBuf,
    active_design: String,
    selected_idx: usize,
    selected_page: SettingsPage,
    status: String,
    dirty: bool,
    // General settings
    widget_enabled: bool,
    refresh_interval: String,
    widget_offset_px: u32,
    // Rotation settings
    rotation_enabled: bool,
    rotation_interval: String,
    rotation_designs: Vec<String>,
    // Codex-specific
    codex_endpoint: String,
    codex_filter: String,
    // Weather-specific
    weather_city: String,
    weather_unit_idx: usize,
    // Discord-specific
    discord_enabled: bool,
    discord_background_enabled: bool,
    // Media-specific
    media_dark_mode: bool,
    discord_client_id: String,
    discord_client_secret: String,
    discord_redirect_uri: String,
}

impl SettingsApp {
    fn load() -> Self {
        let app_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let s = read_settings(&app_dir);
        let design = normalize_design(&s.active_design).to_owned();
        let selected_idx = match design.as_str() {
            WEATHER_DESIGN => 1,
            DISCORD_DESIGN => 2,
            BTC_DESIGN => 3,
            MEDIA_DESIGN => 4,
            _ => 0,
        };
        let rotation_designs = normalize_rotation_designs(s.rotation_designs);

        Self {
            app_dir,
            active_design: design,
            selected_idx,
            selected_page: SettingsPage::Library,
            status: String::new(),
            dirty: false,
            widget_enabled: s.enabled.unwrap_or(true),
            refresh_interval: s.refresh_interval_secs.unwrap_or(30).to_string(),
            widget_offset_px: s.widget_offset_px.unwrap_or(0).min(480),
            rotation_enabled: s.rotation_enabled.unwrap_or(false),
            rotation_interval: s.rotation_interval_secs.unwrap_or(30).to_string(),
            rotation_designs,
            codex_endpoint: s.codex_api_endpoint.unwrap_or_default(),
            codex_filter: s.codex_project_filter.unwrap_or_default(),
            weather_city: s.weather_city.unwrap_or_else(|| "Istanbul".to_owned()),
            weather_unit_idx: if s.weather_temp_unit.as_deref() == Some("F") {
                1
            } else {
                0
            },
            discord_enabled: s.discord_enabled.unwrap_or(false),
            discord_background_enabled: s.discord_background_enabled.unwrap_or(true),
            media_dark_mode: s.media_dark_mode.unwrap_or(true),
            discord_client_id: s.discord_client_id.unwrap_or_default(),
            discord_client_secret: s.discord_client_secret.unwrap_or_default(),
            discord_redirect_uri: s
                .discord_redirect_uri
                .unwrap_or_else(|| "http://127.0.0.1/callback".to_owned()),
        }
    }

    fn save(&mut self) {
        let interval: u32 = self.refresh_interval.parse().unwrap_or(30);
        let rotation_interval: u32 = self.rotation_interval.parse().unwrap_or(30);
        let settings = WidgetSettings {
            active_design: self.active_design.clone(),
            enabled: Some(self.widget_enabled),
            refresh_interval_secs: Some(interval),
            widget_offset_px: Some(self.widget_offset_px.min(480)),
            rotation_enabled: Some(self.rotation_enabled),
            rotation_interval_secs: Some(rotation_interval.clamp(5, 3600)),
            rotation_designs: Some(normalize_rotation_designs(Some(self.rotation_designs.clone()))),
            codex_api_endpoint: if self.codex_endpoint.is_empty() {
                None
            } else {
                Some(self.codex_endpoint.clone())
            },
            codex_project_filter: if self.codex_filter.is_empty() {
                None
            } else {
                Some(self.codex_filter.clone())
            },
            weather_city: Some(self.weather_city.clone()),
            weather_temp_unit: Some(
                if self.weather_unit_idx == 1 { "F" } else { "C" }.to_owned(),
            ),
            discord_enabled: Some(self.discord_enabled),
            discord_background_enabled: Some(self.discord_background_enabled),
            media_dark_mode: Some(self.media_dark_mode),
            discord_client_id: if self.discord_client_id.is_empty() {
                None
            } else {
                Some(self.discord_client_id.clone())
            },
            discord_client_secret: if self.discord_client_secret.is_empty() {
                None
            } else {
                Some(self.discord_client_secret.clone())
            },
            discord_redirect_uri: if self.discord_redirect_uri.is_empty() {
                None
            } else {
                Some(self.discord_redirect_uri.clone())
            },
        };
        match write_settings(&self.app_dir, &settings) {
            Ok(()) => {
                self.status = "✓ Settings saved successfully".to_owned();
                self.dirty = false;
            }
            Err(e) => self.status = format!("✗ Save failed: {e}"),
        }
    }

    fn select_widget(&mut self, idx: usize) {
        self.selected_idx = idx;
        let defs = widget_defs();
        if let Some(def) = defs.get(idx) {
            self.active_design = def.id.to_owned();
            self.dirty = true;
            self.status = String::new();
        }
    }

    fn reset(&mut self) {
        let fresh = Self::load();
        self.active_design = fresh.active_design;
        self.selected_idx = fresh.selected_idx;
        self.selected_page = fresh.selected_page;
        self.widget_enabled = fresh.widget_enabled;
        self.refresh_interval = fresh.refresh_interval;
        self.widget_offset_px = fresh.widget_offset_px;
        self.rotation_enabled = fresh.rotation_enabled;
        self.rotation_interval = fresh.rotation_interval;
        self.rotation_designs = fresh.rotation_designs;
        self.codex_endpoint = fresh.codex_endpoint;
        self.codex_filter = fresh.codex_filter;
        self.weather_city = fresh.weather_city;
        self.weather_unit_idx = fresh.weather_unit_idx;
        self.discord_enabled = fresh.discord_enabled;
        self.discord_background_enabled = fresh.discord_background_enabled;
        self.media_dark_mode = fresh.media_dark_mode;
        self.discord_client_id = fresh.discord_client_id;
        self.discord_client_secret = fresh.discord_client_secret;
        self.discord_redirect_uri = fresh.discord_redirect_uri;
        self.dirty = false;
        self.status = "↺ Settings reset".to_owned();
    }

    fn check_updates(&mut self) {
        match run_loader_command(&self.app_dir, "--check-updates") {
            Ok(()) => self.status = "Checking for updates...".to_owned(),
            Err(e) => self.status = format!("✗ Update check failed to start: {e}"),
        }
    }

    fn install_update(&mut self) {
        match run_loader_command(&self.app_dir, "--update") {
            Ok(()) => self.status = "Downloading update...".to_owned(),
            Err(e) => self.status = format!("✗ Update install failed to start: {e}"),
        }
    }

    fn open_widget_libraries(&mut self) {
        let directory = self.app_dir.join("WidgetLibraries");
        if let Err(e) = fs::create_dir_all(&directory) {
            self.status = format!("Folder error: {e}");
            return;
        }
        let readme = directory.join("README.txt");
        if !readme.exists() {
            let _ = fs::write(&readme, "TaskbarStats widget design packs.\r\n");
        }
        match Command::new("explorer").arg(&directory).spawn() {
            Ok(_) => self.status = "WidgetLibraries opened".to_owned(),
            Err(e) => self.status = format!("Open failed: {e}"),
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Entry Point
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TaskbarStats Settings")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([760.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "TaskbarStats Settings",
        options,
        Box::new(|cc| {
            configure_style(&cc.egui_ctx);
            Ok(Box::new(SettingsApp::load()))
        }),
    )
}

fn configure_style(ctx: &egui::Context) {
    let mut s = (*ctx.style()).clone();
    s.visuals.window_fill = bg();
    s.visuals.panel_fill = bg();
    s.visuals.widgets.inactive.bg_fill = surface_el();
    s.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, border_subtle());
    s.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, text_primary());
    s.visuals.widgets.hovered.bg_fill = surface_3();
    s.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(45, 55, 70));
    s.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, text_primary());
    s.visuals.widgets.active.bg_fill = surface_3();
    s.visuals.widgets.active.bg_stroke = Stroke::new(1.0, accent());
    s.visuals.widgets.active.fg_stroke = Stroke::new(1.0, text_primary());
    s.visuals.selection.bg_fill = accent();
    s.spacing.item_spacing = Vec2::new(8.0, 8.0);
    ctx.set_style(s);
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Thin accent bar at the very top
        egui::TopBottomPanel::top("accent_strip")
            .exact_height(2.0)
            .frame(Frame::new().fill(accent()).inner_margin(Margin::same(0)))
            .show(ctx, |_ui| {});

        let app_width = ctx.available_rect().width();
        let panel_width = if app_width < 860.0 {
            230.0
        } else if app_width < 1040.0 {
            270.0
        } else {
            310.0
        };

        // Left panel — Widget gallery
        egui::SidePanel::left("gallery_panel")
            .exact_width(panel_width)
            .resizable(false)
            .frame(Frame::new().fill(surface()).inner_margin(Margin::same(0)))
            .show(ctx, |ui| {
                draw_left_panel(ui, self);
            });

        // Central — Detail & settings
        CentralPanel::default()
            .frame(Frame::new().fill(bg()))
            .show(ctx, |ui| {
                draw_right_panel(ui, self);
            });
    }
}

fn content_margin(ui: &Ui) -> Margin {
    let w = ui.available_width();
    if w < 560.0 {
        Margin::symmetric(18, 20)
    } else if w < 760.0 {
        Margin::symmetric(26, 26)
    } else {
        Margin::symmetric(40, 32)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Left Panel — Widget Gallery
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
fn draw_left_panel(ui: &mut Ui, app: &mut SettingsApp) {
    ui.set_min_height(ui.available_height());
    let compact = ui.available_width() < 260.0;
    let header_margin = if compact {
        Margin::symmetric(14, 18)
    } else {
        Margin::symmetric(24, 24)
    };
    let nav_margin = if compact {
        Margin::symmetric(10, 12)
    } else {
        Margin::symmetric(14, 14)
    };
    let side_margin = if compact {
        Margin::symmetric(12, 0)
    } else {
        Margin::symmetric(14, 0)
    };

    // ── Header ──
    Frame::new()
        .fill(surface())
        .inner_margin(header_margin)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Logo circle
                let (logo_rect, _) = ui.allocate_exact_size(vec2(38.0, 38.0), Sense::hover());
                let p = ui.painter();
                p.circle_filled(logo_rect.center(), 19.0, accent());
                p.circle_filled(
                    logo_rect.center(),
                    15.0,
                    Color32::from_rgb(88, 72, 211),
                );
                p.text(
                    logo_rect.center(),
                    Align2::CENTER_CENTER,
                    "T",
                    FontId::proportional(17.0),
                    Color32::WHITE,
                );

                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        RichText::new("TaskbarStats")
                            .color(text_primary())
                            .font(FontId::proportional(20.0)),
                    );
                    ui.label(RichText::new("Widget Studio").color(text_muted()).size(12.0));
                });
            });
        });

    // Separator
    let (sep, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(sep, CornerRadius::ZERO, border_color());

    // ── Navigation ──
    Frame::new()
        .fill(surface())
        .inner_margin(nav_margin)
        .show(ui, |ui| {
            if draw_nav_item(
                ui,
                "Widget Library",
                "Choose active design",
                app.selected_page == SettingsPage::Library,
                Color32::from_rgb(56, 189, 248),
            ) {
                app.selected_page = SettingsPage::Library;
            }
            ui.add_space(6.0);
            if draw_nav_item(
                ui,
                "Slider Rotation",
                "Order and timing",
                app.selected_page == SettingsPage::Rotation,
                Color32::from_rgb(108, 92, 231),
            ) {
                app.selected_page = SettingsPage::Rotation;
            }
            ui.add_space(6.0);
            if draw_nav_item(
                ui,
                "Updates",
                "Check and install",
                app.selected_page == SettingsPage::Updates,
                Color32::from_rgb(34, 197, 94),
            ) {
                app.selected_page = SettingsPage::Updates;
            }
        });

    // ── Section label ──
    Frame::new()
        .fill(surface())
        .inner_margin(Margin::symmetric(24, 6))
        .show(ui, |ui| {
            ui.add_space(6.0);
            ui.label(
                RichText::new("WIDGETS")
                    .color(Color32::from_rgb(70, 82, 100))
                    .size(11.0),
            );
        });

    // ── Widget cards ──
    let defs = widget_defs();
    Frame::new()
        .fill(surface())
        .inner_margin(side_margin)
        .show(ui, |ui| {
            for (i, def) in defs.iter().enumerate() {
                draw_gallery_card(ui, app, i, def);
                if i < defs.len() - 1 {
                    ui.add_space(4.0);
                }
            }
        });

    // ── Bottom area ──
    ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
        Frame::new()
            .fill(surface())
            .inner_margin(Margin::symmetric(20, 18))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("TaskbarStats v0.1.0")
                        .color(Color32::from_rgb(42, 50, 62))
                        .size(10.0),
                );
            });
    });
}

fn draw_gallery_card(ui: &mut Ui, app: &mut SettingsApp, idx: usize, def: &WidgetDef) {
    let selected = app.selected_idx == idx;
    let is_active = app.active_design == def.id;
    let h = 80.0;
    let w = ui.available_width();

    let (rect, response) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    let p = ui.painter_at(rect);
    let hovered = response.hovered();

    // Animated hover/select transition
    let hover_t = ui.ctx().animate_bool(response.id, hovered || selected);

    // Background
    let fill = lerp_color(Color32::from_rgb(17, 20, 25), Color32::from_rgb(22, 26, 33), hover_t);
    p.rect_filled(rect, CornerRadius::same(12), fill);

    // Border — accent glow when selected
    if selected {
        p.rect_stroke(
            rect,
            CornerRadius::same(12),
            Stroke::new(1.5, def.accent),
            egui::StrokeKind::Inside,
        );
        // Subtle outer glow (simulated with a slightly larger faded rect)
        let glow = rect.expand(2.0);
        p.rect_stroke(
            glow,
            CornerRadius::same(14),
            Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(def.accent.r(), def.accent.g(), def.accent.b(), 30),
            ),
            egui::StrokeKind::Outside,
        );
    } else if hovered {
        p.rect_stroke(
            rect,
            CornerRadius::same(12),
            Stroke::new(1.0, Color32::from_rgb(38, 46, 58)),
            egui::StrokeKind::Inside,
        );
    }

    // ── Icon area ──
    let icon_size = 46.0;
    let icon_rect = egui::Rect::from_min_size(
        rect.left_top() + vec2(14.0, (h - icon_size) / 2.0),
        vec2(icon_size, icon_size),
    );
    p.rect_filled(
        icon_rect,
        CornerRadius::same(11),
        Color32::from_rgba_unmultiplied(def.accent.r(), def.accent.g(), def.accent.b(), 18),
    );
    draw_mini_icon(&p, icon_rect, def);

    // ── Text ──
    let tx = icon_rect.right() + 12.0;
    p.text(
        pos2(tx, rect.top() + 19.0),
        Align2::LEFT_TOP,
        def.title,
        FontId::proportional(14.5),
        if selected {
            text_primary()
        } else {
            Color32::from_rgb(190, 200, 215)
        },
    );
    p.text(
        pos2(tx, rect.top() + 39.0),
        Align2::LEFT_TOP,
        def.subtitle,
        FontId::proportional(11.0),
        text_muted(),
    );

    // ── Active badge ──
    if is_active {
        let bw = 54.0;
        let bh = 22.0;
        let badge = egui::Rect::from_min_size(
            pos2(rect.right() - bw - 12.0, rect.center().y - bh / 2.0),
            vec2(bw, bh),
        );
        p.rect_filled(
            badge,
            CornerRadius::same(6),
            Color32::from_rgba_unmultiplied(16, 185, 129, 22),
        );
        p.rect_stroke(
            badge,
            CornerRadius::same(6),
            Stroke::new(
                0.8,
                Color32::from_rgba_unmultiplied(16, 185, 129, 60),
            ),
            egui::StrokeKind::Inside,
        );
        p.text(
            badge.center(),
            Align2::CENTER_CENTER,
            "Active",
            FontId::proportional(10.0),
            success(),
        );
    }

    if response.clicked() {
        app.selected_page = SettingsPage::Library;
        app.select_widget(idx);
    }
}

fn draw_nav_item(
    ui: &mut Ui,
    title: &str,
    subtitle: &str,
    selected: bool,
    color: Color32,
) -> bool {
    let w = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(w, 58.0), Sense::click());
    let p = ui.painter_at(rect);
    let hover_t = ui.ctx().animate_bool(response.id, selected || response.hovered());
    let fill = lerp_color(surface(), Color32::from_rgb(23, 28, 36), hover_t);

    p.rect_filled(rect, CornerRadius::same(12), fill);
    if selected {
        p.rect_stroke(
            rect,
            CornerRadius::same(12),
            Stroke::new(1.4, color),
            egui::StrokeKind::Inside,
        );
    }

    let dot = egui::Rect::from_min_size(rect.left_top() + vec2(14.0, 20.0), vec2(10.0, 10.0));
    p.circle_filled(dot.center(), 5.0, color);
    p.text(
        rect.left_top() + vec2(34.0, 12.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(13.0),
        text_primary(),
    );
    p.text(
        rect.left_top() + vec2(34.0, 31.0),
        Align2::LEFT_TOP,
        subtitle,
        FontId::proportional(11.0),
        text_muted(),
    );

    response.clicked()
}

fn draw_mini_icon(p: &egui::Painter, rect: egui::Rect, def: &WidgetDef) {
    let cx = rect.center().x;
    let cy = rect.center().y;

    match def.id {
        CODEX_DESIGN => {
            // Bar chart icon
            let bar_w = 5.0;
            let gap = 3.0;
            let heights = [12.0, 18.0, 9.0, 15.0];
            let total_w = 4.0 * bar_w + 3.0 * gap;
            let sx = cx - total_w / 2.0;
            let base = cy + 11.0;
            for (i, &h) in heights.iter().enumerate() {
                let x = sx + i as f32 * (bar_w + gap);
                p.rect_filled(
                    egui::Rect::from_min_size(pos2(x, base - h), vec2(bar_w, h)),
                    CornerRadius::same(2),
                    Color32::from_rgba_unmultiplied(
                        def.accent.r(),
                        def.accent.g(),
                        def.accent.b(),
                        if i == 1 { 210 } else { 120 },
                    ),
                );
            }
        }
        WEATHER_DESIGN => {
            // Sun with rays
            p.circle_filled(pos2(cx, cy), 8.0, def.accent);
            for i in 0..8 {
                let a = std::f32::consts::TAU * i as f32 / 8.0;
                let (sin, cos) = a.sin_cos();
                p.line_segment(
                    [
                        pos2(cx + cos * 11.0, cy + sin * 11.0),
                        pos2(cx + cos * 14.5, cy + sin * 14.5),
                    ],
                    Stroke::new(
                        1.5,
                        Color32::from_rgba_unmultiplied(
                            def.accent.r(),
                            def.accent.g(),
                            def.accent.b(),
                            140,
                        ),
                    ),
                );
            }
        }
        DISCORD_DESIGN => {
            for i in 0..3 {
                let x = cx - 18.0 + i as f32 * 18.0;
                let active = i == 1;
                p.circle_filled(
                    pos2(x, cy),
                    if active { 11.0 } else { 9.0 },
                    if active {
                        Color32::from_rgb(88, 101, 242)
                    } else {
                        Color32::from_rgb(55, 65, 81)
                    },
                );
                if active {
                    p.circle_stroke(pos2(x, cy), 12.5, Stroke::new(2.0, def.accent));
                }
            }
        }
        BTC_DESIGN => {
            p.text(
                pos2(cx - 11.0, cy - 7.0),
                Align2::CENTER_CENTER,
                "↗",
                FontId::proportional(16.0),
                Color32::from_rgb(23, 255, 207),
            );
            p.text(
                pos2(cx + 11.0, cy + 7.0),
                Align2::CENTER_CENTER,
                "◇",
                FontId::proportional(17.0),
                def.accent,
            );
        }
        MEDIA_DESIGN => {
            p.line_segment(
                [pos2(cx - 13.0, cy - 10.0), pos2(cx - 13.0, cy + 10.0)],
                Stroke::new(2.0, text_primary()),
            );
            p.line_segment(
                [pos2(cx - 13.0, cy - 10.0), pos2(cx + 8.0, cy)],
                Stroke::new(2.0, text_primary()),
            );
            p.line_segment(
                [pos2(cx - 13.0, cy + 10.0), pos2(cx + 8.0, cy)],
                Stroke::new(2.0, text_primary()),
            );
            for i in 0..7 {
                let h = 5.0 + ((i * 7) % 13) as f32;
                let x = cx + 20.0 + i as f32 * 4.0;
                p.line_segment(
                    [pos2(x, cy - h / 2.0), pos2(x, cy + h / 2.0)],
                    Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 150)),
                );
            }
        }
        _ => {}
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Right Panel — Detail & Settings
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
fn draw_right_panel(ui: &mut Ui, app: &mut SettingsApp) {
    if app.selected_page == SettingsPage::Rotation {
        draw_rotation_page(ui, app);
        return;
    }
    if app.selected_page == SettingsPage::Updates {
        draw_updates_page(ui, app);
        return;
    }

    let defs = widget_defs();
    let def = defs
        .get(app.selected_idx)
        .cloned()
        .unwrap_or_else(|| defs[0].clone());

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::new()
                .fill(bg())
                .inner_margin(content_margin(ui))
                .show(ui, |ui| {
                    // Preview
                    draw_preview_section(ui, &def);
                    ui.add_space(26.0);

                    // Widget info
                    draw_info_section(ui, &def);
                    ui.add_space(22.0);

                    // General settings
                    draw_section_header(ui, "General Settings", accent());
                    ui.add_space(10.0);
                    draw_general_settings(ui, app);
                    ui.add_space(22.0);

                    // Widget-specific settings
                    let title = format!("{} Settings", def.title);
                    draw_section_header(ui, &title, def.accent);
                    ui.add_space(10.0);
                    draw_widget_settings(ui, app, &def);
                    ui.add_space(28.0);

                    // Actions
                    draw_action_buttons(ui, app);

                    // Status
                    if !app.status.is_empty() {
                        ui.add_space(12.0);
                        let color = if app.status.starts_with('✗') {
                            Color32::from_rgb(239, 68, 68)
                        } else if app.status.starts_with('✓') {
                            success()
                        } else {
                            text_muted()
                        };
                        ui.label(RichText::new(&app.status).color(color).size(12.0));
                    }
                });
        });
}

fn draw_rotation_page(ui: &mut Ui, app: &mut SettingsApp) {
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::new()
                .fill(bg())
                .inner_margin(content_margin(ui))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Slider Rotation")
                            .color(text_primary())
                            .font(FontId::proportional(30.0)),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(
                            "Select which taskbar widgets rotate, set their order, and choose how often the slider advances.",
                        )
                        .color(text_muted())
                        .size(13.0),
                    );
                    ui.add_space(24.0);

                    draw_section_header(ui, "Rotation Controls", accent());
                    ui.add_space(10.0);
                    draw_rotation_settings(ui, app);
                    ui.add_space(28.0);

                    draw_action_buttons(ui, app);

                    if !app.status.is_empty() {
                        ui.add_space(12.0);
                        let color = if app.status.starts_with('✗') {
                            Color32::from_rgb(239, 68, 68)
                        } else if app.status.starts_with('✓') {
                            success()
                        } else {
                            text_muted()
                        };
                        ui.label(RichText::new(&app.status).color(color).size(12.0));
                    }
                });
        });
}

fn draw_updates_page(ui: &mut Ui, app: &mut SettingsApp) {
    let update = read_update_status(&app.app_dir);
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::new()
                .fill(bg())
                .inner_margin(content_margin(ui))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Updates")
                            .color(text_primary())
                            .font(FontId::proportional(30.0)),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Check GitHub releases and install the latest TaskbarStats build.")
                            .color(text_muted())
                            .size(13.0),
                    );
                    ui.add_space(24.0);

                    draw_section_header(ui, "Release Status", success());
                    ui.add_space(10.0);
                    settings_card(ui, |ui| {
                        update_status_row(
                            ui,
                            "Current version",
                            update.current_version.as_deref().unwrap_or("0.1.0"),
                        );
                        card_separator(ui);
                        update_status_row(
                            ui,
                            "Latest release",
                            update.latest_version.as_deref().filter(|v| !v.is_empty()).unwrap_or("Not checked"),
                        );
                        card_separator(ui);
                        update_status_row(
                            ui,
                            "State",
                            update.state.as_deref().unwrap_or("idle"),
                        );
                        card_separator(ui);
                        let message = update
                            .message
                            .as_deref()
                            .filter(|v| !v.is_empty())
                            .unwrap_or("Run a check to refresh update status.");
                        ui.label(RichText::new(message).color(text_primary()).size(13.0));
                        if let Some(ts) = update.updated_at_unix {
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(format!("Updated at Unix time {ts}"))
                                    .color(text_muted())
                                    .size(11.0),
                            );
                        }
                    });
                    ui.add_space(18.0);

                    ui.horizontal_wrapped(|ui| {
                        let check = ui.add_sized(
                            [150.0, 40.0],
                            Button::new(
                                RichText::new("Check Updates")
                                    .color(Color32::WHITE)
                                    .size(13.0),
                            )
                            .fill(accent())
                            .stroke(Stroke::new(1.0, Color32::from_rgb(128, 112, 241)))
                            .corner_radius(CornerRadius::same(10)),
                        );
                        if check.clicked() {
                            app.check_updates();
                        }

                        let can_install = update.update_available.unwrap_or(false);
                        let install_fill = if can_install { success() } else { surface_el() };
                        let install_text = if can_install {
                            Color32::WHITE
                        } else {
                            text_muted()
                        };
                        let install = ui.add_sized(
                            [150.0, 40.0],
                            Button::new(
                                RichText::new("Install Update")
                                    .color(install_text)
                                    .size(13.0),
                            )
                            .fill(install_fill)
                            .stroke(Stroke::new(1.0, border_color()))
                            .corner_radius(CornerRadius::same(10)),
                        );
                        if install.clicked() {
                            app.install_update();
                        }
                    });

                    if !app.status.is_empty() {
                        ui.add_space(12.0);
                        let color = if app.status.starts_with('✗') {
                            Color32::from_rgb(239, 68, 68)
                        } else {
                            text_muted()
                        };
                        ui.label(RichText::new(&app.status).color(color).size(12.0));
                    }
                });
        });
}

fn update_status_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(label).color(text_muted()).size(12.0));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).color(text_primary()).size(13.0));
        });
    });
}

// ── Preview Section ─────────────────────────────────────────────────────────
fn draw_preview_section(ui: &mut Ui, def: &WidgetDef) {
    let pw = ui.available_width().min(700.0);
    let ph = 130.0;

    let (rect, _) = ui.allocate_exact_size(vec2(pw, ph), Sense::hover());
    let p = ui.painter_at(rect);

    // Dark container
    p.rect_filled(rect, CornerRadius::same(14), Color32::from_rgb(10, 12, 15));
    p.rect_stroke(
        rect,
        CornerRadius::same(14),
        Stroke::new(1.0, Color32::from_rgb(26, 32, 40)),
        egui::StrokeKind::Inside,
    );

    // Taskbar strip
    let tb_h = 46.0;
    let tb = egui::Rect::from_min_size(
        pos2(rect.left() + 24.0, rect.center().y - tb_h / 2.0),
        vec2(rect.width() - 48.0, tb_h),
    );
    p.rect_filled(tb, CornerRadius::same(8), Color32::from_rgb(22, 25, 30));
    p.rect_stroke(
        tb,
        CornerRadius::same(8),
        Stroke::new(0.6, Color32::from_rgb(34, 40, 48)),
        egui::StrokeKind::Inside,
    );

    // Simulated taskbar icons (left side)
    let iy = tb.center().y;
    for i in 0..5 {
        let x = tb.left() + 20.0 + i as f32 * 22.0;
        p.rect_filled(
            egui::Rect::from_center_size(pos2(x, iy), vec2(10.0, 10.0)),
            CornerRadius::same(3),
            Color32::from_rgb(35 + i * 4, 40 + i * 3, 50 + i * 2),
        );
    }

    // Widget capsule (right-center area of taskbar)
    let ww = 210.0;
    let wh = 36.0;
    let wr = egui::Rect::from_min_size(
        pos2(tb.right() - ww - 100.0, tb.center().y - wh / 2.0),
        vec2(ww, wh),
    );
    p.rect_filled(wr, CornerRadius::same(7), Color32::from_rgb(28, 32, 38));
    p.rect_stroke(
        wr,
        CornerRadius::same(7),
        Stroke::new(
            0.6,
            Color32::from_rgba_unmultiplied(def.accent.r(), def.accent.g(), def.accent.b(), 40),
        ),
        egui::StrokeKind::Inside,
    );

    match def.id {
        CODEX_DESIGN => draw_codex_preview(&p, wr, def.accent),
        WEATHER_DESIGN => draw_weather_preview(&p, wr, def.accent),
        DISCORD_DESIGN => draw_discord_preview(&p, wr, def.accent),
        BTC_DESIGN => draw_btc_preview(&p, wr),
        MEDIA_DESIGN => draw_media_preview(&p, wr),
        _ => {}
    }

    // System tray icons (right side)
    for i in 0..3 {
        let x = tb.right() - 18.0 - i as f32 * 16.0;
        p.circle_filled(pos2(x, iy), 3.5, Color32::from_rgb(48, 55, 65));
    }
    // Clock
    p.text(
        pos2(tb.right() - 62.0, iy),
        Align2::LEFT_CENTER,
        "19:54",
        FontId::proportional(10.0),
        Color32::from_rgb(100, 112, 128),
    );

    // Label
    p.text(
        pos2(rect.left() + 14.0, rect.bottom() - 12.0),
        Align2::LEFT_BOTTOM,
        "TASKBAR PREVIEW",
        FontId::proportional(9.0),
        Color32::from_rgb(42, 50, 62),
    );
}

fn draw_codex_preview(p: &egui::Painter, r: egui::Rect, accent: Color32) {
    // Title row
    p.text(
        pos2(r.left() + 12.0, r.top() + 5.0),
        Align2::LEFT_TOP,
        "Antigravity",
        FontId::proportional(11.0),
        Color32::from_rgb(235, 240, 245),
    );
    // State dot + label
    p.circle_filled(pos2(r.right() - 50.0, r.top() + 11.0), 3.0, success());
    p.text(
        pos2(r.right() - 44.0, r.top() + 5.5),
        Align2::LEFT_TOP,
        "RUN",
        FontId::proportional(10.0),
        accent,
    );
    // Divider
    p.line_segment(
        [
            pos2(r.left() + 12.0, r.center().y + 1.0),
            pos2(r.right() - 12.0, r.center().y + 1.0),
        ],
        Stroke::new(0.5, Color32::from_rgb(44, 50, 58)),
    );
    // Metrics row
    let my = r.bottom() - 11.0;
    p.text(
        pos2(r.left() + 12.0, my),
        Align2::LEFT_CENTER,
        "CPU 23%",
        FontId::proportional(9.0),
        Color32::from_rgb(160, 172, 185),
    );
    p.text(
        pos2(r.left() + 72.0, my),
        Align2::LEFT_CENTER,
        "RAM 67%",
        FontId::proportional(9.0),
        Color32::from_rgb(160, 172, 185),
    );
    // Progress bar
    let bw = 55.0;
    let bb = egui::Rect::from_min_size(pos2(r.right() - bw - 12.0, my - 1.5), vec2(bw, 3.0));
    p.rect_filled(bb, CornerRadius::same(2), Color32::from_rgb(40, 46, 54));
    p.rect_filled(
        egui::Rect::from_min_size(bb.left_top(), vec2(bw * 0.67, 3.0)),
        CornerRadius::same(2),
        accent,
    );
}

fn draw_weather_preview(p: &egui::Painter, r: egui::Rect, accent: Color32) {
    // Sun
    let sx = r.left() + 22.0;
    let sy = r.center().y;
    p.circle_filled(pos2(sx, sy), 7.0, accent);
    for i in 0..8 {
        let a = std::f32::consts::TAU * i as f32 / 8.0;
        let (sin, cos) = a.sin_cos();
        p.line_segment(
            [
                pos2(sx + cos * 9.5, sy + sin * 9.5),
                pos2(sx + cos * 12.5, sy + sin * 12.5),
            ],
            Stroke::new(
                1.2,
                Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 150),
            ),
        );
    }
    // City + condition
    p.text(
        pos2(r.left() + 40.0, r.top() + 6.0),
        Align2::LEFT_TOP,
        "Istanbul",
        FontId::proportional(12.0),
        text_primary(),
    );
    p.text(
        pos2(r.left() + 40.0, r.top() + 21.0),
        Align2::LEFT_TOP,
        "Partly Cloudy",
        FontId::proportional(9.0),
        text_muted(),
    );
    // Temperature
    p.text(
        pos2(r.right() - 14.0, r.center().y),
        Align2::RIGHT_CENTER,
        "24°C",
        FontId::proportional(16.0),
        text_primary(),
    );
}

fn draw_discord_preview(p: &egui::Painter, r: egui::Rect, accent: Color32) {
    let names = ["A", "B", "C", "D"];
    for (i, label) in names.iter().enumerate() {
        let x = r.left() + 28.0 + i as f32 * 38.0;
        let c = pos2(x, r.center().y);
        let speaking = i == 1;
        let fill = if speaking {
            Color32::from_rgb(88, 101, 242)
        } else {
            Color32::from_rgba_unmultiplied(88, 101, 242, 95)
        };
        p.circle_filled(c, 13.0, fill);
        if speaking {
            p.circle_stroke(c, 15.5, Stroke::new(2.2, accent));
        }
        p.text(
            c,
            Align2::CENTER_CENTER,
            *label,
            FontId::proportional(11.0),
            Color32::WHITE,
        );
    }

    p.text(
        pos2(r.right() - 16.0, r.center().y),
        Align2::RIGHT_CENTER,
        "Voice",
        FontId::proportional(11.0),
        text_muted(),
    );
}

fn draw_btc_preview(p: &egui::Painter, r: egui::Rect) {
    p.rect_filled(r, CornerRadius::same(18), Color32::from_rgb(18, 8, 24));
    p.text(
        pos2(r.left() + 18.0, r.top() + 7.0),
        Align2::LEFT_TOP,
        "Current date",
        FontId::proportional(12.0),
        Color32::from_rgb(207, 190, 212),
    );
    p.text(
        pos2(r.right() - 18.0, r.top() + 7.0),
        Align2::RIGHT_TOP,
        "January 22, 2022 - 7:23 AM",
        FontId::proportional(12.0),
        text_primary(),
    );
    p.text(
        pos2(r.left() + 18.0, r.bottom() - 12.0),
        Align2::LEFT_CENTER,
        "Fees  ↗",
        FontId::proportional(12.0),
        Color32::from_rgb(207, 190, 212),
    );
    p.text(
        pos2(r.right() - 34.0, r.bottom() - 12.0),
        Align2::RIGHT_CENTER,
        "0.00004353 ETH",
        FontId::proportional(13.0),
        text_primary(),
    );
    p.text(
        pos2(r.right() - 16.0, r.bottom() - 12.0),
        Align2::CENTER_CENTER,
        "◇",
        FontId::proportional(16.0),
        Color32::from_rgb(198, 105, 255),
    );
}

fn draw_media_preview(p: &egui::Painter, r: egui::Rect) {
    p.rect_filled(r, CornerRadius::same(18), Color32::from_rgb(247, 251, 252));
    let tint = egui::Rect::from_min_max(r.left_top(), pos2(r.left() + r.width() * 0.61, r.bottom()));
    p.rect_filled(tint, CornerRadius::same(18), Color32::from_rgb(217, 246, 250));

    let cover = egui::Rect::from_min_size(pos2(r.left() + 11.0, r.top() + 7.0), vec2(47.0, 24.0));
    p.rect_filled(cover, CornerRadius::same(4), Color32::from_rgb(24, 28, 36));
    p.rect_filled(
        egui::Rect::from_min_size(cover.left_top(), vec2(18.0, cover.height())),
        CornerRadius::same(4),
        Color32::from_rgb(245, 118, 38),
    );
    p.circle_filled(pos2(cover.left() + 31.0, cover.center().y), 8.0, Color32::from_rgb(250, 160, 64));

    p.text(
        pos2(r.left() + 70.0, r.top() + 6.0),
        Align2::LEFT_TOP,
        "Visions - Purple Disco",
        FontId::proportional(12.0),
        Color32::BLACK,
    );
    p.text(
        pos2(r.left() + 70.0, r.top() + 21.0),
        Align2::LEFT_TOP,
        "Eli Escobar, Dana Weaver",
        FontId::proportional(9.0),
        Color32::from_rgb(10, 10, 10),
    );

    let cx = r.right() - 20.0;
    let cy = r.center().y;
    p.circle_filled(pos2(cx, cy), 11.0, Color32::BLACK);
    p.line_segment(
        [pos2(cx - 3.0, cy - 5.0), pos2(cx - 3.0, cy + 5.0)],
        Stroke::new(1.5, Color32::WHITE),
    );
    p.line_segment(
        [pos2(cx - 3.0, cy - 5.0), pos2(cx + 5.0, cy)],
        Stroke::new(1.5, Color32::WHITE),
    );
    p.line_segment(
        [pos2(cx - 3.0, cy + 5.0), pos2(cx + 5.0, cy)],
        Stroke::new(1.5, Color32::WHITE),
    );
}

// ── Info Section ────────────────────────────────────────────────────────────
fn draw_info_section(ui: &mut Ui, def: &WidgetDef) {
    ui.label(
        RichText::new(def.title)
            .color(text_primary())
            .font(FontId::proportional(26.0)),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new(def.description)
            .color(text_muted())
            .size(13.0),
    );
}

fn draw_section_header(ui: &mut Ui, title: &str, color: Color32) {
    ui.horizontal(|ui| {
        let (bar, _) = ui.allocate_exact_size(vec2(3.0, 18.0), Sense::hover());
        ui.painter().rect_filled(bar, CornerRadius::same(2), color);
        ui.add_space(8.0);
        ui.label(
            RichText::new(title)
                .color(text_primary())
                .font(FontId::proportional(16.0)),
        );
    });
    ui.add_space(2.0);
    let (sep, _) = ui.allocate_exact_size(
        vec2(ui.available_width().min(700.0), 1.0),
        Sense::hover(),
    );
    ui.painter()
        .rect_filled(sep, CornerRadius::ZERO, Color32::from_rgb(26, 32, 40));
}

// ── Settings Card Wrapper ───────────────────────────────────────────────────
fn settings_card(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    Frame::new()
        .fill(surface_el())
        .stroke(Stroke::new(1.0, border_color()))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(20, 16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width().min(700.0));
            add_contents(ui);
        });
}

fn card_separator(ui: &mut Ui) {
    ui.add_space(10.0);
    let (sep, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter()
        .rect_filled(sep, CornerRadius::ZERO, Color32::from_rgb(30, 36, 44));
    ui.add_space(10.0);
}

// ── General Settings ────────────────────────────────────────────────────────
fn draw_general_settings(ui: &mut Ui, app: &mut SettingsApp) {
    settings_card(ui, |ui| {
        // Widget enabled
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Widget Enabled").color(text_primary()).size(14.0));
                ui.label(
                    RichText::new("Show this widget on the taskbar")
                        .color(text_muted())
                        .size(11.0),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if toggle_switch(ui, &mut app.widget_enabled) {
                    app.dirty = true;
                    app.status = String::new();
                }
            });
        });

        card_separator(ui);

        // Refresh interval
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Refresh Interval")
                        .color(text_primary())
                        .size(14.0),
                );
                ui.label(
                    RichText::new("Update frequency in seconds")
                        .color(text_muted())
                        .size(11.0),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let r = ui.add_sized(
                    [80.0, 30.0],
                    TextEdit::singleline(&mut app.refresh_interval)
                        .font(FontId::proportional(13.0))
                        .horizontal_align(Align::Center),
                );
                if r.changed() {
                    app.dirty = true;
                    app.status = String::new();
                }
            });
        });

        card_separator(ui);

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Move Left").color(text_primary()).size(14.0));
                    ui.label(
                        RichText::new("Avoid Windows widgets and crowded tray areas")
                            .color(text_muted())
                            .size(11.0),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{} px", app.widget_offset_px))
                            .color(text_muted())
                            .size(12.0),
                    );
                });
            });
            ui.add_space(8.0);
            let response = ui.add(
                egui::Slider::new(&mut app.widget_offset_px, 0..=480)
                    .show_value(false)
                    .step_by(4.0),
            );
            if response.changed() {
                app.dirty = true;
                app.status = String::new();
            }
        });
    });
}

// ── Rotation Settings ───────────────────────────────────────────────────────
fn draw_rotation_settings(ui: &mut Ui, app: &mut SettingsApp) {
    settings_card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Auto Rotate Widgets").color(text_primary()).size(14.0));
                ui.label(
                    RichText::new("Cycle through selected widgets in the order below")
                        .color(text_muted())
                        .size(11.0),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if toggle_switch(ui, &mut app.rotation_enabled) {
                    app.dirty = true;
                    app.status = String::new();
                }
            });
        });

        card_separator(ui);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Slide Interval").color(text_primary()).size(14.0));
                ui.label(
                    RichText::new("Seconds before switching to the next widget")
                        .color(text_muted())
                        .size(11.0),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mut seconds = app
                    .rotation_interval
                    .parse::<u32>()
                    .unwrap_or(30)
                    .clamp(5, 300);
                let response = ui.add_sized(
                    [240.0, 28.0],
                    egui::Slider::new(&mut seconds, 5..=300)
                        .suffix(" sec")
                        .show_value(true),
                );
                if response.changed() {
                    app.rotation_interval = seconds.to_string();
                    app.dirty = true;
                    app.status = String::new();
                }
            });
        });

        card_separator(ui);

        ui.label(RichText::new("Rotation Queue").color(text_primary()).size(14.0));
        ui.add_space(2.0);
        ui.label(
            RichText::new("Choose widgets and arrange the exact sequence")
                .color(text_muted())
                .size(11.0),
        );
        ui.add_space(10.0);

        let defs = widget_defs();
        for def in defs.iter() {
            let included = app.rotation_designs.iter().any(|id| id == def.id);
            let mut checked = included;

            Frame::new()
                .fill(Color32::from_rgb(20, 24, 30))
                .stroke(Stroke::new(
                    1.0,
                    if included {
                        Color32::from_rgba_unmultiplied(
                            def.accent.r(),
                            def.accent.g(),
                            def.accent.b(),
                            130,
                        )
                    } else {
                        Color32::from_rgb(31, 38, 49)
                    },
                ))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let checkbox = ui.checkbox(&mut checked, "");
                        if checkbox.changed() {
                            if checked {
                                if !app.rotation_designs.iter().any(|id| id == def.id) {
                                    app.rotation_designs.push(def.id.to_owned());
                                }
                            } else {
                                app.rotation_designs.retain(|id| id != def.id);
                                if app.rotation_designs.is_empty() {
                                    app.rotation_designs.push(def.id.to_owned());
                                }
                            }
                            app.dirty = true;
                            app.status = String::new();
                        }

                        let (dot, _) = ui.allocate_exact_size(vec2(9.0, 9.0), Sense::hover());
                        ui.painter().circle_filled(dot.center(), 4.5, def.accent);

                        ui.vertical(|ui| {
                            ui.label(RichText::new(def.title).color(text_primary()).size(13.0));
                            ui.label(RichText::new(def.subtitle).color(text_muted()).size(11.0));
                        });

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if included {
                                let index = app
                                    .rotation_designs
                                    .iter()
                                    .position(|id| id == def.id)
                                    .unwrap_or(0);
                                let down_enabled = index + 1 < app.rotation_designs.len();
                                let up_enabled = index > 0;

                                let down = ui.add_enabled(
                                    down_enabled,
                                    Button::new(RichText::new("↓").size(14.0))
                                        .fill(surface_el())
                                        .stroke(Stroke::new(1.0, border_color()))
                                        .corner_radius(CornerRadius::same(8)),
                                );
                                if down.clicked() {
                                    app.rotation_designs.swap(index, index + 1);
                                    app.dirty = true;
                                    app.status = String::new();
                                }

                                let up = ui.add_enabled(
                                    up_enabled,
                                    Button::new(RichText::new("↑").size(14.0))
                                        .fill(surface_el())
                                        .stroke(Stroke::new(1.0, border_color()))
                                        .corner_radius(CornerRadius::same(8)),
                                );
                                if up.clicked() {
                                    app.rotation_designs.swap(index, index - 1);
                                    app.dirty = true;
                                    app.status = String::new();
                                }

                                ui.label(
                                    RichText::new(format!("#{}", index + 1))
                                        .color(def.accent)
                                        .size(12.0),
                                );
                            } else {
                                ui.label(RichText::new("Off").color(text_muted()).size(12.0));
                            }
                        });
                    });
                });
            ui.add_space(8.0);
        }

        let summary = app
            .rotation_designs
            .iter()
            .map(|id| widget_def_by_id(id).title)
            .collect::<Vec<_>>()
            .join("  →  ");
        ui.add_space(2.0);
        ui.label(
            RichText::new(format!("Sequence: {summary}"))
                .color(Color32::from_rgb(148, 163, 184))
                .size(12.0),
        );
    });
}

// ── Widget-Specific Settings ────────────────────────────────────────────────
fn draw_widget_settings(ui: &mut Ui, app: &mut SettingsApp, def: &WidgetDef) {
    match def.id {
        CODEX_DESIGN => draw_codex_settings(ui, app),
        WEATHER_DESIGN => draw_weather_settings(ui, app),
        DISCORD_DESIGN => draw_discord_settings(ui, app),
        MEDIA_DESIGN => draw_media_settings(ui, app),
        _ => {}
    }
}

fn draw_codex_settings(ui: &mut Ui, app: &mut SettingsApp) {
    settings_card(ui, |ui| {
        text_field_setting(
            ui,
            "API Endpoint",
            "Custom API endpoint URL",
            &mut app.codex_endpoint,
            "https://api.example.com",
            &mut app.dirty,
            &mut app.status,
        );

        card_separator(ui);

        text_field_setting(
            ui,
            "Project Filter",
            "Filter displayed projects by name",
            &mut app.codex_filter,
            "my-project",
            &mut app.dirty,
            &mut app.status,
        );
    });
}

fn draw_weather_settings(ui: &mut Ui, app: &mut SettingsApp) {
    settings_card(ui, |ui| {
        text_field_setting(
            ui,
            "City",
            "Weather location name",
            &mut app.weather_city,
            "Istanbul",
            &mut app.dirty,
            &mut app.status,
        );

        card_separator(ui);

        // Temperature unit
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Temperature Unit")
                        .color(text_primary())
                        .size(14.0),
                );
                ui.label(
                    RichText::new("Display format for temperature")
                        .color(text_muted())
                        .size(11.0),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if segment_button(ui, &["°C", "°F"], &mut app.weather_unit_idx) {
                    app.dirty = true;
                    app.status = String::new();
                }
            });
        });
    });
}

fn draw_discord_settings(ui: &mut Ui, app: &mut SettingsApp) {
    settings_card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Discord Integration").color(text_primary()).size(14.0));
                ui.label(
                    RichText::new("Read selected voice channel users from the Discord desktop app")
                        .color(text_muted())
                        .size(11.0),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if toggle_switch(ui, &mut app.discord_enabled) {
                    app.dirty = true;
                    app.status = String::new();
                }
            });
        });

        card_separator(ui);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Widget Background").color(text_primary()).size(14.0));
                ui.label(
                    RichText::new("Show the black capsule behind Discord avatars")
                        .color(text_muted())
                        .size(11.0),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if toggle_switch(ui, &mut app.discord_background_enabled) {
                    app.dirty = true;
                    app.status = String::new();
                }
            });
        });

        card_separator(ui);

        text_field_setting(
            ui,
            "Client ID",
            "Discord application client id",
            &mut app.discord_client_id,
            "1525972653641433288",
            &mut app.dirty,
            &mut app.status,
        );

        card_separator(ui);

        secret_field_setting(
            ui,
            "Client Secret",
            "Stored in widget-settings.json; leave empty to disable auto authorize",
            &mut app.discord_client_secret,
            "client secret",
            &mut app.dirty,
            &mut app.status,
        );

        card_separator(ui);

        text_field_setting(
            ui,
            "Redirect URI",
            "Must match Discord Developer Portal redirect URI",
            &mut app.discord_redirect_uri,
            "http://127.0.0.1/callback",
            &mut app.dirty,
            &mut app.status,
        );
    });
}

fn draw_media_settings(ui: &mut Ui, app: &mut SettingsApp) {
    settings_card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Dark Mode").color(text_primary()).size(14.0));
                ui.label(
                    RichText::new("Use a graphite card with a cyan media control")
                        .color(text_muted())
                        .size(11.0),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if toggle_switch(ui, &mut app.media_dark_mode) {
                    app.dirty = true;
                    app.status = String::new();
                }
            });
        });
    });
}

fn text_field_setting(
    ui: &mut Ui,
    label: &str,
    hint: &str,
    value: &mut String,
    placeholder: &str,
    dirty: &mut bool,
    status: &mut String,
) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).color(text_primary()).size(14.0));
        ui.label(RichText::new(hint).color(text_muted()).size(11.0));
        ui.add_space(6.0);
        let r = ui.add_sized(
            [ui.available_width().min(420.0), 32.0],
            TextEdit::singleline(value)
                .font(FontId::proportional(13.0))
                .hint_text(RichText::new(placeholder).color(Color32::from_rgb(55, 65, 78))),
        );
        if r.changed() {
            *dirty = true;
            *status = String::new();
        }
    });
}

fn secret_field_setting(
    ui: &mut Ui,
    label: &str,
    hint: &str,
    value: &mut String,
    placeholder: &str,
    dirty: &mut bool,
    status: &mut String,
) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).color(text_primary()).size(14.0));
        ui.label(RichText::new(hint).color(text_muted()).size(11.0));
        ui.add_space(6.0);
        let r = ui.add_sized(
            [ui.available_width().min(420.0), 32.0],
            TextEdit::singleline(value)
                .password(true)
                .font(FontId::proportional(13.0))
                .hint_text(RichText::new(placeholder).color(Color32::from_rgb(55, 65, 78))),
        );
        if r.changed() {
            *dirty = true;
            *status = String::new();
        }
    });
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Custom Controls
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Animated toggle switch. Returns `true` if the value was changed.
fn toggle_switch(ui: &mut Ui, value: &mut bool) -> bool {
    let size = vec2(46.0, 26.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    let changed = response.clicked();
    if changed {
        *value = !*value;
    }

    let t = ui.ctx().animate_bool(response.id, *value);
    let p = ui.painter();

    // Track
    let track = lerp_color(Color32::from_rgb(50, 56, 65), success(), t);
    p.rect_filled(rect, CornerRadius::same(13), track);

    // Inner shadow on the track
    p.rect_stroke(
        rect,
        CornerRadius::same(13),
        Stroke::new(
            0.8,
            lerp_color(
                Color32::from_rgb(38, 43, 52),
                Color32::from_rgb(12, 160, 110),
                t,
            ),
        ),
        egui::StrokeKind::Inside,
    );

    // Thumb
    let thumb_x = rect.left() + 13.0 + (rect.width() - 26.0) * t;
    p.circle_filled(pos2(thumb_x, rect.center().y), 9.5, Color32::WHITE);
    // Thumb shadow
    p.circle_stroke(
        pos2(thumb_x, rect.center().y),
        9.5,
        Stroke::new(0.5, Color32::from_rgba_unmultiplied(0, 0, 0, 40)),
    );

    changed
}

/// Segmented button control. Returns `true` if the selection changed.
fn segment_button(ui: &mut Ui, labels: &[&str], selected: &mut usize) -> bool {
    let btn_w = 52.0;
    let btn_h = 32.0;
    let total_w = btn_w * labels.len() as f32;
    let (rect, _) = ui.allocate_exact_size(vec2(total_w, btn_h), Sense::hover());
    let p = ui.painter();

    // Track background
    p.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(26, 32, 40));
    p.rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(1.0, Color32::from_rgb(36, 44, 54)),
        egui::StrokeKind::Inside,
    );

    let mut changed = false;
    for (i, label) in labels.iter().enumerate() {
        let br = egui::Rect::from_min_size(
            pos2(rect.left() + i as f32 * btn_w, rect.top()),
            vec2(btn_w, btn_h),
        );
        let resp = ui.interact(br, ui.id().with(("seg", i)), Sense::click());

        if i == *selected {
            p.rect_filled(br.shrink(3.0), CornerRadius::same(6), accent());
            p.text(
                br.center(),
                Align2::CENTER_CENTER,
                *label,
                FontId::proportional(13.0),
                Color32::WHITE,
            );
        } else {
            if resp.hovered() {
                p.rect_filled(
                    br.shrink(3.0),
                    CornerRadius::same(6),
                    Color32::from_rgb(34, 40, 50),
                );
            }
            p.text(
                br.center(),
                Align2::CENTER_CENTER,
                *label,
                FontId::proportional(13.0),
                text_muted(),
            );
        }

        if resp.clicked() && *selected != i {
            *selected = i;
            changed = true;
        }
    }
    changed
}

// ── Action Buttons ──────────────────────────────────────────────────────────
fn draw_action_buttons(ui: &mut Ui, app: &mut SettingsApp) {
    ui.horizontal_wrapped(|ui| {
        // Save
        let save_fill = if app.dirty { accent() } else { surface_el() };
        let save_stroke = if app.dirty {
            Stroke::new(1.0, Color32::from_rgb(128, 112, 241))
        } else {
            Stroke::new(1.0, border_color())
        };
        let save_text = if app.dirty {
            "💾  Save Changes"
        } else {
            "✓  Saved"
        };
        let save_color = if app.dirty {
            Color32::WHITE
        } else {
            text_muted()
        };

        let save = ui.add_sized(
            [150.0, 40.0],
            Button::new(RichText::new(save_text).color(save_color).size(13.0))
                .fill(save_fill)
                .stroke(save_stroke)
                .corner_radius(CornerRadius::same(10)),
        );
        if save.clicked() && app.dirty {
            app.save();
        }

        ui.add_space(6.0);

        // Reset
        let reset = ui.add_sized(
            [100.0, 40.0],
            Button::new(RichText::new("↺  Reset").color(text_muted()).size(13.0))
                .fill(surface_el())
                .stroke(Stroke::new(1.0, border_color()))
                .corner_radius(CornerRadius::same(10)),
        );
        if reset.clicked() {
            app.reset();
        }

        ui.add_space(6.0);

        // Open folder
        let folder = ui.add_sized(
            [140.0, 40.0],
            Button::new(
                RichText::new("📁  Design Packs")
                    .color(text_primary())
                    .size(13.0),
            )
            .fill(surface_el())
            .stroke(Stroke::new(1.0, border_color()))
            .corner_radius(CornerRadius::same(10)),
        );
        if folder.clicked() {
            app.open_widget_libraries();
        }
    });
}
