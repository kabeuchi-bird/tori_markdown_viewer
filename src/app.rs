use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, SyncSender};

use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use notify::{EventKind, RecursiveMode, Watcher};

use crate::settings::{ColorScheme, Settings, ViewMode};

pub struct App {
    // Content
    markdown: String,
    /// Separate clone used by the read-only TextEdit in Source mode.
    source_text: String,
    current_file: Option<PathBuf>,

    // Settings (persisted via eframe::Storage)
    settings: Settings,

    // File-watcher: created once in new(), path updated in open_file()
    watcher: Option<notify::RecommendedWatcher>,
    reload_rx: Receiver<()>,

    // Markdown rendering cache (reset when content changes)
    md_cache: CommonMarkCache,

    // Transient UI
    status: String,
    /// Set to true when Open button is clicked; dialog shown after panel render.
    open_dialog: bool,

    // Font selection
    /// All system font family names, sorted alphabetically.
    font_families: Vec<String>,
    /// System default font family detected via fontconfig (fc-match), if available.
    system_default_font: Option<String>,
    /// Search string for the font ComboBox popup.
    font_search: String,
    /// Which font was last applied to egui so we don't call set_fonts every frame.
    last_applied_font: Option<String>,
    /// Raw font data for the currently applied font (kept alive for egui).
    loaded_font_data: Option<Vec<u8>>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings: Settings = cc
            .storage
            .and_then(|s| eframe::get_value(s, "settings"))
            .unwrap_or_default();

        // Build the file-watcher channel. The watcher itself is created lazily
        // (on first file open) so we can capture the egui Context.
        let (_tx_placeholder, rx) = mpsc::sync_channel::<()>(0);

        // Enumerate system fonts once at startup.
        let font_families = enumerate_system_fonts();
        // Font detection chain: DE setting → fontconfig → None (egui built-ins).
        let system_default_font = detect_de_font().or_else(detect_system_default_font);

        let mut app = Self {
            markdown: String::new(),
            source_text: String::new(),
            current_file: None,
            settings: settings.clone(),
            watcher: None,
            reload_rx: rx,
            md_cache: CommonMarkCache::default(),
            status: "No file open — drop a Markdown file here or click Open".into(),
            open_dialog: false,
            font_families,
            system_default_font,
            font_search: String::new(),
            last_applied_font: None,
            loaded_font_data: None,
        };

        // Apply the saved color scheme before the first frame.
        app.apply_scheme(&cc.egui_ctx);

        // Apply the saved (or default) font before the first frame.
        app.apply_font(&cc.egui_ctx);

        // Re-open last file.
        if let Some(path) = settings.last_file.as_deref().map(PathBuf::from) {
            app.open_file_inner(path, &cc.egui_ctx);
        }

