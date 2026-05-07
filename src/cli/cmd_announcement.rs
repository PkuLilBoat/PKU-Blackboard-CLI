use std::sync::Arc;

use anyhow::Context;
use serde::Serialize;

use super::*;

#[derive(clap::Args)]
pub struct CommandAnnouncement {
    /// 强制刷新
    #[arg(short, long, default_value = "false")]
    force: bool,

    #[command(subcommand)]
    command: AnnouncementCommands,

    /// 输出 Markdown
    #[arg(long = "markdown", visible_alias = "md", alias = "json", default_value = "false")]
    json: bool,

    /// 手机令牌码。当需要使用 OTP 登录，但未提供此参数时，将会从命令行交互式读取 OTP 码。
    #[arg(long, default_value = "")]
    otp_code: String,
}

#[derive(Subcommand)]
enum AnnouncementCommands {
    /// 查看课程公告列表
    #[command(visible_alias("ls"))]
    List {
        /// 显示所有学期的课程公告
        #[arg(long, default_value = "false")]
        all_term: bool,
    },
    /// 按 ID 查看公告详情
    Show {
        /// 公告 ID（可通过 `pku3b announcement ls` 查看）
        id: String,
        /// 在所有学期的课程公告范围中查找
        #[arg(long, default_value = "false")]
        all_term: bool,
    },
}

