use eframe::{egui, App, CreationContext, NativeOptions};
use regex::Regex;
use rfd::FileDialog;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use log::{info, error};
use env_logger;

/// Struktur zur Speicherung der extrahierten Track-Informationen.
#[derive(Debug)]
struct TrackInfo {
    index: String,
    titel: String,
    kuenstler: String,
    duration: Option<f64>, // Dauer in Sekunden
    label_code: String,    // Labelcode
}

/// Hauptanwendungsstruktur für GemaLauncherApp.
struct GemaLauncherApp {
    filenames: Vec<String>,
    tracks: Vec<TrackInfo>,
    error_messages: Vec<String>,
    label_dict: HashMap<String, String>, // Labelcode-Liste
}

impl Default for GemaLauncherApp {
    fn default() -> Self {
        Self {
            filenames: Vec::new(),
            tracks: Vec::new(),
            error_messages: Vec::new(),
            label_dict: Self::load_labelcodes("labelcodes.json"),

        }
    }
}

impl App for GemaLauncherApp {
    /// Aktualisierung der GUI.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handhabung von Drag-and-Drop
        let dropped_files = ctx.input(|input| input.raw.dropped_files.clone());

        if !dropped_files.is_empty() {
            for file in dropped_files.iter() {
                if let Some(path_str) = file.path.as_ref().and_then(|p| p.to_str()) {
                    info!("Datei per Drag-and-Drop hinzugefügt: {}", path_str);
                    self.add_file(path_str.to_string());
                }
            }
            self.parse_filenames();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("GEMA_Launcher - Dateinamen Parser zu CSV");

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Dateien auswählen").clicked() {
                    if let Some(files) = FileDialog::new()
                        .add_filter("Audio Dateien und Textdateien", &["wav", "mp3", "txt"])
                        .pick_files()
                    {
                        for file in files {
                            if let Some(path_str) = file.to_str() {
                                self.add_file(path_str.to_string());
                            }
                        }
                        self.parse_filenames();
                    }
                }

                if ui.button("CSV exportieren").clicked() {
                    self.export_csv();
                }
            });

            ui.add_space(20.0);
            ui.separator();

            ui.label("Dateien, die geladen werden:");
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for filename in &self.filenames {
                    ui.label(filename);
                }
            });

            ui.add_space(20.0);
            ui.separator();

            ui.label(format!("Extrahierte Tracks: {}", self.tracks.len()));
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for track in &self.tracks {
                    ui.horizontal(|ui| {
                        ui.label(&track.index);
                        ui.label(&track.titel);
                        ui.label(&track.kuenstler);
                        if let Some(dauer) = track.duration {
                            ui.label(format!("Dauer: {:.2} Sekunden", dauer));
                        }
                        ui.label(&track.label_code);
                    });
                }
            });

            if !self.error_messages.is_empty() {
                ui.add_space(20.0);
                ui.separator();
                ui.colored_label(egui::Color32::RED, "Fehlerhafte Einträge:");
                egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                    for error in &self.error_messages {
                        ui.label(error);
                    }
                });
            }
        });
    }
}


impl GemaLauncherApp {
    /// Lädt die Labelcodes aus einer Datei.
    fn load_labelcodes(labelcodes_file: &str) -> HashMap<String, String> {
        let mut label_dict = HashMap::new();
        if !Path::new(labelcodes_file).exists() {
            info!("Labelcodes-Datei '{}' nicht gefunden.", labelcodes_file);
            return label_dict;
        }
    
        let file = File::open(labelcodes_file).expect("Kann Labelcodes-Datei nicht öffnen.");
        let reader = io::BufReader::new(file);
    
        // JSON-Daten aus Datei lesen
        match serde_json::from_reader(reader) {
            Ok(json_data) => {
                label_dict = json_data;
                info!("Labelcodes erfolgreich geladen: {:?}", label_dict);
            }
            Err(e) => {
                error!("Fehler beim Parsen der Labelcodes-Datei: {}", e);
            }
        }
    
        label_dict
    }
    

