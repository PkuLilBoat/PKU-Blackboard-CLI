use super::*;
use crate::cli::pbar;
use anyhow::Context;
use chrono::{Datelike as _, TimeZone as _};
use compio::buf::buf_try;
use compio::fs;
use compio::io::AsyncWriteExt;
use std::io::Write;

#[derive(clap::Args)]
pub struct CommandCourseTable {
    /// 强制刷新
    #[arg(short, long, default_value = "false")]
    force: bool,

    /// 显示原始 JSON 数据（用于调试）
    #[arg(short, long, default_value = "false")]
    raw: bool,

    /// 输出 JSON 包装结果
    #[arg(long, default_value = "false")]
    json: bool,

    /// 手机令牌码。当需要使用 OTP 登录，但未提供此参数时，将会从命令行交互式读取 OTP 码。
    #[arg(long, default_value = "")]
    otp_code: String,
}

/// 获取个人课表
pub async fn run(cmd: CommandCourseTable) -> anyhow::Result<()> {
    let CommandCourseTable {
        force,
        raw,
        json,
        otp_code,
    } = cmd;

    let client = build_client(!force).await?;

    let sp = pbar::new_spinner();
    sp.set_message("reading config...");
    let cfg = load_runtime_config().await?;

    sp.set_message("logging in to portal...");

    let portal = match client.portal(&cfg.username, &cfg.password, &otp_code).await {
        Ok(portal) => portal,
        Err(err) if otp_code.is_empty() && super::is_incorrect_otp_error(&err) => {
            match blackboard_fallback(force, format!("{err:#}")).await {
                Ok(fallback) => {
                    sp.finish_and_clear();
                    return write_fallback_output(fallback, json).await;
                }
                Err(_) => {
                    let otp_code = resolve_otp_code(true, otp_code)?;
                    client
                        .portal(&cfg.username, &cfg.password, &otp_code)
                        .await
                        .context("login to portal")?
                }
            }
        }
        Err(err) if otp_code.is_empty() => match blackboard_fallback(force, format!("{err:#}")).await {
            Ok(fallback) => {
                sp.finish_and_clear();
                return write_fallback_output(fallback, json).await;
            }
            Err(_) => return Err(err).context("login to portal"),
        },
        Err(err) => return Err(err).context("login to portal"),
    };

    sp.set_message("fetching course table...");

    let raw_data = portal.get_my_course_table().await?;

    sp.finish_and_clear();

    // 输出结果
    if json {
        let item = serde_json::from_str::<serde_json::Value>(&raw_data)
            .unwrap_or_else(|_| serde_json::Value::String(raw_data.clone()));
        json_output::write_json(&json_output::ok_item(item)).await?;
    } else {
        let mut outbuf = Vec::new();
        if raw {
            writeln!(outbuf, "{}", raw_data)?;
        } else {
            let json: serde_json::Value = serde_json::from_str(&raw_data)?;
            if let Some(courses) = json.get("course").and_then(|c| c.as_array()) {
                if courses.is_empty() {
                    writeln!(outbuf, "暂无课表数据")?;
                } else {
                    writeln!(outbuf, "个人课表\n")?;

                    let days = [
                        ("mon", "周一"),
                        ("tue", "周二"),
                        ("wed", "周三"),
                        ("thu", "周四"),
                        ("fri", "周五"),
                        ("sat", "周六"),
                        ("sun", "周日"),
                    ];

                    for (day_key, day_name) in days.iter() {
                        let mut day_slots: Vec<(usize, String)> = Vec::new();

                        for (idx, slot) in courses.iter().enumerate() {
                            let slot_num = idx + 1;

                            if let Some(course) = slot.get(day_key)
                                && let Some(name) =
                                    course.get("courseName").and_then(|n| n.as_str())
                                && !name.is_empty()
                            {
                                let clean_info = format_course_info(name);
                                day_slots.push((slot_num, clean_info));
                            }
                        }

                        if !day_slots.is_empty() {
                            writeln!(outbuf, "[{}]", day_name)?;

                            let mut i = 0;
                            while i < day_slots.len() {
                                let (start_slot, info) = &day_slots[i];
                                let mut end_slot = *start_slot;

                                let mut j = i + 1;
                                while j < day_slots.len() && day_slots[j].1 == *info {
                                    end_slot = day_slots[j].0;
                                    j += 1;
                                }

                                if start_slot == &end_slot {
                                    writeln!(outbuf, "  第{}节: {}", start_slot, info)?;
                                } else {
                                    writeln!(
                                        outbuf,
                                        "  第{}-{}节: {}",
                                        start_slot, end_slot, info
                                    )?;
                                }

                                i = j;
                            }

                            writeln!(outbuf)?;
                        }
                    }
                }
            } else {
                writeln!(outbuf, "{}", raw_data)?;
            }
        }
        buf_try!(@try fs::stdout().write_all(outbuf).await);
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct CourseTableFallbackCourse {
    title: String,
}

#[derive(serde::Serialize)]
struct CourseTableFallbackEvent {
    calendar_name: String,
    title: String,
    event_type: Option<String>,
    start: Option<String>,
    end: Option<String>,
    location: Option<String>,
}

#[derive(serde::Serialize)]
struct CourseTableFallback {
    source: String,
    reason: String,
    window_start: String,
    window_end: String,
    courses: Vec<CourseTableFallbackCourse>,
    events: Vec<CourseTableFallbackEvent>,
}

async fn blackboard_fallback(force: bool, reason: String) -> anyhow::Result<CourseTableFallback> {
    let (client, courses, sp) = load_client_courses(force, true, String::new()).await?;
    sp.set_message("fetching blackboard calendar fallback...");

    let now = chrono::Local::now();
    let start = chrono::Local
        .with_ymd_and_hms(now.year(), 1, 1, 0, 0, 0)
        .single()
        .context("invalid calendar fallback start")?;
    let end = chrono::Local
        .with_ymd_and_hms(now.year() + 1, 1, 1, 0, 0, 0)
        .single()
        .context("invalid calendar fallback end")?;

    let raw = client
        .bb_calendar_selected_events(start.timestamp_millis(), end.timestamp_millis())
        .await
        .context("fetch blackboard calendar events")?;

    let events_json: Vec<serde_json::Value> =
        serde_json::from_str(&raw).context("parse blackboard calendar events")?;

    let mut events = events_json
        .into_iter()
        .map(|event| CourseTableFallbackEvent {
            calendar_name: event
                .get("calendarName")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned(),
            title: event
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned(),
            event_type: event
                .get("eventType")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned),
            start: event
                .get("start")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned),
            end: event
                .get("end")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned),
            location: event
                .get("location")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned),
        })
        .filter(|event| !event.title.is_empty())
        .collect::<Vec<_>>();

    events.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then_with(|| a.calendar_name.cmp(&b.calendar_name))
            .then_with(|| a.title.cmp(&b.title))
    });

    let mut course_titles = courses
        .into_iter()
        .map(|course| course.title().to_owned())
        .collect::<Vec<_>>();
    course_titles.sort();

    sp.finish_and_clear();

    Ok(CourseTableFallback {
        source: "blackboard_calendar_fallback".to_owned(),
        reason,
        window_start: start.to_rfc3339(),
        window_end: end.to_rfc3339(),
        courses: course_titles
            .into_iter()
            .map(|title| CourseTableFallbackCourse { title })
            .collect(),
        events,
    })
}

