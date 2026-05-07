use std::sync::Arc;

use anyhow::Context;
use serde::Serialize;

use super::*;

#[derive(clap::Args)]
pub struct CommandDocument {
    #[arg(short, long, default_value = "false")]
    force: bool,

    #[command(subcommand)]
    command: DocumentCommands,

    #[arg(long = "markdown", visible_alias = "md", alias = "json", default_value = "false")]
    json: bool,

    #[arg(long, default_value = "")]
    otp_code: String,
}

#[derive(Subcommand)]
enum DocumentCommands {
    #[command(visible_alias("ls"))]
    List {
        #[arg(long, default_value = "false")]
        all_term: bool,
    },
    Show {
        id: String,
        #[arg(long, default_value = "false")]
        all_term: bool,
    },
    #[command(visible_alias("down"))]
    Download {
        id: Option<String>,
        #[arg(short, long, default_value = ".")]
        dir: std::path::PathBuf,
        #[arg(long, default_value = "false")]
        all_term: bool,
    },
}

#[derive(Serialize)]
struct DocumentListRecord {
    course_title: String,
    id: String,
    title: String,
    attachment_count: usize,
    description_count: usize,
}

#[derive(Serialize)]
struct DocumentActionRecord {
    action: String,
    course_title: String,
    id: String,
    title: String,
    path: String,
}

#[derive(Serialize)]
struct DocumentAttachmentRecord {
    name: String,
    url: String,
}

#[derive(Serialize)]
struct DocumentDetailRecord {
    course_title: String,
    id: String,
    title: String,
    descriptions: Vec<String>,
    attachments: Vec<DocumentAttachmentRecord>,
}

fn sort_document_records(records: &mut [DocumentListRecord]) {
    records.sort_by(|a, b| {
        a.title
            .cmp(&b.title)
            .then_with(|| a.course_title.cmp(&b.course_title))
            .then_with(|| a.id.cmp(&b.id))
    });
}

pub async fn run(cmd: CommandDocument) -> anyhow::Result<()> {
    match cmd.command {
        DocumentCommands::List { all_term } => {
            list(cmd.force, !all_term, cmd.otp_code, cmd.json).await?
        }
        DocumentCommands::Show { id, all_term } => {
            show(&id, cmd.force, !all_term, cmd.otp_code, cmd.json).await?
        }
        DocumentCommands::Download { id, dir, all_term } => {
            download(
                id.as_deref(),
                &dir,
                cmd.force,
                !all_term,
                cmd.otp_code,
                cmd.json,
            )
            .await?
        }
    }
    Ok(())
}

type DocumentListItem = (Arc<Course>, String, CourseDocumentHandle);

async fn get_documents(
    c: &Course,
    pb: indicatif::ProgressBar,
) -> anyhow::Result<Vec<CourseDocumentHandle>> {
    let r = super::cmd_assignment::get_contents(c, pb)
        .await?
        .into_iter()
        .filter_map(|c| c.into_document_opt())
        .collect();
    Ok(r)
}

async fn fetch_documents(
    force: bool,
    cur_term: bool,
    otp_code: String,
) -> anyhow::Result<Vec<DocumentListItem>> {
    let courses = load_courses(force, cur_term, otp_code).await?;

    let m = indicatif::MultiProgress::new();
    let pb = m.add(pbar::new(courses.len() as u64)).with_prefix("All");
    let futs = courses.into_iter().map(async |c| -> anyhow::Result<_> {
        let c = Arc::new(c.get().await.context("fetch course")?);
        let docs = get_documents(
            &c,
            m.add(pbar::new(0).with_prefix(c.meta().name().to_owned())),
        )
        .await
        .with_context(|| format!("fetch documents of {}", c.meta().title()))?;

        pb.inc_length(docs.len() as u64);
        let docs = docs
            .into_iter()
            .map(|d| {
                pb.inc(1);
                (c.clone(), d.id(), d)
            })
            .collect::<Vec<_>>();
        pb.inc(1);
        Ok(docs)
    });
    let courses = try_join_all(futs).await?;
    pb.finish_and_clear();
    m.clear().unwrap();
    drop(pb);
    drop(m);

    let mut docs = courses.into_iter().flatten().collect::<Vec<_>>();
    docs.sort_by(|a, b| a.2.title().cmp(b.2.title()));
    Ok(docs)
}

async fn select_document(mut items: Vec<DocumentListItem>) -> anyhow::Result<DocumentListItem> {
    if items.is_empty() {
        anyhow::bail!("documents not found");
    }
    let options = items
        .iter()
        .enumerate()
        .map(|(idx, (c, id, d))| {
            format!("[{}] {} > {} {}", idx + 1, c.meta().name(), d.title(), id)
        })
        .collect::<Vec<_>>();
    let s = inquire::Select::new("请选择要下载的文档", options).raw_prompt()?;
    Ok(items.swap_remove(s.index))
}

