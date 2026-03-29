use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    sync::mpsc,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};
use ashpd::desktop::{
    PersistMode,
    screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType},
    screenshot::Screenshot,
};
use ashpd::enumflags2::BitFlags;
use chrono::Local;
use gst::prelude::*;
use gstreamer as gst;
use tokio::sync::oneshot;
use url::Url;

use crate::config::{self, AppConfig, CaptureKind, CaptureSource};

#[derive(Debug, Clone)]
pub enum CaptureOutcome {
    Finished(PathBuf),
    Recording(ActiveRecording),
}

#[derive(Debug, Clone)]
pub struct ActiveRecording {
    pub capture_kind: CaptureKind,
    pub output_path: PathBuf,
    stop_tx: Option<mpsc::Sender<WorkerCommand>>,
    done_rx: Arc<Mutex<Option<oneshot::Receiver<Result<PathBuf, String>>>>>,
    stop_delay_secs: u32,
}

#[derive(Debug)]
enum WorkerCommand {
    Stop,
}

#[derive(Debug, Clone)]
struct RecordingJob {
    capture_kind: CaptureKind,
    output_path: PathBuf,
    temp_video_path: Option<PathBuf>,
    pipewire_node_id: u32,
    pipewire_fd: i32,
    frame_rate: u32,
    selection: Option<SelectionRegion>,
}

#[derive(Debug, Clone, Copy)]
struct SelectionRegion {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    source_width: i32,
    source_height: i32,
}

#[derive(Debug, Clone, Copy)]
struct CropRegion {
    top: i32,
    right: i32,
    bottom: i32,
    left: i32,
}

pub async fn begin_capture(config: AppConfig) -> Result<CaptureOutcome> {
    config::ensure_output_dir(output_directory_for(&config, config.capture_kind))?;

    if config.start_delay_secs > 0 {
        tokio::time::sleep(Duration::from_secs(u64::from(config.start_delay_secs))).await;
    }

    match config.capture_kind {
        CaptureKind::Screenshot => {
            let path = take_screenshot(&config).await?;
            Ok(CaptureOutcome::Finished(path))
        }
        CaptureKind::Gif | CaptureKind::Video => {
            let recording = start_recording(&config).await?;
            Ok(CaptureOutcome::Recording(recording))
        }
    }
}

pub async fn stop_capture(mut recording: ActiveRecording) -> Result<PathBuf> {
    if recording.stop_delay_secs > 0 {
        tokio::time::sleep(Duration::from_secs(u64::from(recording.stop_delay_secs))).await;
    }

    if let Some(stop_tx) = recording.stop_tx.take() {
        stop_tx
            .send(WorkerCommand::Stop)
            .map_err(|_| anyhow!("Recording worker is no longer running"))?;
    }

    let done_rx = recording
        .done_rx
        .lock()
        .map_err(|_| anyhow!("Recording completion state is unavailable"))?
        .take()
        .ok_or_else(|| anyhow!("Recording stop has already been requested"))?;

    let result = done_rx
        .await
        .map_err(|_| anyhow!("Recording worker stopped unexpectedly"))?;

    result.map_err(anyhow::Error::msg)
}

async fn take_screenshot(config: &AppConfig) -> Result<PathBuf> {
    let interactive = matches!(config.capture_source, CaptureSource::Interactive);
    let response = Screenshot::request()
        .interactive(interactive)
        .modal(true)
        .send()
        .await?
        .response()?;

    let portal_path = uri_to_path(response.uri().as_str())?;
    let extension = portal_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("png");
    let target_path = config.screenshot_directory.join(format!(
        "taffy-{}.{}",
        Local::now().format("%Y%m%d-%H%M%S"),
        extension
    ));

    fs::copy(&portal_path, &target_path).with_context(|| {
        format!(
            "Failed to copy screenshot from {} to {}",
            portal_path.display(),
            target_path.display()
        )
    })?;

    Ok(target_path)
}

