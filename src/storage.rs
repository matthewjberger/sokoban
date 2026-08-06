#[cfg(not(target_arch = "wasm32"))]
use crate::schema::MIN_EXTENT;
use crate::schema::Map;
use nightshade::prelude::serde_json;

/// Where authored maps live on a desktop build. Ignored by git, because these
/// are the player's own puzzles rather than shipped content.
#[cfg(not(target_arch = "wasm32"))]
pub const MAP_DIRECTORY: &str = "local_maps";

pub fn to_json(map: &Map) -> String {
    serde_json::to_string_pretty(map).unwrap_or_default()
}

/// Reads a map back out of the exchange format. Only the desktop build has
/// somewhere to read one from, and a browser build hands files out rather than
/// taking them in.
#[cfg(not(target_arch = "wasm32"))]
pub fn from_json(text: &str) -> Result<Map, String> {
    let map: Map = serde_json::from_str(text).map_err(|error| error.to_string())?;
    // A file can say anything. Floors divide the coordinate space, so a
    // nonsense size would divide by zero the moment anything read a square.
    if map.floor_width < MIN_EXTENT || map.floor_height < MIN_EXTENT || map.floors.is_empty() {
        return Err("that file is not shaped like a map".to_string());
    }
    Ok(map)
}

pub fn file_stem(map: &Map) -> String {
    let stem: String = map
        .name
        .trim()
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect();
    if stem.is_empty() {
        "untitled".to_string()
    } else {
        stem
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn copy_to_clipboard(map: &Map) -> Result<String, String> {
    if nightshade::platform::host::write_clipboard(&to_json(map)) {
        Ok("copied json to the clipboard".to_string())
    } else {
        Err("the system clipboard refused the write".to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save(map: &Map) -> Result<String, String> {
    std::fs::create_dir_all(MAP_DIRECTORY).map_err(|error| error.to_string())?;
    let path = format!("{MAP_DIRECTORY}/{}.json", file_stem(map));
    std::fs::write(&path, to_json(map)).map_err(|error| error.to_string())?;
    Ok(path)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn list() -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(MAP_DIRECTORY) else {
        return Vec::new();
    };
    let mut stems: Vec<String> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension()?.to_str()? != "json" {
                return None;
            }
            Some(path.file_stem()?.to_str()?.to_string())
        })
        .collect();
    stems.sort();
    stems
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load(stem: &str) -> Result<Map, String> {
    let path = format!("{MAP_DIRECTORY}/{stem}.json");
    let text = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    from_json(&text)
}

/// The page owns the clipboard in a browser build, so the write goes through
/// the navigator rather than the platform layer.
#[cfg(target_arch = "wasm32")]
pub fn copy_to_clipboard(map: &Map) -> Result<String, String> {
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    drop(window.navigator().clipboard().write_text(&to_json(map)));
    Ok("copied json to the clipboard".to_string())
}

/// A browser build cannot choose a path, so saving hands the file to the
/// user's own download prompt.
#[cfg(target_arch = "wasm32")]
pub fn save(map: &Map) -> Result<String, String> {
    use wasm_bindgen::JsCast;

    let name = format!("{}.json", file_stem(map));
    let parts = js_sys::Array::new();
    parts.push(&wasm_bindgen::JsValue::from_str(&to_json(map)));
    let blob = web_sys::Blob::new_with_str_sequence(&parts)
        .map_err(|_| "could not build the file".to_string())?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|_| "could not publish the file".to_string())?;
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| "no document".to_string())?;
    let anchor = document
        .create_element("a")
        .map_err(|_| "could not build the download".to_string())?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "could not build the download".to_string())?;
    anchor.set_href(&url);
    anchor.set_download(&name);
    anchor.click();
    drop(web_sys::Url::revoke_object_url(&url));
    Ok(name)
}

#[cfg(target_arch = "wasm32")]
pub fn list() -> Vec<String> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
pub fn load(_stem: &str) -> Result<Map, String> {
    Err("a browser build cannot read local files".to_string())
}