async fn write_fallback_output(fallback: CourseTableFallback, json: bool) -> anyhow::Result<()> {
    if json {
        json_output::write_json(&json_output::ok_item(fallback)).await?;
        return Ok(());
    }

    let mut outbuf = Vec::new();
    writeln!(outbuf, "未能直接获取门户课表，已回退到 Blackboard 日程视图。")?;
    writeln!(outbuf, "原因: {}", fallback.reason)?;
    writeln!(outbuf)?;

    if !fallback.courses.is_empty() {
        writeln!(outbuf, "[当前学期课程]")?;
        for course in &fallback.courses {
            writeln!(outbuf, "- {}", course.title)?;
        }
        writeln!(outbuf)?;
    }

    if fallback.events.is_empty() {
        writeln!(outbuf, "Blackboard 日程中没有可用事件。")?;
    } else {
        writeln!(outbuf, "[Blackboard 日程事件]")?;
        for event in &fallback.events {
            let when = event.start.as_deref().unwrap_or("unknown time");
            let event_type = event.event_type.as_deref().unwrap_or("事件");
            writeln!(
                outbuf,
                "- {} | {} | {} | {}",
                when, event.calendar_name, event_type, event.title
            )?;
        }
    }

    buf_try!(@try fs::stdout().write_all(outbuf).await);
    Ok(())
}

