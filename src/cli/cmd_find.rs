use serde::Serialize;

use super::query_match::title_match_type;
use super::*;

#[derive(clap::Args)]
pub struct CommandFind {
    #[arg(short, long, default_value = "false")]
    force: bool,

    /// 标题关键词或完整标题
    query: String,

    /// 仅搜索某一种内容类型，例如 assignment/document/announcement
    #[arg(long)]
    kind: Option<String>,

    /// 仅在指定课程索引内搜索
    #[arg(long)]
    course_index: Option<usize>,

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
struct FindRecord {
    course_index: usize,
    course_title: String,
    id: String,
    title: String,
    kind: String,
    match_type: String,
    description_count: usize,
    attachment_count: usize,
}

fn sort_find_records(records: &mut [(u8, FindRecord)]) {
    records.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.course_index.cmp(&b.1.course_index))
            .then_with(|| a.1.kind.cmp(&b.1.kind))
            .then_with(|| a.1.title.cmp(&b.1.title))
            .then_with(|| a.1.id.cmp(&b.1.id))
    });
}

pub async fn run(cmd: CommandFind) -> anyhow::Result<()> {
    let results = find(
        cmd.force,
        &cmd.query,
        cmd.kind.as_deref(),
        cmd.course_index,
        !cmd.all_term,
        cmd.otp_code,
    )
    .await?;

    if cmd.json {
        return markdown_output::write_markdown(&markdown_output::ok_item(serde_json::json!({
            "query": cmd.query,
            "kind": cmd.kind,
            "course_index": cmd.course_index,
            "matches": results,
        })))
        .await;
    }

    println!("{D}>{D:#} {B}查找结果{B:#} {D}<{D:#}\n");
    for item in results {
        println!(
            "{GR}[{}]{GR:#} {BL}{}{BL:#} {D}>{D:#} [{}:{}] {} {D}{}{D:#}",
            item.course_index, item.course_title, item.kind, item.match_type, item.title, item.id
        );
    }
    Ok(())
}

async fn find(
    force: bool,
    query: &str,
    kind: Option<&str>,
    course_index_filter: Option<usize>,
    cur_term: bool,
    otp_code: String,
) -> anyhow::Result<Vec<FindRecord>> {
    let courses = load_courses(force, cur_term, otp_code).await?;
    let kind_lc = kind.map(|k| k.to_lowercase());
    let mut scored = Vec::<(u8, FindRecord)>::new();

    for (course_index, handle) in courses.into_iter().enumerate() {
        if let Some(filter) = course_index_filter
            && filter != course_index
        {
            continue;
        }
        let course = handle.get().await?;
        let pb = pbar::new(0);
        let contents = super::cmd_assignment::get_contents(&course, pb).await?;
        for content in contents {
            if let Some(kind_lc) = kind_lc.as_deref()
                && content.kind_name() != kind_lc
            {
                continue;
            }
            let Some((match_type_name, score)) =
                title_match_type(content.title(), content.id(), query)
            else {
                continue;
            };
            scored.push((
                score,
                FindRecord {
                    course_index,
                    course_title: course.meta().title().to_owned(),
                    id: content.id().to_owned(),
                    title: content.title().to_owned(),
                    kind: content.kind_name().to_owned(),
                    match_type: match_type_name.to_owned(),
                    description_count: content.descriptions().len(),
                    attachment_count: content.attachments().len(),
                },
            ));
        }
    }

    sort_find_records(&mut scored);
    Ok(scored.into_iter().map(|(_, item)| item).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(score: u8, course_index: usize, kind: &str, title: &str, id: &str) -> (u8, FindRecord) {
        (
            score,
            FindRecord {
                course_index,
                course_title: "Course".to_owned(),
                id: id.to_owned(),
                title: title.to_owned(),
                kind: kind.to_owned(),
                match_type: "prefix".to_owned(),
                description_count: 0,
                attachment_count: 0,
            },
        )
    }

    #[test]
    fn sort_find_records_uses_score_then_tie_breakers() {
        let mut records = vec![
            mk(4, 1, "document", "Week 1", "b"),
            mk(4, 1, "document", "Week 1", "a"),
            mk(0, 2, "document", "Week 1", "z"),
        ];

        sort_find_records(&mut records);

        assert_eq!(records[0].0, 0);
        assert_eq!(records[0].1.id, "z");
        assert_eq!(records[1].1.id, "a");
        assert_eq!(records[2].1.id, "b");
    }

    #[test]
    fn find_record_serializes_match_type() {
        let value = serde_json::to_value(FindRecord {
            course_index: 0,
            course_title: "Course".to_owned(),
            id: "abc".to_owned(),
            title: "Week 1".to_owned(),
            kind: "document".to_owned(),
            match_type: "exact".to_owned(),
            description_count: 1,
            attachment_count: 2,
        })
        .unwrap();

        assert_eq!(value["match_type"], "exact");
        assert_eq!(value["attachment_count"], 2);
    }

    #[test]
    fn find_matches_payload_fits_json_envelope() {
        let payload = serde_json::json!({
            "query": "Week 1",
            "kind": "document",
            "course_index": 0,
            "matches": [FindRecord {
                course_index: 0,
                course_title: "Course".to_owned(),
                id: "abc".to_owned(),
                title: "Week 1".to_owned(),
                kind: "document".to_owned(),
                match_type: "exact".to_owned(),
                description_count: 0,
                attachment_count: 1,
            }],
        });
        let value = serde_json::to_value(markdown_output::ok_item(payload)).unwrap();

        assert_eq!(value["schema_version"], "1");
        assert_eq!(value["item"]["matches"][0]["match_type"], "exact");
    }
}