pub async fn list(force: bool, cur_term: bool, otp_code: String, json: bool) -> anyhow::Result<()> {
    let items = fetch_documents(force, cur_term, otp_code).await?;
    if json {
        let mut items = items
            .into_iter()
            .map(|(c, id, d)| DocumentListRecord {
                course_title: c.meta().name().to_owned(),
                id,
                title: d.title().to_owned(),
                attachment_count: d.attachments().len(),
                description_count: d.descriptions().len(),
            })
            .collect::<Vec<_>>();
        sort_document_records(&mut items);
        return markdown_output::write_markdown(&markdown_output::ok_items(items)).await;
    }

    let mut outbuf = Vec::new();
    writeln!(outbuf, "{D}>{D:#} {B}课程文档{B:#} {D}<{D:#}\n")?;
    for (course, id, doc) in items {
        writeln!(
            outbuf,
            "{BL}{B}{}{B:#}{BL:#} {D}>{D:#} {BL}{B}{}{B:#}{BL:#} ({GR}{} 个附件{GR:#}) {D}{}{D:#}",
            course.meta().name(),
            doc.title(),
            doc.attachments().len(),
            id
        )?;
    }
    buf_try!(@try fs::stdout().write_all(outbuf).await);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_document_records_uses_tie_breakers() {
        let mut records = vec![
            DocumentListRecord {
                course_title: "B".to_owned(),
                id: "2".to_owned(),
                title: "Week 1".to_owned(),
                attachment_count: 0,
                description_count: 0,
            },
            DocumentListRecord {
                course_title: "A".to_owned(),
                id: "1".to_owned(),
                title: "Week 1".to_owned(),
                attachment_count: 0,
                description_count: 0,
            },
        ];

        sort_document_records(&mut records);

        assert_eq!(records[0].course_title, "A");
        assert_eq!(records[1].course_title, "B");
    }

    #[test]
    fn document_detail_record_serializes_attachments() {
        let value = serde_json::to_value(DocumentDetailRecord {
            course_title: "Course".to_owned(),
            id: "doc1".to_owned(),
            title: "Week 1".to_owned(),
            descriptions: vec!["desc".to_owned()],
            attachments: vec![DocumentAttachmentRecord {
                name: "file.pdf".to_owned(),
                url: "https://example.com/file.pdf".to_owned(),
            }],
        })
        .unwrap();

        assert_eq!(value["attachments"][0]["name"], "file.pdf");
        assert_eq!(
            value["attachments"][0]["url"],
            "https://example.com/file.pdf"
        );
    }

    #[test]
    fn document_action_record_serializes_path() {
        let value = serde_json::to_value(DocumentActionRecord {
            action: "document.download".to_owned(),
            course_title: "Course".to_owned(),
            id: "doc1".to_owned(),
            title: "Week 1".to_owned(),
            path: "/tmp/doc".to_owned(),
        })
        .unwrap();

        assert_eq!(value["action"], "document.download");
        assert_eq!(value["path"], "/tmp/doc");
    }

    #[test]
    fn document_detail_payload_fits_json_envelope() {
        let detail = DocumentDetailRecord {
            course_title: "Course".to_owned(),
            id: "doc1".to_owned(),
            title: "Week 1".to_owned(),
            descriptions: vec!["desc".to_owned()],
            attachments: vec![DocumentAttachmentRecord {
                name: "file.pdf".to_owned(),
                url: "https://example.com/file.pdf".to_owned(),
            }],
        };
        let value = serde_json::to_value(markdown_output::ok_item(detail)).unwrap();

        assert_eq!(value["schema_version"], "1");
        assert_eq!(value["ok"], true);
        assert_eq!(value["item"]["attachments"][0]["name"], "file.pdf");
    }
}

pub async fn show(
    id: &str,
    force: bool,
    cur_term: bool,
    otp_code: String,
    json: bool,
) -> anyhow::Result<()> {
    let items = fetch_documents(force, cur_term, otp_code).await?;
    let item = items
        .into_iter()
        .find(|x| x.1 == id)
        .with_context(|| format!("document with id {} not found", id))?;

    if json {
        let detail = DocumentDetailRecord {
            course_title: item.0.meta().name().to_owned(),
            id: item.1,
            title: item.2.title().to_owned(),
            descriptions: item.2.descriptions().to_vec(),
            attachments: item
                .2
                .attachments()
                .iter()
                .map(|(name, url)| DocumentAttachmentRecord {
                    name: name.to_string(),
                    url: url.to_string(),
                })
                .collect(),
        };
        return markdown_output::write_markdown(&markdown_output::ok_item(detail)).await;
    }

    println!(
        "{BL}{B}{}{B:#}{BL:#} {D}>{D:#} {BL}{B}{}{B:#}{BL:#}",
        item.0.meta().name(),
        item.2.title()
    );
    println!("{D}ID:{D:#} {}", item.1);
    if !item.2.descriptions().is_empty() {
        println!();
        for line in item.2.descriptions() {
            println!("{line}");
        }
    }
    if !item.2.attachments().is_empty() {
        println!();
        for (name, _) in item.2.attachments() {
            println!("{D}[附件]{D:#} {UL}{name}{UL:#}");
        }
    }
    Ok(())
}

pub async fn download(
    id: Option<&str>,
    dir: &std::path::Path,
    force: bool,
    cur_term: bool,
    otp_code: String,
    json: bool,
) -> anyhow::Result<()> {
    let items = fetch_documents(force, cur_term, otp_code).await?;
    let item = match id {
        Some(id) => items
            .into_iter()
            .find(|x| x.1 == id)
            .with_context(|| format!("document with id {} not found", id))?,
        None => {
            if json {
                anyhow::bail!("document download with --markdown requires an explicit document id")
            }
            select_document(items).await?
        }
    };

    if !dir.exists() {
        compio::fs::create_dir_all(dir).await?;
    }
    for (name, uri) in item.2.attachments() {
        item.2
            .download_attachment(uri, &dir.join(name))
            .await
            .with_context(|| format!("download document attachment '{}'", name))?;
    }

    if json {
        let result = DocumentActionRecord {
            action: "document.download".to_owned(),
            course_title: item.0.meta().name().to_owned(),
            id: item.1,
            title: item.2.title().to_owned(),
            path: dir.display().to_string(),
        };
        markdown_output::write_markdown(&markdown_output::ok_item(result)).await?;
    } else {
        println!("Done.");
    }
    Ok(())
}
