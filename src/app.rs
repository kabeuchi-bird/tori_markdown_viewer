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
    /// System fonts loaded at startup and appended as Unicode fallbacks.
    fallback_fonts: Vec<Vec<u8>>,
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

        // Enumerate system fonts once at startup (shared DB avoids rebuilding twice).
        let (font_families, fallback_fonts) = enumerate_and_collect_fonts();
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
            fallback_fonts,
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
                // Strip UTF-8 BOM (\u{FEFF}) that some editors prepend; it
                // would prevent pulldown-cmark from recognising the first `#`.
                let content = content.strip_prefix('\u{FEFF}').map_or(content.clone(), str::to_owned);
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
            if let Ok(raw) = std::fs::read_to_string(&path) {
                let content = raw.strip_prefix('\u{FEFF}').map_or(raw.clone(), str::to_owned);
                self.source_text = content.clone();
                self.markdown = content;
                self.md_cache = CommonMarkCache::default();
            }
        }
    }

    // ------------------------------------------------------------------ theme

    fn apply_scheme(&self, ctx: &egui::Context) {
        let mut visuals = match self.settings.color_scheme {
            ColorScheme::Light => egui::Visuals::light(),
            ColorScheme::Dark => egui::Visuals::dark(),
            ColorScheme::Auto => (*ctx.style()).visuals.clone(),
        };

        // Boost text contrast beyond egui's default.
        // Dark theme: default body text is ~Color32::from_gray(200); lift to near-white.
        // Light theme: default body text is ~Color32::from_gray(60); drop to near-black.
        let text_color = if visuals.dark_mode {
            egui::Color32::from_gray(242)
        } else {
            egui::Color32::from_gray(15)
        };
        visuals.widgets.noninteractive.fg_stroke.color = text_color;
        visuals.widgets.inactive.fg_stroke.color = text_color;

        ctx.set_visuals(visuals);
    }

    // ------------------------------------------------------------------ font

    /// Rebuild egui's FontDefinitions with:
    ///   1. User/system-default font prepended (highest priority)
    ///   2. egui built-in fonts (middle)
    ///   3. System fallback fonts appended (lowest priority, Unicode coverage)
    fn apply_font(&mut self, ctx: &egui::Context) {
        let desired = self.settings.font_family.clone();

        if desired == self.last_applied_font {
            return; // Nothing changed.
        }

        // Resolve which family to actually load as the primary font.
        let family_to_load: Option<&str> = match &desired {
            Some(name) => Some(name.as_str()),
            None => self.system_default_font.as_deref(),
        };

        // Start from egui defaults so built-in fonts remain in the middle.
        let mut fonts = egui::FontDefinitions::default();

        // --- Primary font (prepended → highest priority) ---
        if let Some(data) = family_to_load.and_then(|n| load_font_data(n)) {
            let key = "user_font".to_owned();
            fonts.font_data.insert(key.clone(), egui::FontData::from_owned(data).into());
            fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, key.clone());
            fonts.families.entry(egui::FontFamily::Monospace).or_default().insert(0, key);
        }

        // --- System fallback fonts (appended → lowest priority) ---
        // These cover Unicode ranges absent from the primary/built-in fonts.
        for (i, data) in self.fallback_fonts.iter().enumerate() {
            let key = format!("sys_fallback_{i}");
            fonts.font_data.insert(key.clone(), egui::FontData::from_owned(data.clone()).into());
            fonts.families.entry(egui::FontFamily::Proportional).or_default().push(key.clone());
            fonts.families.entry(egui::FontFamily::Monospace).or_default().push(key);
        }

        ctx.set_fonts(fonts);
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
                // Larger ratio → more visible size steps between H1–H6.
                fid.size = font_size * 2.2;
            }
        }
        ui.set_style(style);

        match self.settings.view_mode {
            ViewMode::Normal => {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // word_wrap=false: allow infinite horizontal extent so
                        // CommonMarkViewer never wraps (available_width → ∞).
                        // word_wrap=true: do NOT cap via set_max_width; let the
                        // scroll-area's natural available_width control wrapping
                        // so we never accidentally pass 0 on the first frame.
                        if !self.settings.word_wrap {
                            ui.set_max_width(f32::INFINITY);
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
                            // Wider vertical spacing between headings and body text.
                            // ui.style() is &Arc<Style>; double-deref to clone the Style.
                            let mut inner_style = (**ui.style()).clone();
                            inner_style.spacing.item_spacing.y = 8.0;
                            ui.set_style(inner_style);

                            if self.settings.word_wrap {
                                // Cap at 840 px for comfortable reading width.
                                // Guard against the 0-width first frame by only
                                // capping when the value is actually meaningful.
                                let w = ui.available_width();
                                if w > 0.0 {
                                    ui.set_max_width(w.min(840.0));
                                }
                            } else {
                                ui.set_max_width(f32::INFINITY);
                            }
                            let preprocessed = preprocess_decorated(&self.markdown);
                            CommonMarkViewer::new("md_decorated")
                                .show(ui, &mut self.md_cache, &preprocessed);
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

        // ---- TOC sidebar (Decorated mode only, file loaded) ----
        // SidePanel must be added before CentralPanel.
        if self.settings.view_mode == ViewMode::Decorated && !self.markdown.is_empty() {
            let toc = extract_toc(&self.markdown);
            let font_size = self.settings.font_size.max(8.0);
            egui::SidePanel::left("toc_panel")
                .resizable(true)
                .min_width(120.0)
                .default_width(200.0)
                .show(ctx, |ui| {
                    ui.add_space(6.0);
                    ui.strong("Contents");
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for entry in &toc {
                            let indent = (entry.level - 1) as f32 * 10.0;
                            ui.horizontal(|ui| {
                                ui.add_space(indent);
                                let size = font_size * match entry.level {
                                    1 => 1.0,
                                    2 => 0.9,
                                    _ => 0.82,
                                };
                                let text = egui::RichText::new(&entry.title).size(size);
                                let text = if entry.level <= 2 { text.strong() } else { text };
                                ui.label(text);
                            });
                        }
                    });
                });
        }

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

/// Build the fontdb once and return:
///   - sorted list of all family names (for the font picker ComboBox)
///   - raw font data for Unicode fallback fonts (in priority order)
fn enumerate_and_collect_fonts() -> (Vec<String>, Vec<Vec<u8>>) {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    // All family names for the ComboBox.
    let mut families: Vec<String> = db
        .faces()
        .filter_map(|f| f.families.first().map(|(name, _)| name.clone()))
        .collect();
    families.sort_unstable();
    families.dedup();

    // Fallback fonts: prioritise broad Unicode coverage.
    // Noto fonts cover virtually all of Unicode by design.
    // DejaVu/Liberation/Unifont fill in where Noto is absent.
    let priority: &[&str] = &[
        "Noto Sans",
        "Noto Serif",
        "Noto Sans CJK JP",
        "Noto Sans CJK SC",
        "Noto Sans CJK TC",
        "Noto Sans CJK KR",
        "Noto Color Emoji",
        "Noto Emoji",
        "DejaVu Sans",
        "Liberation Sans",
        "FreeSans",
        "Symbola",   // exceptional symbol/historic script coverage
        "Unifont",   // bitmap-backed, covers almost all of BMP
    ];

    let mut fallbacks: Vec<Vec<u8>> = Vec::new();
    for &name in priority {
        if let Some(face) = db.faces().find(|f| {
            f.families.first().map_or(false, |(n, _)| n.eq_ignore_ascii_case(name))
        }) {
            if let Some(data) = db.with_face_data(face.id, |d, _| d.to_vec()) {
                fallbacks.push(data);
            }
        }
        if fallbacks.len() >= 6 {
            break;
        }
    }

    // Additionally, ask fontconfig which fonts cover each decoration character.
    // This guarantees display even when none of the priority fonts are installed.
    for data in collect_deco_fonts() {
        if fallbacks.len() >= 10 {
            break;
        }
        fallbacks.push(data);
    }

    (families, fallbacks)
}

/// Use `fc-match :charset=XXXX` to find the best font covering each decoration
/// character. Returns unique font file data not already in the priority list.
fn collect_deco_fonts() -> Vec<Vec<u8>> {
    // Codepoints used in H*_DECO that are absent from Ubuntu Light / Hack.
    const DECO_CPS: &[u32] = &[
        0x273C, // ✼  OPEN CENTRE TEARDROP-SPOKED ASTERISK
        0x2508, // ┈  BOX DRAWINGS LIGHT QUADRUPLE DASH HORIZONTAL
        0x2726, // ✦  BLACK FOUR POINTED STAR
        0x273B, // ✻  TEARDROP-SPOKED ASTERISK
        0x2724, // ✤  HEAVY FOUR BALLOON-SPOKED ASTERISK
        0x2767, // ❧  ROTATED FLORAL HEART BULLET
        0x273F, // ✿  BLACK FLORETTE
    ];

    let mut seen: std::collections::HashSet<String> = Default::default();
    let mut result: Vec<Vec<u8>> = Vec::new();

    for &cp in DECO_CPS {
        let pattern = format!(":charset={cp:X}");
        if let Some(path) = run_command("fc-match", &["--format=%{file}", &pattern]) {
            if seen.insert(path.clone()) {
                if let Ok(data) = std::fs::read(&path) {
                    result.push(data);
                }
            }
        }
    }

    result
}

// ------------------------------------------------------------------ TOC helpers

struct TocEntry {
    level: u8,
    title: String,
}

/// Parse ATX headings from raw Markdown, skipping code fences.
fn extract_toc(markdown: &str) -> Vec<TocEntry> {
    let mut entries = Vec::new();
    let mut in_code = false;
    for line in markdown.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let level = t.chars().take_while(|&c| c == '#').count();
        if level == 0 || level > 6 {
            continue;
        }
        let rest = &t[level..];
        if rest.starts_with(' ') {
            let title = rest[1..].trim_end_matches(|c: char| c == '#' || c == ' ');
            entries.push(TocEntry { level: level as u8, title: title.to_owned() });
        } else if rest.is_empty() {
            entries.push(TocEntry { level: level as u8, title: String::new() });
        }
    }
    entries
}

