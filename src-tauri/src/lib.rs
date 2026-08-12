use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use hunspell_rs::{CheckResult, Hunspell};
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

const EN_US_AFF: &[u8] = include_bytes!("../dicts/en_US.aff");
const EN_US_DIC: &[u8] = include_bytes!("../dicts/en_US.dic");
const EN_CA_AFF: &[u8] = include_bytes!("../dicts/en_CA.aff");
const EN_CA_DIC: &[u8] = include_bytes!("../dicts/en_CA.dic");
const EN_GB_AFF: &[u8] = include_bytes!("../dicts/en_GB.aff");
const EN_GB_DIC: &[u8] = include_bytes!("../dicts/en_GB.dic");
const FR_CA_AFF: &[u8] = include_bytes!("../dicts/fr_CA.aff");
const FR_CA_DIC: &[u8] = include_bytes!("../dicts/fr_CA.dic");
const ES_ES_AFF: &[u8] = include_bytes!("../dicts/es_ES.aff");
const ES_ES_DIC: &[u8] = include_bytes!("../dicts/es_ES.dic");

const LANGUAGES: [(&str, &[u8], &[u8]); 5] = [
    ("en_US", EN_US_AFF, EN_US_DIC),
    ("en_CA", EN_CA_AFF, EN_CA_DIC),
    ("en_GB", EN_GB_AFF, EN_GB_DIC),
    ("fr_CA", FR_CA_AFF, FR_CA_DIC),
    ("es_ES", ES_ES_AFF, ES_ES_DIC),
];

fn settings_path(app: &tauri::AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .expect("failed to resolve app data dir");
    fs::create_dir_all(&dir).ok();
    dir.join("settings.json")
}

#[tauri::command]
fn set_dirty(dirty: bool, state: tauri::State<'_, Mutex<bool>>) {
    *state.lock().unwrap() = dirty;
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, &content).map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
fn read_settings(app: tauri::AppHandle) -> Result<String, String> {
    let path = settings_path(&app);
    if path.exists() {
        fs::read_to_string(&path).map_err(|e| format!("Failed to read settings: {}", e))
    } else {
        Ok("{}".to_string())
    }
}

#[tauri::command]
fn write_settings(app: tauri::AppHandle, json: String) -> Result<(), String> {
    let path = settings_path(&app);
    fs::write(&path, &json).map_err(|e| format!("Failed to write settings: {}", e))
}

#[derive(serde::Serialize)]
struct Misspelling {
    start: usize,
    end: usize,
    word: String,
}

// Hunspell's C handle is !Send/!Sync; every use is serialized through a Mutex,
// so marking the wrapper Send+Sync is sound.
struct SpellChecker(Hunspell);
unsafe impl Send for SpellChecker {}
unsafe impl Sync for SpellChecker {}

// Scans markdown text, skipping fenced code blocks, inline code spans, and
// URLs; returns UTF-16 code-unit offsets (CodeMirror positions) of words the
// dictionary doesn't know.
fn spell_check_text(text: &str, h: &Hunspell) -> Vec<Misspelling> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = Vec::new();
    let mut u16 = 0usize;
    let mut i = 0usize;
    let mut in_fence = false;

    fn line_after_is_fence(chars: &[char], i: usize) -> bool {
        let mut j = i;
        while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
            j += 1;
        }
        j + 2 < chars.len()
            && ((chars[j] == '`' && chars[j + 1] == '`' && chars[j + 2] == '`')
                || (chars[j] == '~' && chars[j + 1] == '~' && chars[j + 2] == '~'))
    }

    while i < n {
        let c = chars[i];
        if in_fence {
            if c == '\n' && line_after_is_fence(&chars, i + 1) {
                // consume the closing fence line so its backticks don't fall
                // through to the inline-code path
                u16 += 1;
                i += 1;
                while i < n && chars[i] != '\n' {
                    u16 += chars[i].len_utf16();
                    i += 1;
                }
                if i < n {
                    u16 += 1;
                    i += 1;
                }
                in_fence = false;
                continue;
            }
            u16 += c.len_utf16();
            i += 1;
            continue;
        }
        if (i == 0 || c == '\n') && line_after_is_fence(&chars, if c == '\n' { i + 1 } else { i }) {
            in_fence = true;
            u16 += c.len_utf16();
            i += 1;
            continue;
        }
        if c == '`' {
            u16 += 1;
            i += 1;
            while i < n && chars[i] != '`' {
                u16 += chars[i].len_utf16();
                i += 1;
            }
            if i < n {
                u16 += 1;
                i += 1;
            }
            continue;
        }
        if (c == 'h' || c == 'H' || c == 'w' || c == 'W')
            && i + 2 < n
            && (chars[i..i + 3].iter().collect::<String>().to_ascii_lowercase() == "htt"
                || chars[i..i + 3].iter().collect::<String>().to_ascii_lowercase() == "www")
        {
            // skip rest of the URL token (until whitespace)
            while i < n && !chars[i].is_whitespace() {
                u16 += chars[i].len_utf16();
                i += 1;
            }
            continue;
        }
        if c.is_ascii_alphabetic() {
            let start = u16;
            let mut j = i;
            while j < n
                && (chars[j].is_ascii_alphabetic()
                    || (chars[j] == '\'' && j + 1 < n && chars[j + 1].is_ascii_alphabetic()))
            {
                u16 += chars[j].len_utf16();
                j += 1;
            }
            let word: String = chars[i..j].iter().collect();
            if word.len() >= 2 && h.check(&word) == CheckResult::MissingInDictionary {
                out.push(Misspelling { start, end: u16, word });
            }
            i = j;
            continue;
        }
        u16 += c.len_utf16();
        i += 1;
    }
    out
}