    /// Fügt eine Datei zur Liste hinzu, falls sie noch nicht vorhanden ist.
    fn add_file(&mut self, path: String) {
        if !self.filenames.contains(&path) {
            self.filenames.push(path.clone());
            info!("Datei hinzugefügt: {}", path);
        } else {
            info!("Datei bereits in der Liste: {}", path);
        }
    }

    /// Parst die Dateinamen und extrahiert Track-Informationen.
    fn parse_filenames(&mut self) {
        self.tracks.clear();
        self.error_messages.clear();

        // Regex zur Extraktion von Index, Titel und Künstler
        let re = Regex::new(r"^(?P<index>.*?)(?P<titel>[A-Z_]+)_(?P<kuenstler>[^.]+)\.(wav|mp3)$").unwrap();

        // Clone der Dateinamen, um Konflikte zwischen mutable und immutable Borrows zu vermeiden
        let filenames_clone = self.filenames.clone();

        for filename in filenames_clone {
            let path = Path::new(&filename);
            if path.extension().and_then(|s| s.to_str()) == Some("txt") {
                info!("Textdatei erkannt: {}", filename);
                if let Err(e) = self.parse_text_file(&filename) {
                    error!("Fehler beim Parsen der Textdatei {}: {}", filename, e);
                    self.error_messages.push(format!("Fehler beim Parsen der Textdatei {}: {}", filename, e));
                }
                continue;
            }

            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();

            if let Some(caps) = re.captures(&file_name) {
                let index = caps.name("index").map_or("", |m| m.as_str()).to_string();
                let titel = caps.name("titel").map_or("", |m| m.as_str()).to_string();
                let kuenstler = caps.name("kuenstler").map_or("", |m| m.as_str()).to_string();

                let label_code = self.find_label_code(&index); // Verwendung von label_code

                self.tracks.push(TrackInfo {
                    index: index.clone(),
                    titel: titel.clone(),
                    kuenstler: kuenstler.clone(),
                    duration: None, // Dauer kann später hinzugefügt werden
                    label_code: label_code.clone(),
                });
                info!(
                    "Track extrahiert: Index={}, Titel={}, Künstler={}, Labelcode={}",
                    index, titel, kuenstler, label_code
                );
            } else {
                let file_name_str = file_name.clone();
                let error_msg = format!("Unbekanntes Format: {}", file_name_str);
                self.error_messages.push(error_msg.clone());
                error!("{}", error_msg);
            }
        }
    }