pub async fn run(cmd: CommandAnnouncement) -> anyhow::Result<()> {
    match cmd.command {
        AnnouncementCommands::List { all_term } => {
            list(cmd.force, !all_term, cmd.otp_code, cmd.json).await?
        }
        AnnouncementCommands::Show { id, all_term } => {
            show(cmd.force, !all_term, &id, cmd.otp_code, cmd.json).await?
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct AnnouncementListRecord {
    course_title: String,
    id: String,
    title: String,
    published_at: Option<String>,
    attachment_count: usize,
}

#[derive(Serialize)]
struct AnnouncementAttachmentRecord {
    name: String,
    url: String,
}

#[derive(Serialize)]
struct AnnouncementDetailRecord {
    course_title: String,
    id: String,
    title: String,
    published_at: Option<String>,
    descriptions: Vec<String>,
    attachments: Vec<AnnouncementAttachmentRecord>,
}

type AnnouncementListItem = (Arc<Course>, String, CourseAnnouncementHandle);

async fn get_announcements(
    course: &Course,
    pb: indicatif::ProgressBar,
) -> anyhow::Result<Vec<CourseAnnouncementHandle>> {
    let announcements = course
        .list_announcements_from_coursepage()
        .await
        .with_context(|| {
            format!(
                "fetch announcements from course page for {}",
                course.meta().title()
            )
        })?;
    pb.finish_with_message("done.");
    Ok(announcements)
}

async fn get_courses_and_announcements(
    force: bool,
    cur_term: bool,
    otp_code: String,
) -> anyhow::Result<Vec<(Course, Vec<CourseAnnouncementHandle>)>> {
    let courses = load_courses(force, cur_term, otp_code).await?;

    let m = indicatif::MultiProgress::new();
    let pb = m.add(pbar::new(courses.len() as u64)).with_prefix("All");
    let futs = courses
        .into_iter()
        .map(async |course| -> anyhow::Result<_> {
            let course = course.get().await.context("fetch course")?;
            let announcements = get_announcements(
                &course,
                m.add(pbar::new(0).with_prefix(course.meta().name().to_owned())),
            )
            .await
            .with_context(|| format!("fetch announcement handles of {}", course.meta().title()))?;

            pb.inc_length(announcements.len() as u64);
            let futs = announcements
                .into_iter()
                .map(async |announcement| -> anyhow::Result<_> {
                    pb.inc(1);
                    Ok(announcement)
                });
            let announcements = try_join_all(futs).await?;

            pb.inc(1);
            Ok((course, announcements))
        });
    let courses = try_join_all(futs).await?;
    pb.finish_and_clear();
    m.clear().unwrap();
    drop(pb);
    drop(m);

    Ok(courses)
}

pub async fn list(force: bool, cur_term: bool, otp_code: String, json: bool) -> anyhow::Result<()> {
    let courses = get_courses_and_announcements(force, cur_term, otp_code).await?;
    let all_announcements = courses
        .iter()
        .flat_map(|(course, announcements)| {
            announcements.iter().map(move |announcement| {
                (course.to_owned(), announcement.id(), announcement.clone())
            })
        })
        .collect::<Vec<_>>();

    let announcements = sort_announcements_owned(all_announcements);
    if json {
        let items = announcements
            .into_iter()
            .map(|(course, id, announcement)| AnnouncementListRecord {
                course_title: course.meta().name().to_owned(),
                id,
                title: announcement.title().to_owned(),
                published_at: announcement.time().map(|t| t.to_string()),
                attachment_count: announcement.attachments().len(),
            })
            .collect::<Vec<_>>();
        markdown_output::write_markdown(&markdown_output::ok_items(items)).await
    } else {
        list_brief(announcements).await
    }
}

async fn list_brief(items: Vec<(Course, String, CourseAnnouncementHandle)>) -> anyhow::Result<()> {
    let mut outbuf = Vec::new();
    let title = "课程公告";
    let total = items.len();
    writeln!(outbuf, "{D}>{D:#} {B}{title} ({total}){B:#} {D}<{D:#}\n")?;

    for (idx, (course, id, announcement)) in items.iter().enumerate() {
        write!(outbuf, "{GR}[{:>2}]{GR:#} ", idx + 1)?;
        write!(
            outbuf,
            "{BL}{B}{}{B:#}{BL:#} {D}>{D:#} {}",
            course.meta().name(),
            announcement.title()
        )?;
        let att_count = announcement.attachments().len();
        if att_count > 0 {
            write!(outbuf, " ({GR}{att_count} 个附件{GR:#})")?;
        }
        writeln!(outbuf, " {D}{id}{D:#}")?;
    }

    buf_try!(@try fs::stdout().write_all(outbuf).await);
    Ok(())
}

pub async fn show(
    force: bool,
    cur_term: bool,
    id: &str,
    otp_code: String,
    json: bool,
) -> anyhow::Result<()> {
    let items = fetch_announcements(force, cur_term, otp_code).await?;
    let Some((course, ann_id, announcement)) =
        items.into_iter().find(|(_, ann_id, _)| ann_id == id)
    else {
        anyhow::bail!("announcement with id {} not found", id);
    };

    if json {
        let item = AnnouncementDetailRecord {
            course_title: course.meta().name().to_owned(),
            id: ann_id,
            title: announcement.title().to_owned(),
            published_at: announcement.time().map(|t| t.to_string()),
            descriptions: announcement
                .descriptions()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            attachments: announcement
                .attachments()
                .iter()
                .map(|(name, url)| AnnouncementAttachmentRecord {
                    name: name.to_string(),
                    url: url.to_string(),
                })
                .collect(),
        };
        markdown_output::write_markdown(&markdown_output::ok_item(item)).await?;
    } else {
        let mut outbuf = Vec::new();
        writeln!(outbuf, "{D}>{D:#} {B}公告详情{B:#} {D}<{D:#}\n")?;
        write_announcement_detail(&mut outbuf, &ann_id, &course, &announcement)
            .context("io error")?;
        buf_try!(@try fs::stdout().write_all(outbuf).await);
    }
    Ok(())
}

fn sort_announcements_owned(
    mut items: Vec<(Course, String, CourseAnnouncementHandle)>,
) -> Vec<(Course, String, CourseAnnouncementHandle)> {
    items.sort_by(|a, b| match (b.2.time(), a.2.time()) {
        (Some(time_b), Some(time_a)) => time_b.cmp(time_a),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    items
}

#[cfg(test)]
fn sort_announcement_records(records: &mut [AnnouncementListRecord]) {
    records.sort_by(|a, b| {
        b.published_at
            .cmp(&a.published_at)
            .then_with(|| a.course_title.cmp(&b.course_title))
            .then_with(|| a.title.cmp(&b.title))
            .then_with(|| a.id.cmp(&b.id))
    });
}

fn sort_announcements_items(mut items: Vec<AnnouncementListItem>) -> Vec<AnnouncementListItem> {
    items.sort_by(|a, b| match (b.2.time(), a.2.time()) {
        (Some(time_b), Some(time_a)) => time_b.cmp(time_a),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    items
}

fn write_announcement_detail(
    buf: &mut Vec<u8>,
    id: &str,
    course: &Course,
    announcement: &CourseAnnouncementHandle,
) -> std::io::Result<()> {
    writeln!(
        buf,
        "{BL}{B}{}{B:#}{BL:#} {D}>{D:#} {BL}{B}{}{B:#}{BL:#}",
        course.meta().name(),
        announcement.title()
    )?;
    writeln!(buf, "{D}ID:{D:#} {id}")?;

    if let Some(time) = announcement.time() {
        writeln!(buf, "{D}发布时间:{D:#} {time}")?;
    }

    if !announcement.descriptions().is_empty() {
        writeln!(buf)?;
        for line in announcement.descriptions() {
            writeln!(buf, "{line}")?;
        }
    }

    if !announcement.attachments().is_empty() {
        writeln!(buf)?;
        for (name, _) in announcement.attachments() {
            writeln!(buf, "{D}[附件]{D:#} {UL}{name}{UL:#}")?;
        }
    }

    writeln!(buf)?;
    Ok(())
}

async fn fetch_announcements(
    force: bool,
    cur_term: bool,
    otp_code: String,
) -> anyhow::Result<Vec<AnnouncementListItem>> {
    let courses = get_courses_and_announcements(force, cur_term, otp_code).await?;

    let mut all_announcements = courses
        .into_iter()
        .flat_map(|(course, announcements)| {
            let course = Arc::new(course);
            announcements
                .into_iter()
                .map(move |announcement| (course.clone(), announcement.id(), announcement))
        })
        .collect::<Vec<_>>();

    all_announcements = sort_announcements_items(all_announcements);
    Ok(all_announcements)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_announcement_records_uses_published_at_then_tie_breakers() {
        let mut records = vec![
            AnnouncementListRecord {
                course_title: "B".to_owned(),
                id: "2".to_owned(),
                title: "Notice".to_owned(),
                published_at: Some("2026-01-01 10:00:00".to_owned()),
                attachment_count: 0,
            },
            AnnouncementListRecord {
                course_title: "A".to_owned(),
                id: "1".to_owned(),
                title: "Notice".to_owned(),
                published_at: Some("2026-01-01 10:00:00".to_owned()),
                attachment_count: 0,
            },
        ];

        sort_announcement_records(&mut records);

        assert_eq!(records[0].course_title, "A");
        assert_eq!(records[1].course_title, "B");
    }

    #[test]
    fn announcement_detail_payload_fits_json_envelope() {
        let detail = AnnouncementDetailRecord {
            course_title: "Course".to_owned(),
            id: "ann1".to_owned(),
            title: "Notice".to_owned(),
            published_at: Some("2026-01-01 10:00:00".to_owned()),
            descriptions: vec!["desc".to_owned()],
            attachments: vec![AnnouncementAttachmentRecord {
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