        app
    }

    // ------------------------------------------------------------------ file I/O

    /// Open a file, update the watcher, and refresh the displayed content.
    pub fn open_file(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.open_file_inner(path, ctx);
    }

    fn open_file_inner(&mut self, path: PathBuf, ctx: &egui::Context) {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                // Ensure the watcher exists; create it on the first open.
                if self.watcher.is_none() {
                    let (tx, rx): (SyncSender<()>, Receiver<()>) = mpsc::sync_channel(8);
                    let ctx2 = ctx.clone();
                    match notify::recommended_watcher(
                        move |res: notify::Result<notify::Event>| {
                            if let Ok(ev) = res {
                                if matches!(ev.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                                    let _ = tx.try_send(());
                                    ctx2.request_repaint();
                                }
                            }
                        },
                    ) {
                        Ok(w) => {
                            self.reload_rx = rx;
                            self.watcher = Some(w);
                        }
                        Err(_) => {}
                    }
                }

                // Update watched path.
                if let Some(ref mut w) = self.watcher {
                    if let Some(ref old) = self.current_file {
                        let _ = w.unwatch(old);
                    }
                    let _ = w.watch(&path, RecursiveMode::NonRecursive);
                }

                // Update content.
                self.source_text = content.clone();
                self.markdown = content;
                self.md_cache = CommonMarkCache::default();
                self.settings.last_file = Some(path.to_string_lossy().into_owned());
                self.status = path.to_string_lossy().into_owned();
                // Update window title: "filename.md - tori markdown viewer"
                if let Some(name) = path.file_name() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                        format!("{} - tori markdown viewer", name.to_string_lossy()),
                    ));
                }
                self.current_file = Some(path);
            }
            Err(e) => {
                self.status = format!("Error: {e}");
            }
        }
    }

    fn reload_current(&mut self) {
        if let Some(path) = self.current_file.clone() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                self.source_text = content.clone();
                self.markdown = content;
                self.md_cache = CommonMarkCache::default();
            }
        }
    }

    // ------------------------------------------------------------------ theme

    fn apply_scheme(&self, ctx: &egui::Context) {
        match self.settings.color_scheme {
            ColorScheme::Light => ctx.set_visuals(egui::Visuals::light()),
            ColorScheme::Dark => ctx.set_visuals(egui::Visuals::dark()),
            ColorScheme::Auto => {
                // eframe sets the initial visuals from the OS; leave them as-is.
            }
        }
    }

    // ------------------------------------------------------------------ font

    /// Load the font matching `settings.font_family` and register it with egui.
    /// When `font_family` is None, the OS default font (via fontconfig) is used;
    /// falls back to egui built-in fonts if detection or loading fails.
    fn apply_font(&mut self, ctx: &egui::Context) {
        let desired = self.settings.font_family.clone();

        if desired == self.last_applied_font {
            return; // Nothing changed.
        }

        // Resolve which family to actually load.
        let family_to_load: Option<&str> = match &desired {
            Some(name) => Some(name.as_str()),
            None => self.system_default_font.as_deref(),
        };

        match family_to_load.and_then(|name| load_font_data(name)) {
            Some(data) => {
                let mut fonts = egui::FontDefinitions::default();
                let key = "user_font".to_owned();
                fonts.font_data.insert(
                    key.clone(),
                    egui::FontData::from_owned(data.clone()).into(),
                );
                // Prepend our font so it takes priority over the built-ins.
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, key.clone());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, key.clone());
                ctx.set_fonts(fonts);
                self.loaded_font_data = Some(data);
            }
            None => {
                // System font not found or no fontconfig; fall back to egui built-ins.
                ctx.set_fonts(egui::FontDefinitions::default());
                self.loaded_font_data = None;
            }
        }

        self.last_applied_font = desired;
    }

    // ------------------------------------------------------------------ UI helpers

    /// Draw the toolbar. Returns `(open_requested, scheme_changed, font_changed)`.
    fn toolbar_ui(&mut self, ui: &mut egui::Ui) -> (bool, bool, bool) {
        let mut open_requested = false;
        let mut scheme_changed = false;
        let mut font_changed = false;

        ui.horizontal(|ui| {
            if ui.button("Open").on_hover_text("Open a Markdown file (Ctrl+O)").clicked() {
                open_requested = true;
            }

            ui.separator();

            ui.selectable_value(&mut self.settings.view_mode, ViewMode::Normal,    "Normal");
            ui.selectable_value(&mut self.settings.view_mode, ViewMode::Decorated, "Decorated");
            ui.selectable_value(&mut self.settings.view_mode, ViewMode::Source,    "Source");

            ui.separator();

            ui.checkbox(&mut self.settings.word_wrap, "Wrap");

            ui.separator();

            ui.label("Size:");
            ui.add(
                egui::DragValue::new(&mut self.settings.font_size)
                    .range(8.0..=72.0_f32)
                    .speed(0.5)
                    .suffix("pt"),
            );

            ui.separator();

            // ---- Font selector ----
            ui.label("Font:");
            let current_label = self.settings.font_family.clone().unwrap_or_else(|| {
                match &self.system_default_font {
                    Some(name) => format!("{name} (OS default)"),
                    None => "System default".to_owned(),
                }
            });

            egui::ComboBox::from_id_source("font_selector")
                .selected_text(&current_label)
                .width(160.0)
                .show_ui(ui, |ui: &mut egui::Ui| {
                    // Search box at the top of the dropdown.
                    ui.text_edit_singleline(&mut self.font_search);

                    let search_lower = self.font_search.to_lowercase();

                    // "System default" option.
                    let selected_default = self.settings.font_family.is_none();
                    if ui
                        .selectable_label(selected_default, "System default")
                        .clicked()
                        && !selected_default
                    {
                        self.settings.font_family = None;
                        font_changed = true;
                    }

                    // System font list, filtered by search.
                    for name in &self.font_families {
                        if !search_lower.is_empty()
                            && !name.to_lowercase().contains(&search_lower)
                        {
                            continue;
                        }
                        let selected = self.settings.font_family.as_deref() == Some(name.as_str());
                        if ui.selectable_label(selected, name).clicked() && !selected {
                            self.settings.font_family = Some(name.clone());
                            font_changed = true;
                        }
                    }
                });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = match self.settings.color_scheme {
                    ColorScheme::Auto  => "Auto",
                    ColorScheme::Light => "Light",
                    ColorScheme::Dark  => "Dark",
                };
                if ui.button(label).on_hover_text("Color scheme: Auto / Light / Dark").clicked() {
                    self.settings.color_scheme = match self.settings.color_scheme {
                        ColorScheme::Auto  => ColorScheme::Light,
                        ColorScheme::Light => ColorScheme::Dark,
                        ColorScheme::Dark  => ColorScheme::Auto,
                    };
                    scheme_changed = true;
                }
            });
        });

        (open_requested, scheme_changed, font_changed)
    }

    /// Draw the main content area.
    fn content_ui(&mut self, ui: &mut egui::Ui) {
        if self.markdown.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("Drop a Markdown file here, or click Open");
            });
            return;
        }

        let font_size = self.settings.font_size.max(8.0);

        // Apply per-panel font size without touching the global style.
        let mut style = (*ui.ctx().style()).clone();
        for (ts, fid) in style.text_styles.iter_mut() {
            if *ts == egui::TextStyle::Body || *ts == egui::TextStyle::Small {
                fid.size = font_size;
            } else if *ts == egui::TextStyle::Heading {
                fid.size = font_size * 1.6;
            }
        }
        ui.set_style(style);

        match self.settings.view_mode {
            ViewMode::Normal => {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.settings.word_wrap {
                            ui.set_max_width(ui.available_width());
                        }
                        CommonMarkViewer::new("md_normal")
                            .show(ui, &mut self.md_cache, &self.markdown);
                    });
            }

            ViewMode::Decorated => {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let frame = egui::Frame {
                            fill: ui.visuals().extreme_bg_color,
                            inner_margin: egui::Margin::same(28.0),
                            rounding: egui::Rounding::same(6.0),
                            ..Default::default()
                        };
                        frame.show(ui, |ui| {
                            if self.settings.word_wrap {
                                ui.set_max_width(ui.available_width().min(840.0));
                            }
                            let decorated = decorate_markdown(&self.markdown);
                            CommonMarkViewer::new("md_decorated")
                                .show(ui, &mut self.md_cache, &decorated);
                        });
                    });
            }

            ViewMode::Source => {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let desired_width = if self.settings.word_wrap {
                            ui.available_width()
                        } else {
                            f32::INFINITY
                        };
                        ui.add(
                            egui::TextEdit::multiline(&mut self.source_text)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(desired_width)
                                .interactive(false),
                        );
                    });
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ---- Check for file-watcher reload signals ----
        let mut changed = false;
        while self.reload_rx.try_recv().is_ok() {
            changed = true;
        }
        if changed {
            self.reload_current();
        }

        // ---- Handle drag-and-drop (extract path before panel closures) ----
        let dropped: Option<PathBuf> = ctx.input(|i| {
            i.raw.dropped_files.first().and_then(|f| f.path.clone())
        });

        // ---- Toolbar ----
        let (open_requested, scheme_changed, font_changed) =
            egui::TopBottomPanel::top("toolbar")
                .show(ctx, |ui| self.toolbar_ui(ui))
                .inner;

        if scheme_changed {
            self.apply_scheme(ctx);
        }
        if font_changed {
            self.apply_font(ctx);
        }

        // ---- Status bar ----
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(&self.status);
        });

        // ---- Content ----
        egui::CentralPanel::default().show(ctx, |ui| {
            self.content_ui(ui);
        });

        // ---- Post-render: handle file opens (after all panels are done) ----
        if let Some(path) = dropped {
            self.open_file(path, ctx);
        }

        if open_requested || self.open_dialog {
            self.open_dialog = false;
            let start_dir = self
                .current_file
                .as_deref()
                .and_then(|p| p.parent())
                .map(|p| p.to_path_buf());

            let mut dialog = rfd::FileDialog::new()
                .add_filter("Markdown", &["md", "markdown", "txt"])
                .add_filter("All files", &["*"]);
            if let Some(dir) = start_dir {
                dialog = dialog.set_directory(dir);
            }
            if let Some(path) = dialog.pick_file() {
                self.open_file(path, ctx);
            }
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "settings", &self.settings);
    }
}