async fn start_recording(config: &AppConfig) -> Result<ActiveRecording> {
    if matches!(config.capture_source, CaptureSource::Interactive) && !binary_exists("slurp") {
        bail!(
            "Selection mode needs `slurp` right now for choosing the crop area. Install `slurp`, or switch to Whole Screen."
        );
    }

    let proxy = Screencast::new().await?;
    let session = proxy.create_session(Default::default()).await?;

    let source_types: BitFlags<SourceType> = SourceType::Monitor.into();
    let cursor_mode = if config.show_pointer {
        CursorMode::Embedded
    } else {
        CursorMode::Hidden
    };

    proxy
        .select_sources(
            &session,
            SelectSourcesOptions::default()
                .set_cursor_mode(cursor_mode)
                .set_sources(source_types)
                .set_multiple(false)
                .set_persist_mode(PersistMode::DoNot),
        )
        .await?;

    let response = proxy
        .start(&session, None, Default::default())
        .await?
        .response()?;
    let stream = response
        .streams()
        .first()
        .ok_or_else(|| anyhow!("No screen source was selected"))?
        .to_owned();

    let selection = if matches!(config.capture_source, CaptureSource::Interactive) {
        Some(select_crop_region(&stream)?)
    } else {
        None
    };

    let remote_fd = proxy
        .open_pipe_wire_remote(&session, Default::default())
        .await?;

    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let video_stem = format!("taffy-{timestamp}");

    let output_path = match config.capture_kind {
        CaptureKind::Gif => config.gif_directory.join(format!("{video_stem}.gif")),
        CaptureKind::Video => config.video_directory.join(format!("{video_stem}.mp4")),
        CaptureKind::Screenshot => unreachable!(),
    };

    let temp_video_path = match config.capture_kind {
        CaptureKind::Gif => Some(config.gif_directory.join(format!("{video_stem}.mp4"))),
        CaptureKind::Video if selection.is_some() => Some(
            config
                .video_directory
                .join(format!("{video_stem}.source.mp4")),
        ),
        CaptureKind::Video => None,
        CaptureKind::Screenshot => None,
    };

    let (stop_tx, stop_rx) = mpsc::channel();
    let (done_tx, done_rx) = oneshot::channel();

    let job = RecordingJob {
        capture_kind: config.capture_kind,
        output_path: output_path.clone(),
        temp_video_path,
        pipewire_node_id: stream.pipe_wire_node_id(),
        pipewire_fd: std::os::fd::AsRawFd::as_raw_fd(&remote_fd),
        frame_rate: config.frame_rate.max(1),
        selection,
    };

    thread::spawn(move || {
        let result = run_recording_worker(job, stop_rx).map_err(|error| format!("{error:#}"));
        let _ = done_tx.send(result);
        drop(remote_fd);
    });

    Ok(ActiveRecording {
        capture_kind: config.capture_kind,
        output_path,
        stop_tx: Some(stop_tx),
        done_rx: Arc::new(Mutex::new(Some(done_rx))),
        stop_delay_secs: config.stop_delay_secs,
    })
}

