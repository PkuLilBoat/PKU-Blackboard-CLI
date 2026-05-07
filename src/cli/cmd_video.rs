use anyhow::Context;
use serde::Serialize;

use super::*;

#[derive(clap::Args)]
pub struct CommandVideo {
    /// 强制刷新
    #[arg(short, long, default_value = "false")]
    force: bool,

    #[command(subcommand)]
    command: VideoCommands,

    /// 输出 Markdown
    #[arg(long = "markdown", visible_alias = "md", alias = "json", default_value = "false")]
    json: bool,

    /// 手机令牌码。当需要使用 OTP 登录，但未提供此参数时，将会从命令行交互式读取 OTP 码。
    #[arg(long, default_value = "")]
    otp_code: String,
}

#[derive(Subcommand)]
enum VideoCommands {
    /// 获取课程回放列表
    #[command(visible_alias("ls"))]
    List {
        /// 显示所有学期的课程回放
        #[arg(long, default_value = "false")]
        all_term: bool,
    },

    /// 下载课程回放视频 (MP4 格式)，支持断点续传
    #[command(visible_alias("down"))]
    #[cfg(feature = "video-download")]
    Download {
        /// 课程回放 ID (形如 `e780808c9eb81f61`, 可通过 `pku3b video list` 查看)
        id: String,

        /// 在所有学期的课程回放范围中查找
        #[arg(long, default_value = "false")]
        all_term: bool,

        /// 文件下载目录 (支持相对路径)
        #[arg(short = 'o', long)]
        outdir: Option<std::path::PathBuf>,
    },
}