// ------------------------------------------------------------------ font helpers

// ---- Desktop-environment font detection ----

/// Top-level: try to read the font configured in the current DE.
/// Returns None if the DE is undetected or if all DE-specific probes fail.
fn detect_de_font() -> Option<String> {
    // Collect DE hints from the two most common env vars.
    let xdg = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_uppercase();
    let ds  = std::env::var("DESKTOP_SESSION").unwrap_or_default().to_uppercase();
    let hint = format!("{xdg}:{ds}");

    // Try the DE-specific method first, then probe all as a last resort.
    if hint.contains("CINNAMON") {
        detect_cinnamon_font().or_else(detect_gnome_font)
    } else if hint.contains("GNOME")
           || hint.contains("UBUNTU")
           || hint.contains("BUDGIE")
           || hint.contains("PANTHEON")
    {
        detect_gnome_font()
    } else if hint.contains("MATE") {
        detect_mate_font()
    } else if hint.contains("KDE") {
        detect_kde_font()
    } else if hint.contains("XFCE") {
        detect_xfce_font()
    } else if hint.contains("LXQT") {
        detect_lxqt_font()
    } else {
        // Unknown DE — try each method in order.
        detect_gnome_font()
            .or_else(detect_cinnamon_font)
            .or_else(detect_mate_font)
            .or_else(detect_kde_font)
            .or_else(detect_xfce_font)
            .or_else(detect_lxqt_font)
    }
}

