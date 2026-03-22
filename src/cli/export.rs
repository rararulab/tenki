use snafu::ResultExt as _;

use crate::{
    db::Database,
    error::{self, Result},
};

fn default_filename(company: &str, position: &str, ext: &str) -> String {
    let name = format!("{company}-{position}")
        .to_lowercase()
        .replace(' ', "-");
    format!("{name}.{ext}")
}

pub async fn export(
    db: &Database,
    id: &str,
    typ: bool,
    pdf: bool,
    output: Option<&str>,
    json: bool,
) -> Result<()> {
    let full_id = db.resolve_app_id(id).await?;
    let app = db.get_application(&full_id).await?;

    if typ {
        let content = db.get_resume_typ(&full_id).await?;
        match content {
            Some(src) => {
                let path = output.map_or_else(
                    || default_filename(&app.company, &app.position, "typ"),
                    String::from,
                );
                std::fs::write(&path, &src).context(error::IoSnafu)?;
                if json {
                    let out = serde_json::json!({ "exported": path });
                    println!("{}", serde_json::to_string(&out).context(error::JsonSnafu)?);
                } else {
                    eprintln!("Exported typ to {path}");
                }
            }
            None => {
                eprintln!("No resume typ found for application {}", &full_id[..8]);
            }
        }
    }

    if pdf {
        let content = db.get_resume_pdf(&full_id).await?;
        match content {
            Some(bytes) => {
                let path = output.map_or_else(
                    || default_filename(&app.company, &app.position, "pdf"),
                    String::from,
                );
                std::fs::write(&path, &bytes).context(error::IoSnafu)?;
                if json {
                    let out = serde_json::json!({ "exported": path });
                    println!("{}", serde_json::to_string(&out).context(error::JsonSnafu)?);
                } else {
                    eprintln!("Exported pdf to {path}");
                }
            }
            None => {
                eprintln!("No resume pdf found for application {}", &full_id[..8]);
            }
        }
    }

    if !typ && !pdf {
        eprintln!("Specify --typ and/or --pdf");
    }

    Ok(())
}

pub async fn import(db: &Database, id: &str, typ_path: &str, json: bool) -> Result<()> {
    let full_id = db.resolve_app_id(id).await?;
    let content = std::fs::read_to_string(typ_path).context(error::IoSnafu)?;

    sqlx::query(
        "UPDATE applications SET resume_typ = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
    )
    .bind(&content)
    .bind(&full_id)
    .execute(db.pool())
    .await
    .context(error::SqlxSnafu)?;

    if json {
        let out = serde_json::json!({ "imported": full_id });
        println!("{}", serde_json::to_string(&out).context(error::JsonSnafu)?);
    } else {
        eprintln!("Imported typ for application {}", &full_id[..8]);
    }
    Ok(())
}