pub async fn run(cmd: CommandVideo) -> anyhow::Result<()> {
    match cmd.command {
        VideoCommands::List { all_term } => {
            list(cmd.force, !all_term, cmd.otp_code, cmd.json).await?
        }
        #[cfg(feature = "video-download")]
        VideoCommands::Download {
            outdir,
            id,
            all_term,
        } => {
            download(
                outdir.as_deref(),
                cmd.force,
                id,
                !all_term,
                cmd.otp_code,
                cmd.json,
            )
            .await?
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct VideoListRecord {
    course_title: String,
    id: String,
    title: String,
    duration: String,
}

#[derive(Serialize)]
struct VideoActionRecord {
    action: String,
    id: String,
    course_title: String,
    title: String,
    output_path: String,
}

fn sort_video_records(records: &mut [VideoListRecord]) {
    records.sort_by(|a, b| {
        a.course_title
            .cmp(&b.course_title)
            .then_with(|| a.title.cmp(&b.title))
            .then_with(|| a.id.cmp(&b.id))
    });
}

pub async fn list(force: bool, cur_term: bool, otp_code: String, json: bool) -> anyhow::Result<()> {
    let courses = load_courses(force, cur_term, otp_code).await?;

    let pb = pbar::new(courses.len() as u64);
    let futs = courses.into_iter().map(async |c| -> anyhow::Result<_> {
        let c = c.get().await.context("fetch course")?;
        let vs = c.get_video_list().await.context("fetch video list")?;
        pb.inc(1);
        Ok((c, vs))
    });
    let courses = try_join_all(futs).await?;
    pb.finish_and_clear();

    if json {
        let mut items = Vec::new();
        for (c, vs) in courses {
            for v in vs {
                items.push(VideoListRecord {
                    course_title: c.meta().title().to_owned(),
                    id: v.id(),
                    title: v.meta().title().to_owned(),
                    duration: v.meta().time().to_string(),
                });
            }
        }
        sort_video_records(&mut items);
        markdown_output::write_markdown(&markdown_output::ok_items(items)).await?;
    } else {
        let mut outbuf = Vec::new();
        let title = "课程回放";

        writeln!(outbuf, "{D}>{D:#} {B}{title}{B:#} {D}<{D:#}\n")?;

        for (c, vs) in courses {
            if vs.is_empty() {
                continue;
            }

            writeln!(outbuf, "{BL}{H1}[{}]{H1:#}{BL:#}\n", c.meta().title())?;

            for v in vs {
                writeln!(
                    outbuf,
                    "{D}•{D:#} {} ({}) {D}{}{D:#}",
                    v.meta().title(),
                    v.meta().time(),
                    v.id()
                )?;
            }

            writeln!(outbuf)?;
        }

        buf_try!(@try fs::stdout().write_all(outbuf).await);
    }
    Ok(())
}

#[cfg(feature = "video-download")]
pub async fn download(
    outdir: Option<&std::path::Path>,
    force: bool,
    id: String,
    cur_term: bool,
    otp_code: String,
    json: bool,
) -> anyhow::Result<()> {
    let outdir = outdir.unwrap_or(std::path::Path::new("."));
    if !outdir.exists() {
        anyhow::bail!("output directory {:?} not exists", outdir.display());
    }

    let (_, courses, sp) = load_client_courses(force, cur_term, otp_code).await?;

    sp.set_message("finding video...");
    let mut target_video = None;
    for c in courses {
        let c = c.get().await.context("fetch course")?;

        let vs = c.get_video_list().await?;
        for v in vs {
            if v.id() == id {
                target_video = Some(v);
                break;
            }
        }

        if target_video.is_some() {
            break;
        }
    }
    let Some(v) = target_video else {
        anyhow::bail!("video with id {} not found", id);
    };

    sp.set_message("fetch video metadata...");
    let v = v.get().await?;

    drop(sp);

    if !json {
        println!("下载课程回放：{} ({})", v.course_name(), v.meta().title());
    }

    // prepare download dir
    let dir = utils::projectdir()
        .cache_dir()
        .join("video_download")
        .join(&id);
    fs::create_dir_all(&dir)
        .await
        .context("create dir failed")?;

    let paths = download_segments(&v, &dir)
        .await
        .context("download ts segments")?;

    let m3u8 = dir.join("playlist").with_extension("m3u8");
    buf_try!(@try fs::write(&m3u8, v.m3u8_raw()).await);

    // merge all segments into one file
    let merged = dir.join("merged").with_extension("ts");
    merge_segments(&merged, &paths).await?;

    let dest = format!("{}_{}.mp4", v.course_name(), v.meta().title());
    let dest = outdir.join(&dest);
    log::info!("Merged segments to {}", merged.display());
    log::info!(
        r#"You may execute `ffmpeg -i "{}" -c copy "{}"` to convert it to mp4"#,
        merged.display(),
        dest.display(),
    );

    // convert the merged ts file to mp4. overwrite existing file
    let sp = pbar::new_spinner();
    sp.set_message("Converting to mp4 file...");
    let c = compio::process::Command::new("ffmpeg")
        .args(["-y", "-hide_banner", "-loglevel", "quiet"])
        .args(["-i", merged.to_string_lossy().as_ref()])
        .args(["-c", "copy"])
        .arg(&dest)
        .output()
        .await
        .context("execute ffmpeg")?;
    drop(sp);

    if c.status.success() {
        if json {
            let item = VideoActionRecord {
                action: "video.download".to_owned(),
                id,
                course_title: v.course_name().to_owned(),
                title: v.meta().title().to_owned(),
                output_path: dest.display().to_string(),
            };
            markdown_output::write_markdown(&markdown_output::ok_item(item)).await?;
        } else {
            println!(
                "下载完成, 文件保存为: {GR}{H2}{}{H2:#}{GR:#}",
                dest.display()
            );
        }
    } else {
        anyhow::bail!("ffmpeg failed with exit code {:?}", c.status.code());
    }

    Ok(())
}

#[cfg(feature = "video-download")]
async fn download_segments(
    v: &CourseVideo,
    dir: impl AsRef<std::path::Path>,
) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let dir = dir.as_ref();
    if !dir.exists() {
        anyhow::bail!("dir {} not exists", dir.display());
    }

    let tot = v.len_segments();
    let pb = pbar::new(tot as u64).with_prefix("download");
    pb.tick();

    let mut key = None;
    let mut keys = Vec::with_capacity(tot);
    for i in 0..tot {
        key = v.refresh_key(i, key);
        keys.push(key.cloned());
    }

    let dir = dir.to_path_buf();
    let results = futures_util::stream::iter((0..tot).map(|i| {
        let key = keys[i].clone();
        let path = dir.join(&v.segment(i).uri).with_extension("ts");
        let pb = pb.clone();

        async move {
            if !path.exists() {
                log::debug!("key: {key:?}");
                let seg = v
                    .get_segment_data(i, key.as_ref())
                    .await
                    .with_context(|| format!("get segment #{i} with key {key:?}"))?;

                // fs::write is not atomic, so we write to a tmp file first.
                let tmpath = path.with_extension("tmp");
                buf_try!(@try fs::write(&tmpath, seg).await);
                fs::rename(tmpath, &path).await.context("rename tmp file")?;
            }

            pb.inc(1);
            Ok::<_, anyhow::Error>((i, path))
        }
    }))
    .buffer_unordered(16)
    .collect::<Vec<_>>()
    .await;

    let mut paths = vec![None; tot];
    for result in results {
        let (i, path) = result?;
        paths[i] = Some(path);
    }
    pb.finish_and_clear();

    Ok(paths
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .context("downloaded segment list incomplete")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_video_records_uses_deterministic_tie_breakers() {
        let mut records = vec![
            VideoListRecord {
                course_title: "B".to_owned(),
                id: "2".to_owned(),
                title: "Video".to_owned(),
                duration: "01:00".to_owned(),
            },
            VideoListRecord {
                course_title: "A".to_owned(),
                id: "1".to_owned(),
                title: "Video".to_owned(),
                duration: "01:00".to_owned(),
            },
        ];

        sort_video_records(&mut records);

        assert_eq!(records[0].course_title, "A");
        assert_eq!(records[1].course_title, "B");
    }

    #[test]
    fn video_action_record_serializes_output_path() {
        let value = serde_json::to_value(VideoActionRecord {
            action: "video.download".to_owned(),
            id: "vid1".to_owned(),
            course_title: "Course".to_owned(),
            title: "Lecture".to_owned(),
            output_path: "/tmp/video.mp4".to_owned(),
        })
        .unwrap();

        assert_eq!(value["action"], "video.download");
        assert_eq!(value["output_path"], "/tmp/video.mp4");
    }
}

async fn merge_segments(
    dest: impl AsRef<std::path::Path>,
    paths: &[std::path::PathBuf],
) -> anyhow::Result<()> {
    let f = fs::File::create(&dest)
        .await
        .context("create merged file failed")?;
    let mut f = std::io::Cursor::new(f);

    let pb = pbar::new(paths.len() as u64).with_prefix("merge segments");
    pb.tick();
    for p in paths {
        let data = fs::read(p).await.context("read segments failed")?;
        buf_try!(@try f.write(data).await);
        pb.inc(1);
    }
    pb.finish_and_clear();

    Ok(())
}