fn run_recording_worker(
    job: RecordingJob,
    stop_rx: mpsc::Receiver<WorkerCommand>,
) -> Result<PathBuf> {
    gst::init().context("Failed to initialize GStreamer")?;

    let recording_target = job
        .temp_video_path
        .clone()
        .unwrap_or_else(|| job.output_path.clone());

    let pipewire_source = gst::ElementFactory::make("pipewiresrc")
        .name("source")
        .property("fd", job.pipewire_fd)
        .property("path", job.pipewire_node_id.to_string())
        .build()
        .context("Failed to create pipewiresrc")?;

    let rate = gst::ElementFactory::make("videorate")
        .build()
        .context("Failed to create videorate")?;
    let convert = gst::ElementFactory::make("videoconvert")
        .build()
        .context("Failed to create videoconvert")?;
    let capsfilter = gst::ElementFactory::make("capsfilter")
        .property(
            "caps",
            gst::Caps::builder("video/x-raw")
                .field("framerate", gst::Fraction::new(job.frame_rate as i32, 1))
                .build(),
        )
        .build()
        .context("Failed to create capsfilter")?;
    let encoder = gst::ElementFactory::make("x264enc")
        .property_from_str("speed-preset", "veryfast")
        .property_from_str("tune", "zerolatency")
        .build()
        .context("Failed to create x264enc")?;
    let muxer = gst::ElementFactory::make("mp4mux")
        .build()
        .context("Failed to create mp4mux")?;
    let sink = gst::ElementFactory::make("filesink")
        .property("location", recording_target.to_string_lossy().to_string())
        .build()
        .context("Failed to create filesink")?;

    let pipeline = gst::Pipeline::default();
    pipeline
        .add_many([
            &pipewire_source,
            &rate,
            &convert,
            &capsfilter,
            &encoder,
            &muxer,
            &sink,
        ])
        .context("Failed to add elements to pipeline")?;
    gst::Element::link_many([
        &pipewire_source,
        &rate,
        &convert,
        &capsfilter,
        &encoder,
        &muxer,
        &sink,
    ])
    .context("Failed to link recording pipeline")?;

    pipeline
        .set_state(gst::State::Playing)
        .context("Failed to start recording pipeline")?;

    let bus = pipeline.bus().context("Failed to acquire pipeline bus")?;
    let mut sent_eos = false;

    loop {
        if !sent_eos && matches!(stop_rx.try_recv(), Ok(WorkerCommand::Stop)) {
            let _ = pipeline.send_event(gst::event::Eos::new());
            sent_eos = true;
        }

        if let Some(message) = bus.timed_pop(gst::ClockTime::from_mseconds(200)) {
            match message.view() {
                gst::MessageView::Eos(..) => break,
                gst::MessageView::Error(err) => {
                    pipeline
                        .set_state(gst::State::Null)
                        .context("Failed to reset pipeline after error")?;
                    bail!(
                        "Recording failed: {} ({})",
                        err.error(),
                        err.debug().unwrap_or_else(|| "no debug details".into())
                    );
                }
                _ => {}
            }
        }
    }

    pipeline
        .set_state(gst::State::Null)
        .context("Failed to stop recording pipeline")?;

    match job.capture_kind {
        CaptureKind::Gif => {
            convert_video_to_gif(
                &recording_target,
                &job.output_path,
                job.frame_rate,
                job.selection,
            )?;
            fs::remove_file(&recording_target).with_context(|| {
                format!(
                    "Failed to remove temporary file {}",
                    recording_target.display()
                )
            })?;
        }
        CaptureKind::Video => {
            if let Some(selection) = job.selection {
                crop_video(&recording_target, &job.output_path, selection)?;
                if recording_target != job.output_path {
                    fs::remove_file(&recording_target).with_context(|| {
                        format!(
                            "Failed to remove temporary file {}",
                            recording_target.display()
                        )
                    })?;
                }
            }
        }
        CaptureKind::Screenshot => unreachable!(),
    }

    Ok(job.output_path)
}

fn convert_video_to_gif(
    source: &PathBuf,
    target: &PathBuf,
    fps: u32,
    selection: Option<SelectionRegion>,
) -> Result<()> {
    let mut filters = Vec::new();
    if let Some(selection) = selection {
        let (width, height) = video_size(source)?;
        let crop = selection.to_crop_region(width, height);
        let crop_width = width - crop.left - crop.right;
        let crop_height = height - crop.top - crop.bottom;
        filters.push(format!(
            "crop={crop_width}:{crop_height}:{}:{}",
            crop.left, crop.top
        ));
    }

    filters.push(format!(
        "fps={fps},split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse=dither=bayer"
    ));

    let filter = filters.join(",");
    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            &source.to_string_lossy(),
            "-vf",
            &filter,
            &target.to_string_lossy(),
        ])
        .output()
        .context("Failed to run ffmpeg for GIF conversion")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("ffmpeg failed to create GIF: {stderr}");
    }

    Ok(())
}

fn crop_video(source: &PathBuf, target: &PathBuf, selection: SelectionRegion) -> Result<()> {
    let (width, height) = video_size(source)?;
    let crop = selection.to_crop_region(width, height);
    let crop_width = width - crop.left - crop.right;
    let crop_height = height - crop.top - crop.bottom;
    let filter = format!("crop={crop_width}:{crop_height}:{}:{}", crop.left, crop.top);

    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            &source.to_string_lossy(),
            "-vf",
            &filter,
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-pix_fmt",
            "yuv420p",
            &target.to_string_lossy(),
        ])
        .output()
        .context("Failed to crop the selected recording")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("ffmpeg failed to crop the selected recording: {stderr}");
    }

    Ok(())
}

fn uri_to_path(uri: &str) -> Result<PathBuf> {
    let url = Url::parse(uri).with_context(|| format!("Invalid URI returned by portal: {uri}"))?;
    url.to_file_path()
        .map_err(|_| anyhow!("Portal returned a non-file URI: {uri}"))
}