    /// Parst eine Textdatei und fügt darin aufgeführte Tracks und Dauern hinzu.
    fn parse_text_file(&mut self, path: &str) -> io::Result<()> {
        let file = File::open(path)?;
        let reader = io::BufReader::new(file);
        let lines = reader.lines().filter_map(Result::ok).collect::<Vec<String>>();

        if lines.is_empty() {
            self.error_messages.push(format!("Leere Textdatei: {}", path));
            return Ok(());
        }

        // Prüfen, ob abwechselndes Format vorliegt (Track, Dauer, Track, Dauer, ...)
        let is_alternating = lines.len() % 2 == 0 && lines.iter().enumerate().all(|(i, line)| {
            if i % 2 == 1 {
                line.contains(':') // Dauer enthält einen Doppelpunkt
            } else {
                true
            }
        });

        let mut track_duration_pairs = Vec::new();

        if is_alternating {
            // Format: Track, Dauer, Track, Dauer, ...
            for i in (0..lines.len()).step_by(2) {
                let track = lines[i].trim().to_string();
                let duration = lines[i + 1].trim().to_string();
                track_duration_pairs.push((track, duration));
            }
        } else {
            // Annahme: Alle Tracks zuerst, dann alle Dauern
            let half = lines.len() / 2;
            let tracks = &lines[..half];
            let durations = &lines[half..];

            if tracks.len() != durations.len() {
                self.error_messages.push(format!(
                    "Die Anzahl der Tracks und Dauern stimmt nicht überein in Datei: {}",
                    path
                ));
                return Ok(());
            }

            for (track, duration) in tracks.iter().zip(durations.iter()) {
                track_duration_pairs.push((track.trim().to_string(), duration.trim().to_string()));
            }
        }

        for (track, duration_str) in track_duration_pairs {
            let (index, titel, kuenstler) = self.parse_track_filename(&track);
            let duration_in_seconds = self.parse_duration(&duration_str);

            if duration_in_seconds.is_none() {
                self.error_messages.push(format!(
                    "Ungültige Dauer '{}' für Track '{}'",
                    duration_str, track
                ));
                error!("Ungültige Dauer '{}' für Track '{}'", duration_str, track);
                continue;
            }

            let duration = duration_in_seconds.unwrap();

            let label_code = self.find_label_code(&index);

            // Suche, ob der Track bereits existiert
            if let Some(existing_track) = self.tracks.iter_mut().find(|t| t.index == index && t.titel == titel && t.kuenstler == kuenstler) {
                existing_track.duration = existing_track.duration.map(|d| d + duration).or(Some(duration));
                info!(
                    "Track aktualisiert: Index={}, Titel={}, Künstler={}, Dauer={}",
                    index, titel, kuenstler, existing_track.duration.unwrap()
                );
            } else {
                self.tracks.push(TrackInfo {
                    index,
                    titel,
                    kuenstler,
                    duration: Some(duration),
                    label_code,
                });
                info!(
                    "Track hinzugefügt: Index={}, Titel={}, Künstler={}, Dauer={}",
                    self.tracks.last().unwrap().index,
                    self.tracks.last().unwrap().titel,
                    self.tracks.last().unwrap().kuenstler,
                    self.tracks.last().unwrap().duration.unwrap()
                );
            }
        }

        Ok(())
    }

    /// Entfernt die Erweiterung von einem Dateinamen.
    fn remove_extension(filename: &str) -> &str {
        filename.split('.').next().unwrap_or("")
    }

    /// Parst den Dateinamen und extrahiert Index, Titel und Künstler.
    fn parse_track_filename(&self, filename: &str) -> (String, String, String) {
        let original_base = Self::remove_extension(filename);
        let base = original_base.replace('_', " ");
        let tokens = base.split_whitespace().collect::<Vec<&str>>();

        fn contains_digit(t: &str) -> bool {
            t.chars().any(|ch| ch.is_digit(10))
        }

        fn is_upper_token(t: &str) -> bool {
            t.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase()) && t.chars().any(|c| c.is_alphabetic())
        }

        let mut state = "BEFORE_DIGIT";
        let mut index_tokens = Vec::new();
        let mut title_tokens = Vec::new();
        let mut artist_tokens = Vec::new();

        for t in tokens {
            match state {
                "BEFORE_DIGIT" => {
                    index_tokens.push(t);
                    if contains_digit(t) {
                        state = "AFTER_DIGIT_BEFORE_TITLE";
                    }
                }
                "AFTER_DIGIT_BEFORE_TITLE" => {
                    if is_upper_token(t) {
                        title_tokens.push(t);
                        state = "TITLE";
                    } else {
                        index_tokens.push(t);
                    }
                }
                "TITLE" => {
                    if is_upper_token(t) {
                        title_tokens.push(t);
                    } else {
                        artist_tokens.push(t);
                        state = "ARTIST";
                    }
                }
                "ARTIST" => {
                    artist_tokens.push(t);
                }
                _ => {}
            }
        }

        let index_str = index_tokens.join("_").to_lowercase();
        let title_str = title_tokens.join(" ").to_lowercase();
        let artist_str = artist_tokens.join(" ").to_lowercase();

