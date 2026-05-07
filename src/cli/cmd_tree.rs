use anyhow::Context;
use serde::Serialize;

use super::query_match::{QueryKey, title_match_type};
use super::*;

#[derive(clap::Args)]
pub struct CommandTree {
    #[arg(short, long, default_value = "false")]
    force: bool,

    #[command(subcommand)]
    command: TreeCommands,

    #[arg(long, default_value = "false")]
    json: bool,

    #[arg(long, default_value = "")]
    otp_code: String,
}

#[derive(Subcommand)]
enum TreeCommands {
    #[command(visible_alias("summary"))]
    List {
        course_index: usize,
    },
    Find {
        course_index: usize,
        query: String,
    },
    Kinds {
        course_index: usize,
        kind: String,
    },
}

#[derive(Serialize)]
struct TreeNodeRecord {
    id: String,
    title: String,
    kind: String,
    has_link: bool,
    attachment_count: usize,
    description_count: usize,
}

#[derive(Serialize)]
struct TreeFindRecord {
    id: String,
    title: String,
    kind: String,
    has_link: bool,
    attachment_count: usize,
    description_count: usize,
    match_type: String,
}

#[derive(Serialize)]
struct TreeSummaryRecord {
    course_index: usize,
    course_title: String,
    counts_by_kind: std::collections::BTreeMap<String, usize>,
    nodes: Vec<TreeNodeRecord>,
    summary_text: String,
}

pub async fn run(cmd: CommandTree) -> anyhow::Result<()> {
    match cmd.command {
        TreeCommands::List { course_index } => {
            list(cmd.force, course_index, cmd.otp_code, cmd.json).await?
        }
        TreeCommands::Find {
            course_index,
            query,
        } => find(cmd.force, course_index, &query, cmd.otp_code, cmd.json).await?,
        TreeCommands::Kinds { course_index, kind } => {
            kinds(cmd.force, course_index, &kind, cmd.otp_code, cmd.json).await?
        }
    }
    Ok(())
}

fn node_record(content: &CourseContent) -> TreeNodeRecord {
    TreeNodeRecord {
        id: content.id().to_owned(),
        title: content.title().to_owned(),
        kind: content.kind_name().to_owned(),
        has_link: content.has_link(),
        attachment_count: content.attachments().len(),
        description_count: content.descriptions().len(),
    }
}

fn find_record(content: &CourseContent, match_type: &str) -> TreeFindRecord {
    TreeFindRecord {
        id: content.id().to_owned(),
        title: content.title().to_owned(),
        kind: content.kind_name().to_owned(),
        has_link: content.has_link(),
        attachment_count: content.attachments().len(),
        description_count: content.descriptions().len(),
        match_type: match_type.to_owned(),
    }
}

fn summary_text_from_counts(counts: &std::collections::BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(kind, count)| format!("{kind}:{count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn tree_matches_query(title: &str, id: &str, query: &QueryKey, raw_query: &str) -> bool {
    query.contains_in(title) || id.contains(raw_query)
}

fn collect_tree_find_matches(contents: &[CourseContent], query: &str) -> Vec<TreeFindRecord> {
    let query_key = QueryKey::new(query);
    let mut matches = contents
        .iter()
        .filter_map(|content| {
            if let Some((match_type, score)) =
                title_match_type(content.title(), content.id(), query)
            {
                return Some((score, find_record(content, match_type)));
            }
            if tree_matches_query(content.title(), content.id(), &query_key, query) {
                return Some((9, find_record(content, "contains_normalized")));
            }
            None
        })
        .collect::<Vec<_>>();

    matches.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.kind.cmp(&b.1.kind))
            .then_with(|| a.1.title.cmp(&b.1.title))
            .then_with(|| a.1.id.cmp(&b.1.id))
    });
    matches.into_iter().map(|(_, item)| item).collect()
}

async fn get_course_contents(
    force: bool,
    course_index: usize,
    otp_code: String,
) -> anyhow::Result<(Course, Vec<CourseContent>)> {
    let courses = load_courses(force, false, otp_code).await?;
    let course = courses
        .into_iter()
        .nth(course_index)
        .with_context(|| format!("course index {} not found", course_index))?
        .get()
        .await
        .context("fetch course")?;
    let pb = pbar::new(0);
    let contents = super::cmd_assignment::get_contents(&course, pb).await?;
    Ok((course, contents))
}