/// Pre-process Markdown for Decorated mode.
///
/// Currently: inserts a horizontal rule (`---`) after every H1 heading so
/// egui-commonmark renders a visible separator line below it.
/// Code fences are left untouched.
fn preprocess_decorated(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len() + 64);
    let mut in_code = false;
    for line in markdown.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_code = !in_code;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_code {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        out.push_str(line);
        out.push('\n');
        // H1 = exactly one '#' followed by a space or end-of-line.
        let hashes = t.chars().take_while(|&c| c == '#').count();
        if hashes == 1 {
            let rest = &t[1..];
            if rest.is_empty() || rest.starts_with(' ') {
                out.push_str("\n---\n");
            }
        }
    }
    out
}

/// Load raw font bytes for a family name.
///
/// Tries an exact case-insensitive match first. If that fails, strips the
/// trailing word and retries — this handles DE font strings like
/// `"Noto Sans DemiLight"` where fontdb stores the family as `"Noto Sans"`.
fn load_font_data(family_name: &str) -> Option<Vec<u8>> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    load_font_from_db(&db, family_name)
}

fn load_font_from_db(db: &fontdb::Database, family_name: &str) -> Option<Vec<u8>> {
    let mut candidate = family_name;
    loop {
        if let Some(face) = db.faces().find(|f| {
            f.families.first().map_or(false, |(n, _)| n.eq_ignore_ascii_case(candidate))
        }) {
            return db.with_face_data(face.id, |data, _| data.to_vec());
        }
        // Strip last word (e.g. "DemiLight", "Bold", "Italic", "Light") and retry.
        let trimmed = candidate.trim_end();
        let pos = trimmed.rfind(' ')?; // no space left → family not found
        candidate = &trimmed[..pos];
    }
}