/// GNOME / Ubuntu / Budgie / Pantheon: read via gsettings.
fn detect_gnome_font() -> Option<String> {
    let raw = run_command(
        "gsettings",
        &["get", "org.gnome.desktop.interface", "font-name"],
    )?;
    parse_gtk_font_name(&raw)
}

/// Cinnamon: read via gsettings.
fn detect_cinnamon_font() -> Option<String> {
    let raw = run_command(
        "gsettings",
        &["get", "org.cinnamon.desktop.interface", "font-name"],
    )?;
    parse_gtk_font_name(&raw)
}

/// MATE: read via gsettings.
fn detect_mate_font() -> Option<String> {
    let raw = run_command(
        "gsettings",
        &["get", "org.mate.interface", "font-name"],
    )?;
    parse_gtk_font_name(&raw)
}

/// KDE Plasma: read from ~/.config/kdeglobals (INI, [General] > font).
fn detect_kde_font() -> Option<String> {
    let path = dirs_config()?.join("kdeglobals");
    let raw = read_ini_font(&path, "General", "font")?;
    parse_qt_font_name(&raw)
}

/// XFCE: try xfconf-query first, fall back to xsettings.xml.
fn detect_xfce_font() -> Option<String> {
    // Primary: xfconf-query
    if let Some(raw) = run_command(
        "xfconf-query",
        &["-c", "xsettings", "-p", "/Gtk/FontName"],
    ) {
        if let Some(name) = parse_gtk_font_name(&raw) {
            return Some(name);
        }
    }

    // Fallback: parse XML config file
    let path = dirs_config()?
        .join("xfce4/xfconf/xfce-perchannel-xml/xsettings.xml");
    let xml = std::fs::read_to_string(path).ok()?;
    // Find: <property name="FontName" type="string" value="Noto Sans 11"/>
    for line in xml.lines() {
        if line.contains(r#"name="FontName""#) {
            if let Some(v) = extract_xml_attr(line, "value") {
                return parse_gtk_font_name(&v);
            }
        }
    }
    None
}