#[tauri::command]
fn spell_check(
    text: String,
    lang: String,
    state: tauri::State<'_, Mutex<Option<HashMap<String, SpellChecker>>>>,
) -> Vec<Misspelling> {
    // ponytail: full-doc recheck on every change, debounced in JS; switch to
    // dirty-range checking if large files lag.
    let Ok(h) = state.lock() else { return Vec::new() };
    match h.as_ref() {
        Some(map) => match map.get(&lang) {
            Some(c) => spell_check_text(&text, &c.0),
            None => spell_check_text(&text, &map["en_US"].0),
        },
        None => Vec::new(),
    }
}

#[tauri::command]
fn add_to_dictionary(
    word: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<Option<HashMap<String, SpellChecker>>>>,
) -> Result<(), String> {
    let word = word.trim();
    if word.is_empty() {
        return Ok(());
    }
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let custom = dir.join("custom_words.txt");
    let existing = fs::read_to_string(&custom).unwrap_or_default();
    if !existing.lines().any(|l| l == word) {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&custom)
            .map_err(|e| e.to_string())?;
        use std::io::Write;
        writeln!(f, "{word}").map_err(|e| e.to_string())?;
    }
    if let Ok(mut h) = state.lock() {
        if let Some(map) = h.as_mut() {
            for c in map.values_mut() {
                c.0.add(&word);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checker_flags_misspellings_skips_code() {
        let dir = std::env::temp_dir().join("bearpad-spellcheck-test");
        fs::create_dir_all(&dir).unwrap();
        let aff = dir.join("en_US.aff");
        let dic = dir.join("en_US.dic");
        fs::write(&aff, EN_US_AFF).unwrap();
        fs::write(&dic, EN_US_DIC).unwrap();
        let h = Hunspell::new(aff.to_str().unwrap(), dic.to_str().unwrap());

        let text = "This is a correct sentence with a mispeling word.\n\
                    ```rust\nlet worsd = 1;\n```\n\
                    inline `worsd` here and https://example.com/worsd too.";
        let hits = spell_check_text(text, &h);
        let words: Vec<&str> = hits.iter().map(|m| m.word.as_str()).collect();
        assert_eq!(words, vec!["mispeling"], "got: {:?}", words);
        // verify offsets point at the actual word in the source
        let m = &hits[0];
        assert_eq!(&text[m.start..m.end], "mispeling");
        assert!(h.check("correct") == CheckResult::FoundInDictionary);
        assert!(h.check("zzqqxxyy") == CheckResult::MissingInDictionary);
        // user-added words flip the result (custom dictionary path)
        let mut h = h;
        h.add("zzqqxxyy");
        assert!(h.check("zzqqxxyy") == CheckResult::FoundInDictionary);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(false))
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let dict_dir = dir.join("dicts");
            fs::create_dir_all(&dict_dir)?;
            let mut checkers = HashMap::new();
            for (lang, aff, dic) in LANGUAGES {
                let aff_name = format!("{lang}.aff");
                let dic_name = format!("{lang}.dic");
                fs::write(dict_dir.join(&aff_name), aff)?;
                fs::write(dict_dir.join(&dic_name), dic)?;
                let h = Hunspell::new(
                    dict_dir.join(&aff_name).to_str().unwrap(),
                    dict_dir.join(&dic_name).to_str().unwrap(),
                );
                checkers.insert(lang.to_string(), SpellChecker(h));
            }
            // load persisted user-added words into every checker
            if let Ok(words) = fs::read_to_string(dir.join("custom_words.txt")) {
                for c in checkers.values_mut() {
                    for w in words.lines() {
                        c.0.add(w);
                    }
                }
            }
            app.manage(Mutex::new(Some(checkers)));
            Ok(())
        })
        .on_window_event(|window, event| {
            // Close confirmation runs natively: the JS dialog path (async listener
            // + preventDefault/destroy) hangs on Windows in every variant. Here the
            // close is prevented, a native dialog asks, and destroy() closes for
            // real on confirm. Dirty state is synced from the webview via set_dirty.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let dirty = *window.state::<Mutex<bool>>().lock().unwrap();
                if !dirty {
                    return; // clean document: default close proceeds
                }
                api.prevent_close();
                let win = window.clone();
                window
                    .dialog()
                    .message("You have unsaved changes. Discard and close?")
                    .title("BearPad")
                    .kind(MessageDialogKind::Warning)
                    .buttons(MessageDialogButtons::OkCancel)
                    .show(move |ok| {
                        if ok {
                            let _ = win.destroy();
                        }
                    });
            }
        })
        .invoke_handler(tauri::generate_handler![
            read_file,
            write_file,
            read_settings,
            write_settings,
            set_dirty,
            spell_check,
            add_to_dictionary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}