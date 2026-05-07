use serde::Serialize;

use super::*;

#[derive(clap::Args)]
pub struct CommandCourse {
    #[arg(short, long, default_value = "false")]
    force: bool,

    #[command(subcommand)]
    command: CourseCommands,

    #[arg(long = "markdown", visible_alias = "md", alias = "json", default_value = "false")]
    json: bool,

    #[arg(long, default_value = "")]
    otp_code: String,
}

#[derive(Subcommand)]
enum CourseCommands {
    #[command(visible_alias("ls"))]
    List {
        #[arg(long, default_value = "false")]
        all_term: bool,
    },
    Entries {
        course_index: usize,
        #[arg(long, default_value = "false")]
        all_term: bool,
    },
}

#[derive(Serialize)]
struct CourseListRecord {
    index: usize,
    title: String,
}

#[derive(Serialize)]
struct CourseEntryRecord {
    title: String,
    url: String,
}

fn sort_entries(entries: &mut [CourseEntryRecord]) {
    entries.sort_by(|a, b| a.title.cmp(&b.title));
}

pub async fn run(cmd: CommandCourse) -> anyhow::Result<()> {
    match cmd.command {
        CourseCommands::List { all_term } => {
            list(cmd.force, !all_term, cmd.otp_code, cmd.json).await?
        }
        CourseCommands::Entries {
            course_index,
            all_term,
        } => entries(cmd.force, course_index, !all_term, cmd.otp_code, cmd.json).await?,
    }
    Ok(())
}

pub async fn list(force: bool, cur_term: bool, otp_code: String, json: bool) -> anyhow::Result<()> {
    let courses = load_courses(force, cur_term, otp_code).await?;
    if json {
        let items = courses
            .iter()
            .enumerate()
            .map(|(index, c)| CourseListRecord {
                index,
                title: c.title().to_owned(),
            })
            .collect::<Vec<_>>();
        return markdown_output::write_markdown(&markdown_output::ok_items(items)).await;
    }

    println!("{D}>{D:#} {B}课程列表{B:#} {D}<{D:#}\n");
    for (index, c) in courses.iter().enumerate() {
        println!("{GR}[{:>2}]{GR:#} {}", index, c.title());
    }
    Ok(())
}

pub async fn entries(
    force: bool,
    course_index: usize,
    cur_term: bool,
    otp_code: String,
    json: bool,
) -> anyhow::Result<()> {
    let courses = load_courses(force, cur_term, otp_code).await?;
    let course = courses
        .into_iter()
        .nth(course_index)
        .with_context(|| format!("course index {} not found", course_index))?
        .get()
        .await?;

    let mut pairs = course
        .entries()
        .iter()
        .map(|(title, url)| CourseEntryRecord {
            title: title.to_owned(),
            url: url.to_owned(),
        })
        .collect::<Vec<_>>();
    sort_entries(&mut pairs);

    if json {
        return markdown_output::write_markdown(&markdown_output::ok_item(serde_json::json!({
            "course_index": course_index,
            "course_title": course.meta().title(),
            "entries": pairs,
        })))
        .await;
    }

    println!("{BL}{B}{}{B:#}{BL:#}", course.meta().title());
    for entry in pairs {
        println!("{D}•{D:#} {} {D}{}{D:#}", entry.title, entry.url);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_entries_orders_by_title() {
        let mut entries = vec![
            CourseEntryRecord {
                title: "z-work".to_owned(),
                url: "/b".to_owned(),
            },
            CourseEntryRecord {
                title: "a-announcement".to_owned(),
                url: "/a".to_owned(),
            },
        ];

        sort_entries(&mut entries);

        assert_eq!(entries[0].title, "a-announcement");
        assert_eq!(entries[1].title, "z-work");
    }

    #[test]
    fn course_entry_record_serializes_url() {
        let value = serde_json::to_value(CourseEntryRecord {
            title: "Assignments".to_owned(),
            url: "https://example.com".to_owned(),
        })
        .unwrap();

        assert_eq!(value["title"], "Assignments");
        assert_eq!(value["url"], "https://example.com");
    }

    #[test]
    fn course_list_record_serializes_index() {
        let value = serde_json::to_value(CourseListRecord {
            index: 3,
            title: "Course".to_owned(),
        })
        .unwrap();

        assert_eq!(value["index"], 3);
        assert_eq!(value["title"], "Course");
    }

    #[test]
    fn course_entries_payload_fits_json_envelope() {
        let payload = serde_json::json!({
            "course_index": 1,
            "course_title": "Course",
            "entries": [CourseEntryRecord {
                title: "Assignments".to_owned(),
                url: "https://example.com".to_owned(),
            }],
        });
        let value = serde_json::to_value(markdown_output::ok_item(payload)).unwrap();

        assert_eq!(value["schema_version"], "1");
        assert_eq!(value["ok"], true);
        assert_eq!(value["item"]["entries"][0]["title"], "Assignments");
    }
}