/// LXQt: read from ~/.config/lxqt/lxqt.conf (INI, [General] > font).
fn detect_lxqt_font() -> Option<String> {
    let path = dirs_config()?.join("lxqt/lxqt.conf");
    let raw = read_ini_font(&path, "General", "font")?;
    parse_qt_font_name(&raw)
}

// ---- Parsing helpers ----

/// Parse GTK-style font string: `'Noto Sans 11'` or `"Ubuntu 12"` → `"Noto Sans"`.
/// Strips surrounding quotes/whitespace, then removes the trailing point-size token.
fn parse_gtk_font_name(s: &str) -> Option<String> {
    let s = s.trim().trim_matches(|c| c == '\'' || c == '"').trim();
    if s.is_empty() {
        return None;
    }
    // Font name may end with a size token (pure digits), e.g. "Noto Sans 11".
    // Strip it if present, keeping compound names like "DejaVu Sans Condensed 10".
    let family = match s.rsplit_once(' ') {
        Some((family, size)) if size.chars().all(|c| c.is_ascii_digit()) => family.trim(),
        _ => s,
    };
    if family.is_empty() { None } else { Some(family.to_owned()) }
}

/// Parse Qt-style font string: `Noto Sans,10,-1,5,50,0,0,0,0,0` → `"Noto Sans"`.
fn parse_qt_font_name(s: &str) -> Option<String> {
    let family = s.trim().split(',').next()?.trim().to_owned();
    if family.is_empty() { None } else { Some(family) }
}

/// Read an INI file and return the value for `key` inside `[section]`.
/// Handles both `key=value` and `key = value` with optional whitespace.
fn read_ini_font(path: &std::path::Path, section: &str, key: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let section_header = format!("[{section}]");
    let mut in_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed.eq_ignore_ascii_case(&section_header);
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            if k.trim().eq_ignore_ascii_case(key) {
                return Some(v.trim().to_owned());
            }
        }
    }
    None
}

/// Extract the value of an XML attribute from a single line, e.g. `value="Noto Sans 11"`.
fn extract_xml_attr<'a>(line: &'a str, attr: &str) -> Option<&'a str> {
    let needle = format!(r#"{attr}=""#);
    let start = line.find(needle.as_str())? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// Run an external command and return its stdout as a trimmed String, or None on failure.
fn run_command(program: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if s.is_empty() { None } else { Some(s) }
}

/// Returns the XDG config directory (`$XDG_CONFIG_HOME` or `~/.config`).
fn dirs_config() -> Option<std::path::PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config"))
        })
}

// ---- fontconfig fallback ----