fn output_directory_for(config: &AppConfig, kind: CaptureKind) -> &PathBuf {
    match kind {
        CaptureKind::Screenshot => &config.screenshot_directory,
        CaptureKind::Gif => &config.gif_directory,
        CaptureKind::Video => &config.video_directory,
    }
}

fn select_crop_region(stream: &ashpd::desktop::screencast::Stream) -> Result<SelectionRegion> {
    let (stream_x, stream_y) = stream.position().ok_or_else(|| {
        anyhow!("The selected monitor did not report a position for region capture")
    })?;
    let (stream_width, stream_height) = stream
        .size()
        .ok_or_else(|| anyhow!("The selected monitor did not report a size for region capture"))?;

    let output = Command::new("slurp")
        .args(["-f", "%x %y %w %h"])
        .output()
        .context(
            "Failed to launch slurp for region selection. Install `slurp` to use Selection mode.",
        )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Region selection was cancelled or failed: {}",
            stderr.trim().if_empty_then("slurp did not return a region")
        );
    }

    let stdout = String::from_utf8(output.stdout).context("slurp returned invalid UTF-8")?;
    let geometry = stdout.trim();
    let parts = geometry
        .split_whitespace()
        .map(str::parse::<i32>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("Could not parse region geometry from slurp")?;

    if parts.len() != 4 {
        bail!("slurp returned unexpected region geometry: {geometry}");
    }

    let selection_x = parts[0];
    let selection_y = parts[1];
    let selection_width = parts[2].max(1);
    let selection_height = parts[3].max(1);

    let left = (selection_x - stream_x).clamp(0, stream_width.saturating_sub(1));
    let top = (selection_y - stream_y).clamp(0, stream_height.saturating_sub(1));
    let max_width = stream_width.saturating_sub(left).max(1);
    let max_height = stream_height.saturating_sub(top).max(1);
    let width = selection_width.min(max_width);
    let height = selection_height.min(max_height);

    Ok(SelectionRegion {
        left,
        top,
        width,
        height,
        source_width: stream_width,
        source_height: stream_height,
    })
}

impl SelectionRegion {
    fn to_crop_region(self, frame_width: i32, frame_height: i32) -> CropRegion {
        let scale_x = frame_width as f64 / self.source_width.max(1) as f64;
        let scale_y = frame_height as f64 / self.source_height.max(1) as f64;

        let left = ((self.left as f64) * scale_x).round() as i32;
        let top = ((self.top as f64) * scale_y).round() as i32;
        let width = ((self.width as f64) * scale_x).round() as i32;
        let height = ((self.height as f64) * scale_y).round() as i32;

        let left = left.clamp(0, frame_width.saturating_sub(1));
        let top = top.clamp(0, frame_height.saturating_sub(1));
        let width = width.clamp(1, frame_width.saturating_sub(left).max(1));
        let height = height.clamp(1, frame_height.saturating_sub(top).max(1));
        let right = (frame_width - (left + width)).max(0);
        let bottom = (frame_height - (top + height)).max(0);

        CropRegion {
            top,
            right,
            bottom,
            left,
        }
    }
}

fn video_size(source: &PathBuf) -> Result<(i32, i32)> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0:s=x",
            &source.to_string_lossy(),
        ])
        .output()
        .context("Failed to inspect recorded video dimensions with ffprobe")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("ffprobe could not read the recorded video dimensions: {stderr}");
    }

    let dims = String::from_utf8(output.stdout).context("ffprobe returned invalid UTF-8")?;
    let dims = dims.trim();
    let mut parts = dims.split('x');
    let width = parts
        .next()
        .ok_or_else(|| anyhow!("ffprobe did not return a video width"))?
        .parse::<i32>()
        .context("Could not parse recorded video width")?;
    let height = parts
        .next()
        .ok_or_else(|| anyhow!("ffprobe did not return a video height"))?
        .parse::<i32>()
        .context("Could not parse recorded video height")?;

    Ok((width, height))
}

trait EmptyFallback {
    fn if_empty_then<'a>(&'a self, fallback: &'a str) -> &'a str;
}

impl EmptyFallback for str {
    fn if_empty_then<'a>(&'a self, fallback: &'a str) -> &'a str {
        if self.trim().is_empty() {
            fallback
        } else {
            self
        }
    }
}

fn binary_exists(name: &str) -> bool {
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&paths).any(|path| path.join(name).is_file())
}
