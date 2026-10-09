use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use hunspell_rs::{CheckResult, Hunspell};
use tauri::Manager;
use tauri_plugin_clipboard_manager::ClipboardExt;
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

fn recovery_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("failed to resolve app data dir")
        .join("recovery.json")
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
    fs::write(&path, &json).map_err(|e| format!("Failed to write settings: {e}"))
}

// Recovery store for unsaved/untitled tabs. read takes the file (restore-once,
// i.e. after a crash); write persists current untitled tabs for autosave.
#[tauri::command]
fn read_recovery(app: tauri::AppHandle) -> Result<String, String> {
    let path = recovery_path(&app);
    let data = if path.exists() {
        fs::read_to_string(&path).unwrap_or_default()
    } else {
        String::new()
    };
    let _ = fs::remove_file(&path);
    Ok(data)
}

#[tauri::command]
fn write_recovery(app: tauri::AppHandle, json: String) -> Result<(), String> {
    let path = recovery_path(&app);
    fs::write(&path, &json).map_err(|e| format!("Failed to write recovery: {e}"))
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
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Clipboard reads and writes on Windows are not reliable as one-shot calls.
///
/// Menu paste (right-click -> Paste) cannot use the webview clipboard APIs:
/// `navigator.clipboard.readText()` is denied by WebView2 and
/// `document.execCommand('paste')` is disabled for web content outright, so both
/// are dead ends there. Going through the Rust side instead means going through
/// the Win32 clipboard, which has two failure modes the webview's own Cmd+V path
/// never hits:
///
///   1. Contention - `OpenClipboard` fails while the source app still holds it.
///   2. Delayed rendering - browsers (Chrome, Edge, and therefore WebView2 itself)
///      do not put text on the clipboard when you copy. They register the format
///      and render it on demand via `WM_RENDERFORMAT`, so the first read triggers
///      the render and can legitimately come back EMPTY or error while the browser
///      produces the data. Chromium can take hundreds of milliseconds.
///
/// Case 2 is why pasting something copied from a browser never worked while a
/// Notepad copy did: browsers are the delayed renderers. The first version of this
/// retry gave the clipboard 225 ms in total (15/30/60/120) - far shorter than a
/// busy browser needs - and the failure was swallowed into a silent no-op.
///
/// Neither an empty string nor an error is success: both are retried. The budget is
/// asserted by `retry_budget_outlasts_a_delayed_render` so it cannot be quietly
/// shortened back below the render window.
const CLIPBOARD_ATTEMPTS: u32 = 14;
const CLIPBOARD_FIRST_BACKOFF_MS: u64 = 15;
const CLIPBOARD_MAX_BACKOFF_MS: u64 = 250;

/// Backoff before retry number `attempt` (0-based), capped.
fn clipboard_backoff_ms(attempt: u32) -> u64 {
    (CLIPBOARD_FIRST_BACKOFF_MS << attempt.min(16)).min(CLIPBOARD_MAX_BACKOFF_MS)
}

/// Total ms we are willing to wait across `attempts` tries.
fn clipboard_retry_budget_ms(attempts: u32) -> u64 {
    (0..attempts.saturating_sub(1)).map(clipboard_backoff_ms).sum()
}

/// Run `read` until it yields a non-empty string, retrying with capped backoff.
fn read_clipboard_with_retry<F>(attempts: u32, mut read: F) -> Result<String, String>
where
    F: FnMut() -> Result<String, String>,
{
    let mut last = String::from("clipboard read failed");
    for attempt in 0..attempts {
        match read() {
            // An empty string is NOT success: on Windows a contended or
            // delayed-render read can report success-with-nothing.
            Ok(t) if !t.is_empty() => return Ok(t),
            Ok(_) => last = String::from("clipboard is empty"),
            Err(e) => last = e,
        }
        if attempt + 1 < attempts {
            std::thread::sleep(std::time::Duration::from_millis(clipboard_backoff_ms(attempt)));
        }
    }
    Err(last)
}

/// Run `write` until it succeeds - a copy can lose the same race a paste can.
fn write_clipboard_with_retry<F>(attempts: u32, mut write: F) -> Result<(), String>
where
    F: FnMut() -> Result<(), String>,
{
    let mut last = String::from("clipboard write failed");
    for attempt in 0..attempts {
        match write() {
            Ok(()) => return Ok(()),
            Err(e) => last = e,
        }
        if attempt + 1 < attempts {
            std::thread::sleep(std::time::Duration::from_millis(clipboard_backoff_ms(attempt)));
        }
    }
    Err(last)
}

/// Runs on a blocking thread on purpose: the plugin documents that its read must
/// not run on the main thread (Linux deadlock), and spawn_blocking also contains
/// a panic from a poisoned clipboard mutex instead of taking the app down.
#[tauri::command]
async fn read_clipboard_text(app: tauri::AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        read_clipboard_with_retry(CLIPBOARD_ATTEMPTS, || {
            app.clipboard().read_text().map_err(|e| e.to_string())
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Copy/cut through the same retrying path. A rejected write used to surface as
/// nothing at all, which is indistinguishable from a copy that worked.
#[tauri::command]
async fn write_clipboard_text(app: tauri::AppHandle, text: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        write_clipboard_with_retry(CLIPBOARD_ATTEMPTS, || {
            app.clipboard()
                .write_text(text.clone())
                .map_err(|e| e.to_string())
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod clipboard_retry_tests {
    use super::*;

    #[test]
    fn read_returns_the_first_non_empty_value() {
        let mut calls = 0;
        let got = read_clipboard_with_retry(5, || {
            calls += 1;
            Ok("hello".to_string())
        });
        assert_eq!(got.unwrap(), "hello");
        assert_eq!(calls, 1, "must not retry once a real value arrives");
    }

    #[test]
    fn read_retries_through_empty_then_error_then_success() {
        // The Windows shape: empty (delayed render), error (contention), then data.
        let mut calls = 0;
        let got = read_clipboard_with_retry(6, || {
            calls += 1;
            match calls {
                1 => Ok(String::new()),
                2 => Err("clipboard occupied".to_string()),
                _ => Ok("pasted".to_string()),
            }
        });
        assert_eq!(got.unwrap(), "pasted");
        assert_eq!(calls, 3);
    }

    #[test]
    fn read_reports_the_last_failure_after_the_full_budget() {
        let mut calls = 0;
        let got = read_clipboard_with_retry(3, || {
            calls += 1;
            Err(format!("busy {calls}"))
        });
        assert_eq!(calls, 3, "must try every attempt before giving up");
        assert_eq!(got.unwrap_err(), "busy 3");
    }

    #[test]
    fn write_retries_then_succeeds() {
        let mut calls = 0;
        let got = write_clipboard_with_retry(4, || {
            calls += 1;
            if calls < 2 {
                Err("clipboard busy".to_string())
            } else {
                Ok(())
            }
        });
        assert!(got.is_ok());
        assert_eq!(calls, 2);
    }

    #[test]
    fn backoff_is_capped() {
        assert_eq!(clipboard_backoff_ms(0), 15);
        assert_eq!(clipboard_backoff_ms(1), 30);
        assert_eq!(clipboard_backoff_ms(9), CLIPBOARD_MAX_BACKOFF_MS);
        assert_eq!(clipboard_backoff_ms(15), CLIPBOARD_MAX_BACKOFF_MS);
    }

    #[test]
    fn retry_budget_outlasts_a_delayed_render() {
        let budget = clipboard_retry_budget_ms(CLIPBOARD_ATTEMPTS);
        assert!(
            budget >= 2000,
            "clipboard retry budget is {budget} ms. Browsers render clipboard data on \
             demand and need far longer than the 225 ms that made menu paste fail."
        );
    }
}

#[tauri::command]
fn suggest_spellings(
    word: String,
    lang: String,
    state: tauri::State<'_, Mutex<Option<HashMap<String, SpellChecker>>>>,
) -> Vec<String> {
    let Ok(h) = state.lock() else { return Vec::new() };
    match h.as_ref() {
        Some(map) => match map.get(&lang) {
            Some(c) => c.0.suggest(&word),
            None => map["en_US"].0.suggest(&word),
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

    #[test]
    fn suggest_returns_corrections() {
        let dir = std::env::temp_dir().join("bearpad-spellcheck-test");
        fs::create_dir_all(&dir).unwrap();
        let aff = dir.join("en_US.aff");
        let dic = dir.join("en_US.dic");
        fs::write(&aff, EN_US_AFF).unwrap();
        fs::write(&dic, EN_US_DIC).unwrap();
        let h = Hunspell::new(aff.to_str().unwrap(), dic.to_str().unwrap());

        let suggestions = h.suggest("mispeling");
        assert!(!suggestions.is_empty(), "expected suggestions, got: {suggestions:?}");
        assert!(
            suggestions.iter().any(|s| s == "misspelling" || s == "misspelling" && s.to_lowercase() == "misspelling" || s.to_lowercase() == "misspellings"),
            "expected a plausible correction in {suggestions:?}"
        );
        // unknown garbage yields no suggestions
        assert!(h.suggest("zzqqxxyy").is_empty());
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
                // clean close or confirmed discard: forget any crash recovery
                let _ = fs::remove_file(recovery_path(&window.app_handle()));
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
            read_recovery,
            write_recovery,
            set_dirty,
            spell_check,
            suggest_spellings,
            app_version,
            add_to_dictionary,
            read_clipboard_text,
            write_clipboard_text,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}