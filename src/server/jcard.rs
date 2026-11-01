//! Cassette artwork is stored independently from audio/library metadata.
use super::state::{Api, App, Error};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    routing::get,
};
use serde_json::Value;

pub(super) fn routes() -> Router<App> {
    Router::new()
        .route("/api/jcards/{id}", get(load).put(save))
        .layer(DefaultBodyLimit::max(12 * 1024 * 1024))
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 100
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}
fn number(v: &Value, key: &str, min: f64, max: f64) -> bool {
    v[key]
        .as_f64()
        .is_some_and(|n| n.is_finite() && (min..=max).contains(&n))
}
fn validate(v: &Value) -> Result<(), String> {
    if !matches!(v["version"].as_u64(), Some(1 | 2))
        || !number(v, "height", 50., 200.)
        || !number(v, "front", 30., 100.)
        || !number(v, "spine", 5., 30.)
        || !number(v, "flap", 0., 100.)
    {
        return Err("J-CARDの寸法または形式が不正です".into());
    }
    if v["version"] == 2 {
        let panels = v["extensions"]
            .as_array()
            .ok_or("J-CARDの折り返し設定が不正です")?;
        if panels.len() > 30
            || panels
                .iter()
                .any(|n| n.as_f64().is_none_or(|w| !(10.0..=100.0).contains(&w)))
            || !number(v, "margin", 0., 20.)
        {
            return Err("J-CARDの折り返し設定が不正です".into());
        }
        let (pw, ph) = match v["paper"].as_str() {
            Some("a4") => (210., 297.),
            Some("letter") => (215.9, 279.4),
            _ => return Err("J-CARDの用紙設定が不正です".into()),
        };
        let margin = v["margin"].as_f64().unwrap();
        let w = v["front"].as_f64().unwrap()
            + v["spine"].as_f64().unwrap()
            + v["flap"].as_f64().unwrap()
            + panels.iter().map(|p| p.as_f64().unwrap()).sum::<f64>()
            + 10.
            + margin * 2.;
        let h = v["height"].as_f64().unwrap() + 10. + margin * 2.;
        if !((w <= pw + 1e-8 && h <= ph + 1e-8) || (w <= ph + 1e-8 && h <= pw + 1e-8)) {
            return Err("選択した用紙に実寸で収まりません".into());
        }
        validate_elements(&v["backElements"])?;
    }
    validate_elements(&v["elements"])
}
fn validate_elements(value: &Value) -> Result<(), String> {
    let elements = value.as_array().ok_or("J-CARDの要素が不正です")?;
    if elements.len() > 150 {
        return Err("J-CARDの要素は150個までです".into());
    }
    let mut ids = std::collections::HashSet::new();
    for e in elements {
        let id = e["id"].as_str().ok_or("J-CARDの要素IDが不正です")?;
        if !valid_id(id)
            || !ids.insert(id)
            || !matches!(e["kind"].as_str(), Some("text" | "rect" | "image"))
            || !number(e, "x", -300., 300.)
            || !number(e, "y", -300., 300.)
            || !number(e, "w", 1., 300.)
            || !number(e, "h", 1., 300.)
            || !number(e, "rotation", -360., 360.)
        {
            return Err("J-CARDの要素が不正です".into());
        }
        if e["kind"] == "text"
            && (e["text"].as_str().is_none_or(|s| s.len() > 60000)
                || !number(e, "size", 4., 100.)
                || !number(e, "lineHeight", 0.8, 3.))
        {
            return Err("J-CARDの文字設定が不正です".into());
        }
        if e["kind"] == "image" {
            let src = e["src"].as_str().ok_or("J-CARDの画像が不正です")?;
            let data = src
                .strip_prefix("data:image/png;base64,")
                .or_else(|| src.strip_prefix("data:image/jpeg;base64,"));
            if data.is_none_or(|s| {
                s.len() > 8_000_000
                    || s.is_empty()
                    || !s
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"+/=".contains(&c))
            }) {
                return Err("J-CARDの画像はPNG・JPEGを使用してください".into());
            }
        }
    }
    Ok(())
}
fn check_cassette(app: &App, id: &str) -> Result<(), Error> {
    if !valid_id(id)
        || !app
            .snapshot()?
            .collections
            .iter()
            .any(|c| c.id == id && c.kind == "cassette")
    {
        return Err(Error("カセットが見つかりません".into()));
    }
    Ok(())
}
async fn load(State(app): State<App>, Path(id): Path<String>) -> Api<Value> {
    check_cassette(&app, &id)?;
    let path = app.root.join("jcards").join(format!("{id}.json"));
    match std::fs::read(path) {
        Ok(bytes) => Ok(Json(
            serde_json::from_slice(&bytes).map_err(|e| Error(e.to_string()))?,
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Json(Value::Null)),
        Err(e) => Err(Error(e.to_string())),
    }
}
async fn save(
    State(app): State<App>,
    Path(id): Path<String>,
    Json(value): Json<Value>,
) -> Api<Value> {
    validate(&value)?;
    // Serialize saves/deletions and use a unique temporary file for atomic replacement.
    let _guard = app.imports.lock().await;
    check_cassette(&app, &id)?;
    let folder = app.root.join("jcards");
    std::fs::create_dir_all(&folder).map_err(|e| Error(e.to_string()))?;
    let tmp = folder.join(format!("{}.tmp", uuid::Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(&value).map_err(|e| Error(e.to_string()))?;
    std::fs::write(&tmp, bytes).map_err(|e| Error(e.to_string()))?;
    if let Err(e) = std::fs::rename(&tmp, folder.join(format!("{id}.json"))) {
        let _ = std::fs::remove_file(tmp);
        return Err(Error(e.to_string()));
    }
    Ok(Json(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_bounds_and_embedded_images() {
        let mut v = serde_json::json!({"version":1,"height":100,"front":65,"spine":12,"flap":25,"elements":[]});
        assert!(validate(&v).is_ok());
        v["height"] = serde_json::json!(-1);
        assert!(validate(&v).is_err());
        assert!(!valid_id("../library"));
        v["height"] = serde_json::json!(100);
        v["elements"] = serde_json::json!([{"id":"image","kind":"image","x":0,"y":0,"w":10,"h":10,"rotation":0,"src":"https://example.com/image.png"}]);
        assert!(validate(&v).is_err());
    }
}
