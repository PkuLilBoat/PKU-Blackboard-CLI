use serde::Serialize;

use super::query_match::QueryKey;
use super::*;

#[derive(clap::Args)]
pub struct CommandSearch {
    #[arg(short, long, default_value = "false")]
    force: bool,

    /// 搜索关键词
    query: String,

    /// 仅搜索某一种内容类型，例如 assignment/document/announcement
    #[arg(long)]
    kind: Option<String>,

    /// 显示所有学期
    #[arg(long, default_value = "false")]
    all_term: bool,

    /// 输出 Markdown
    #[arg(long = "markdown", visible_alias = "md", alias = "json", default_value = "false")]
    json: bool,

    /// 手机令牌码。当需要使用 OTP 登录，但未提供此参数时，将会从命令行交互式读取 OTP 码。
    #[arg(long, default_value = "")]
    otp_code: String,
}

#[derive(Serialize)]
struct SearchRecord {
    course_index: usize,
    course_title: String,
    id: String,
    title: String,
    kind: String,
    description_count: usize,
    attachment_count: usize,
}

fn sort_search_records(records: &mut [SearchRecord]) {
    records.sort_by(|a, b| {
        a.course_index
            .cmp(&b.course_index)
            .then_with(|| a.kind.cmp(&b.kind))
            .then_with(|| a.title.cmp(&b.title))
            .then_with(|| a.id.cmp(&b.id))
    });
}

pub async fn run(cmd: CommandSearch) -> anyhow::Result<()> {
    let results = search(
        cmd.force,
        &cmd.query,
        cmd.kind.as_deref(),
        !cmd.all_term,
        cmd.otp_code,
    )
    .await?;

    if cmd.json {
        return markdown_output::write_markdown(&markdown_output::ok_item(serde_json::json!({
            "query": cmd.query,
            "kind": cmd.kind,
            "matches": results,
        })))
        .await;
    }

    println!("{D}>{D:#} {B}搜索结果{B:#} {D}<{D:#}\n");
    for item in results {
        println!(
            "{GR}[{}]{GR:#} {BL}{}{BL:#} {D}>{D:#} [{}] {} {D}{}{D:#}",
            item.course_index, item.course_title, item.kind, item.title, item.id
        );
    }
    Ok(())
}

async fn search(
    force: bool,
    query: &str,
    kind: Option<&str>,
    cur_term: bool,
    otp_code: String,
) -> anyhow::Result<Vec<SearchRecord>> {
    let courses = load_courses(force, cur_term, otp_code).await?;
    let query_key = QueryKey::new(query);
    let kind_lc = kind.map(|k| k.to_lowercase());
    let mut results = Vec::new();

    for (course_index, handle) in courses.into_iter().enumerate() {
        let course = handle.get().await?;
        let pb = pbar::new(0);
        let contents = super::cmd_assignment::get_contents(&course, pb).await?;
        for content in contents {
            if let Some(kind_lc) = kind_lc.as_deref()
                && content.kind_name() != kind_lc
            {
                continue;
            }

            let in_title = query_key.contains_in(content.title());
            let in_desc = content
                .descriptions()
                .iter()
                .any(|d| query_key.contains_in(d));
            if !in_title && !in_desc && !content.id().contains(query) {
                continue;
            }

            results.push(SearchRecord {
                course_index,
                course_title: course.meta().title().to_owned(),
                id: content.id().to_owned(),
                title: content.title().to_owned(),
                kind: content.kind_name().to_owned(),
                description_count: content.descriptions().len(),
                attachment_count: content.attachments().len(),
            });
        }
    }

    sort_search_records(&mut results);
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_search_records_uses_all_tie_breakers() {
        let mut records = vec![
            SearchRecord {
                course_index: 1,
                course_title: "Course".to_owned(),
                id: "b".to_owned(),
                title: "Week 1".to_owned(),
                kind: "document".to_owned(),
                description_count: 0,
                attachment_count: 0,
            },
            SearchRecord {
                course_index: 1,
                course_title: "Course".to_owned(),
                id: "a".to_owned(),
                title: "Week 1".to_owned(),
                kind: "document".to_owned(),
                description_count: 0,
                attachment_count: 0,
            },
        ];

        sort_search_records(&mut records);

        assert_eq!(records[0].id, "a");
        assert_eq!(records[1].id, "b");
    }

    #[test]
    fn search_matches_payload_fits_json_envelope() {
        let payload = serde_json::json!({
            "query": "Week",
            "kind": "document",
            "matches": [SearchRecord {
                course_index: 1,
                course_title: "Course".to_owned(),
                id: "a".to_owned(),
                title: "Week 1".to_owned(),
                kind: "document".to_owned(),
                description_count: 0,
                attachment_count: 1,
            }],
        });
        let value = serde_json::to_value(markdown_output::ok_item(payload)).unwrap();

        assert_eq!(value["schema_version"], "1");
        assert_eq!(value["item"]["matches"][0]["title"], "Week 1");
    }
}