/// Ask fontconfig for the default sans-serif font family.
/// Returns None if fc-match is unavailable or fails.
fn detect_system_default_font() -> Option<String> {
    let output = std::process::Command::new("fc-match")
        .args(["--format=%{family}", "sans"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    // fc-match may return a comma-separated list; take the first entry.
    let raw = String::from_utf8_lossy(&output.stdout);
    let family = raw.split(',').next()?.trim().to_owned();
    if family.is_empty() { None } else { Some(family) }
}

/// Return a sorted list of all system font family names.
fn enumerate_system_fonts() -> Vec<String> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let mut families: Vec<String> = db
        .faces()
        .filter_map(|f| f.families.first().map(|(name, _)| name.clone()))
        .collect();

    families.sort_unstable();
    families.dedup();
    families
}

// ------------------------------------------------------------------ decorated mode

const H1_DECO: &str   = "✼••┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈••✼";
const H2_DECO: &str   = "˖✻*˸ꕤ*˸*⋆。";
const H3_DECO: &str   = "✦";
const H4_DECO: &str   = "❧";
const H5_DECO: &str   = "✿";

/// Pre-process Markdown text for Decorated mode by inserting ornamental
/// decorations around headings. Code fences are left untouched.
fn decorate_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 512);
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim_start();

        // Toggle code-fence state; pass the fence line through unchanged.
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            out.push_str(line);
            out.push('\n');
            continue;
        }

        if in_code_block {
            out.push_str(line);
            out.push('\n');
            continue;
        }

        // Count leading '#' to find ATX heading level.
        let level = trimmed.chars().take_while(|&c| c == '#').count();

        if level > 0 && level <= 6 {
            let after = &trimmed[level..];
            // CommonMark requires either a space or end-of-line after the hashes.
            let title = if after.starts_with(' ') {
                // Strip trailing closing hashes (e.g., `## foo ##`)
                after[1..].trim_end_matches(|c: char| c == '#' || c == ' ')
            } else if after.is_empty() {
                ""
            } else {
                // Not a valid ATX heading (e.g., `#nospace`); pass through.
                out.push_str(line);
                out.push('\n');
                continue;
            };

            let hashes = &trimmed[..level];
            match level {
                1 => {
                    out.push('\n');
                    out.push_str(H1_DECO);
                    out.push_str("\n\n");
                    out.push_str(hashes);
                    out.push(' ');
                    out.push_str(title);
                    out.push_str("\n\n");
                    out.push_str(H1_DECO);
                    out.push_str("\n\n");
                }
                2 => {
                    out.push_str(hashes);
                    out.push(' ');
                    out.push_str(H2_DECO);
                    out.push(' ');
                    out.push_str(title);
                    out.push(' ');
                    out.push_str(H2_DECO);
                    out.push('\n');
                }
                3 => {
                    out.push_str(hashes);
                    out.push(' ');
                    out.push_str(H3_DECO);
                    out.push(' ');
                    out.push_str(title);
                    out.push(' ');
                    out.push_str(H3_DECO);
                    out.push('\n');
                }
                4 => {
                    out.push_str(hashes);
                    out.push(' ');
                    out.push_str(H4_DECO);
                    out.push(' ');
                    out.push_str(title);
                    out.push(' ');
                    out.push_str(H4_DECO);
                    out.push('\n');
                }
                5 | 6 => {
                    out.push_str(hashes);
                    out.push(' ');
                    out.push_str(H5_DECO);
                    out.push(' ');
                    out.push_str(title);
                    out.push(' ');
                    out.push_str(H5_DECO);
                    out.push('\n');
                }
                _ => unreachable!(),
            }
            continue;
        }

        out.push_str(line);
        out.push('\n');
    }

    out
}

/// Load raw font bytes for the first face matching the given family name.
fn load_font_data(family_name: &str) -> Option<Vec<u8>> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    // Find a face whose primary family matches.
    let face_id = db.faces().find(|f| {
        f.families
            .first()
            .map_or(false, |(n, _)| n.eq_ignore_ascii_case(family_name))
    })?
    .id;

    // fontdb can give us the raw bytes directly.
    db.with_face_data(face_id, |data, _index| data.to_vec())
}