pub async fn list(
    force: bool,
    course_index: usize,
    otp_code: String,
    json: bool,
) -> anyhow::Result<()> {
    let (course, contents) = get_course_contents(force, course_index, otp_code).await?;
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for content in &contents {
        *counts.entry(content.kind_name().to_owned()).or_default() += 1;
    }
    let summary_text = summary_text_from_counts(&counts);

    if json {
        let item = TreeSummaryRecord {
            course_index,
            course_title: course.meta().title().to_owned(),
            counts_by_kind: counts,
            nodes: contents.iter().map(node_record).collect(),
            summary_text,
        };
        return json_output::write_json(&json_output::ok_item(item)).await;
    }

    println!("{D}>{D:#} {B}课程内容树摘要{B:#} {D}<{D:#}");
    println!("{BL}{B}{}{B:#}{BL:#}", course.meta().title());
    println!("{summary_text}");
    println!();
    for content in &contents {
        println!(
            "{D}•{D:#} [{GR}{}{GR:#}] {} {D}{}{D:#}",
            content.kind_name(),
            content.title(),
            content.id()
        );
    }
    Ok(())
}

pub async fn find(
    force: bool,
    course_index: usize,
    query: &str,
    otp_code: String,
    json: bool,
) -> anyhow::Result<()> {
    let (course, contents) = get_course_contents(force, course_index, otp_code).await?;
    let matches = collect_tree_find_matches(&contents, query);
    if json {
        return json_output::write_json(&json_output::ok_item(serde_json::json!({
            "course_index": course_index,
            "course_title": course.meta().title(),
            "query": query,
            "matches": matches,
        })))
        .await;
    }

    println!("{BL}{B}{}{B:#}{BL:#}", course.meta().title());
    println!("query: {query}");
    for m in &matches {
        println!(
            "{D}•{D:#} [{}:{}] {} {}",
            m.kind, m.match_type, m.title, m.id
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_text_uses_sorted_btreemap_order() {
        let counts = std::collections::BTreeMap::from([
            ("document".to_owned(), 3usize),
            ("assignment".to_owned(), 1usize),
        ]);

        assert_eq!(
            summary_text_from_counts(&counts),
            "assignment:1, document:3"
        );
    }

    #[test]
    fn tree_match_uses_normalized_title_matching() {
        let query = QueryKey::new("Week 1");
        assert!(tree_matches_query("week1 data", "_1", &query, "Week 1"));
        assert!(tree_matches_query(
            "课程安排 Week 1",
            "_1",
            &query,
            "Week 1"
        ));
        assert!(!tree_matches_query("Week 2", "_1", &query, "Week 1"));
    }

    #[test]
    fn tree_find_record_serializes_match_type() {
        let value = serde_json::to_value(TreeFindRecord {
            id: "_1".to_owned(),
            title: "Week 1".to_owned(),
            kind: "document".to_owned(),
            has_link: true,
            attachment_count: 1,
            description_count: 0,
            match_type: "exact".to_owned(),
        })
        .unwrap();

        assert_eq!(value["match_type"], "exact");
    }

    #[test]
    fn tree_find_payload_fits_json_envelope() {
        let payload = serde_json::json!({
            "course_index": 1,
            "course_title": "Course",
            "query": "Week 1",
            "matches": [TreeFindRecord {
                id: "_1".to_owned(),
                title: "Week 1".to_owned(),
                kind: "document".to_owned(),
                has_link: true,
                attachment_count: 1,
                description_count: 0,
                match_type: "exact".to_owned(),
            }],
        });
        let value = serde_json::to_value(json_output::ok_item(payload)).unwrap();

        assert_eq!(value["schema_version"], "1");
        assert_eq!(value["item"]["matches"][0]["match_type"], "exact");
    }
}

pub async fn kinds(
    force: bool,
    course_index: usize,
    kind: &str,
    otp_code: String,
    json: bool,
) -> anyhow::Result<()> {
    let (course, contents) = get_course_contents(force, course_index, otp_code).await?;
    let kind_lc = kind.to_lowercase();
    let matches = contents
        .iter()
        .filter(|content| content.kind_name() == kind_lc)
        .map(node_record)
        .collect::<Vec<_>>();
    if json {
        return json_output::write_json(&json_output::ok_item(serde_json::json!({
            "course_index": course_index,
            "course_title": course.meta().title(),
            "kind": kind,
            "matches": matches,
        })))
        .await;
    }

    println!("{BL}{B}{}{B:#}{BL:#}", course.meta().title());
    println!("kind: {kind}");
    for m in &matches {
        println!("{D}•{D:#} [{}] {} {}", m.kind, m.title, m.id);
    }
    Ok(())
}
