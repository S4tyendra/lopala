use axum::{Json, extract::Query, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub glob: Option<String>,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub path: String,
    pub name: String,
    pub kind: String, // "file"
}

pub async fn search_files(Query(q): Query<SearchQuery>) -> impl IntoResponse {
    let query = q.q.trim().to_string();
    if query.is_empty() {
        return (StatusCode::OK, Json(vec![] as Vec<SearchResult>)).into_response();
    }

    let home = std::env::var("HOME").unwrap_or("/home".into());
    let mut cmd = Command::new("rg");
    cmd.arg("--files")
        .arg("--max-depth")
        .arg("6")
        .arg("--iglob")
        .arg(format!("*{}*", query));

    if let Some(glob_pat) = q.glob {
        if !glob_pat.trim().is_empty() && glob_pat != "*" {
            cmd.arg("-g").arg(&glob_pat);
        }
    }

    cmd.arg(&home).stderr(std::process::Stdio::null());

    let results = match cmd.output() {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines()
                .take(30)
                .filter_map(|line| {
                    let p = std::path::Path::new(line);
                    let name = p.file_name()?.to_str()?.to_string();
                    // Case-insensitive filter on the filename
                    if !name.to_lowercase().contains(&query.to_lowercase()) {
                        return None;
                    }
                    Some(SearchResult {
                        path: line.to_string(),
                        name,
                        kind: "file".into(),
                    })
                })
                .collect::<Vec<_>>()
        }
        Err(_) => vec![],
    };

    (StatusCode::OK, Json(results)).into_response()
}