fn format_course_info(info: &str) -> String {
    let course_name = info.split("(主)").next().unwrap_or(info).trim();

    let mut result = course_name.to_string();

    if let Some(class_idx) = info.find("上课信息：") {
        let class_start = class_idx + 15;
        let rest = &info[class_start..];
        let class_end = rest.find("教师：").unwrap_or(rest.len());
        let class_info = rest[..class_end].trim();
        if !class_info.is_empty() {
            result.push_str(" | ");
            result.push_str(class_info);
        }

        if let Some(teacher_idx) = rest.find("教师：") {
            let teacher_start = teacher_idx + 9;
            let teacher_rest = &rest[teacher_start..];
            let teacher_end = teacher_rest
                .find(' ')
                .or_else(|| teacher_rest.find("\u{003c}"))
                .unwrap_or(teacher_rest.len());
            let teacher = teacher_rest[..teacher_end].trim();
            if !teacher.is_empty() {
                result.push_str(" | 教师：");
                result.push_str(teacher);
            }
        }
    }

    if let Some(exam_idx) = info.find("考试信息：") {
        let exam_start = exam_idx + 15;
        let rest = &info[exam_start..];
        let exam_end = rest.find("\u{003c}").unwrap_or(rest.len());
        let exam_info = rest[..exam_end].trim();
        if !exam_info.is_empty() {
            result.push_str(" | 考试：");
            result.push_str(exam_info);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_course_info_extracts_course_room_and_teacher() {
        let raw = "高级心理统计学(主)上课信息：理教101 教师：张三 <br>考试信息：闭卷";
        let formatted = format_course_info(raw);

        assert!(formatted.contains("高级心理统计学"));
        assert!(formatted.contains("理教101"));
        assert!(formatted.contains("教师：张三"));
        assert!(formatted.contains("考试：闭卷"));
    }

    #[test]
    fn coursetable_payload_fits_json_envelope() {
        let payload = serde_json::json!({
            "course": [
                {
                    "mon": {
                        "courseName": "高级心理统计学(主)上课信息：理教101 教师：张三"
                    }
                }
            ]
        });
        let value = serde_json::to_value(json_output::ok_item(payload)).unwrap();

        assert_eq!(value["schema_version"], "1");
        assert_eq!(value["ok"], true);
        assert_eq!(
            value["item"]["course"][0]["mon"]["courseName"],
            "高级心理统计学(主)上课信息：理教101 教师：张三"
        );
    }

    #[test]
    fn coursetable_fallback_payload_fits_json_envelope() {
        let payload = CourseTableFallback {
            source: "blackboard_calendar_fallback".to_owned(),
            reason: "oauth login not success".to_owned(),
            window_start: "2026-01-01T00:00:00+08:00".to_owned(),
            window_end: "2027-01-01T00:00:00+08:00".to_owned(),
            courses: vec![CourseTableFallbackCourse {
                title: "决策行为(25-26学年第2学期)".to_owned(),
            }],
            events: vec![CourseTableFallbackEvent {
                calendar_name: "决策行为(25-26学年第2学期)".to_owned(),
                title: "作业1（ddl 4.12 23:59）".to_owned(),
                event_type: Some("作业".to_owned()),
                start: Some("2026-04-12T23:59:00".to_owned()),
                end: Some("2026-04-12T23:59:00".to_owned()),
                location: None,
            }],
        };
        let value = serde_json::to_value(json_output::ok_item(payload)).unwrap();

        assert_eq!(value["schema_version"], "1");
        assert_eq!(value["ok"], true);
        assert_eq!(value["item"]["source"], "blackboard_calendar_fallback");
        assert_eq!(value["item"]["courses"][0]["title"], "决策行为(25-26学年第2学期)");
        assert_eq!(value["item"]["events"][0]["title"], "作业1（ddl 4.12 23:59）");
    }
}