        (index_str, title_str, artist_str)
    }

    /// Parst eine Dauer im Format "MM:SS" oder "MM.SS" und gibt sie in Sekunden zurück.
    fn parse_duration(&self, duration_str: &str) -> Option<f64> {
        let duration_str = duration_str.replace(':', ".");
        let parts: Vec<&str> = duration_str.split('.').collect();

        if parts.len() < 2 {
            return None;
        }

        let main_part = parts[0];
        let decimal_part = parts[1];

        let seconds = format!("{}.{}", main_part, decimal_part).parse::<f64>().ok()?;
        Some(seconds)
    }

    /// Formatiert eine Dauer in Sekunden in das Format "S:MM".
    fn format_duration(&self, seconds: f64) -> String {
        let total_hundredths = (seconds * 100.0).round() as i64;
        let s = total_hundredths / 100;
        let ms = total_hundredths % 100;
        format!("{}:{:02}", s, ms)
    }

    /// Findet den Labelcode basierend auf dem Index.
    fn find_label_code(&self, index_str: &str) -> String {
        for (label, code) in &self.label_dict {
            if index_str.to_uppercase().starts_with(label) {
                return code.clone();
            }
        }
        String::new()
    }
    
    /// Exportiert die extrahierten Tracks als CSV-Datei.
    fn export_csv(&mut self) {
        if self.tracks.is_empty() {
            rfd::MessageDialog::new()
                .set_title("Keine Daten")
                .set_description("Es gibt keine Daten zum Exportieren.")
                .set_buttons(rfd::MessageButtons::Ok)
                .show();
            return;
        }
    
        if let Some(input_file) = self.filenames.first() {
            let path = Path::new(input_file);
            let base_name = path.file_stem().unwrap_or_default().to_str().unwrap_or("");
            let formatted_name = format!("{}_formatted.csv", base_name);
    
            if let Some(file) = FileDialog::new().set_file_name(&formatted_name).save_file() {
                match File::create(&file) {
                    Ok(f) => {
                        let mut wtr = csv::Writer::from_writer(f);
                        // Schreiben der Header
                        if let Err(e) = wtr.write_record(&["Index", "Titel", "Künstler", "Dauer", "Labelcode"]) {
                            let error_msg = format!("CSV-Fehler: {}", e);
                            self.error_messages.push(error_msg.clone());
                            error!("{}", error_msg);
                            return;
                        }
                        // Schreiben der Daten
                        for track in &self.tracks {
                            if let Err(e) = wtr.write_record(&[
                                &track.index,
                                &track.titel,
                                &track.kuenstler,
                                &track.duration.map_or(String::new(), |d| self.format_duration(d)),
                                &track.label_code,
                            ]) {
                                let error_msg = format!("CSV-Fehler: {}", e);
                                self.error_messages.push(error_msg.clone());
                                error!("{}", error_msg);
                                return;
                            }
                        }
                        if let Err(e) = wtr.flush() {
                            let error_msg = format!("CSV-Fehler beim Flush: {}", e);
                            self.error_messages.push(error_msg.clone());
                            error!("{}", error_msg);
                            return;
                        }
                        rfd::MessageDialog::new()
                            .set_title("Erfolg")
                            .set_description(format!(
                                "CSV erfolgreich exportiert nach:\n{}",
                                file.display()
                            ))
                            .set_buttons(rfd::MessageButtons::Ok)
                            .show();
                        info!("CSV erfolgreich exportiert nach {}", file.display());
                    }
                    Err(e) => {
                        let error_msg = format!("Datei-Fehler: {}", e);
                        self.error_messages.push(error_msg.clone());
                        error!("{}", error_msg);
                    }
                }
            }
        } else {
            rfd::MessageDialog::new()
                .set_title("Fehler")
                .set_description("Keine Eingabedatei gefunden.")
                .set_buttons(rfd::MessageButtons::Ok)
                .show();
        }
    }
    
}

fn main() {
    // Initialisiere das Logging
    env_logger::init();
    info!("GEMA_Launcher startet");

    let native_options = NativeOptions::default();
    eframe::run_native(
        "GEMA_Launcher", // App-Name
        native_options,
        Box::new(|_cc: &CreationContext| Box::new(GemaLauncherApp::default()) as Box<dyn App>),
    ).expect("GEMA_Launcher konnte nicht gestartet werden");
}
