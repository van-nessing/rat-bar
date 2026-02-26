use std::{
    iter,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use color_eyre::eyre::eyre;
use itertools::Itertools;
use pipewire::{
    self as pw,
    main_loop::{MainLoopBox, MainLoopRc},
    spa::{
        self,
        param::{
            audio::AudioInfoRaw,
            format::{MediaSubtype, MediaType},
            format_utils,
        },
        pod::Pod,
    },
};
use pipewire::{context::ContextBox, properties::properties};
use ratatui::widgets::{Paragraph, Widget};
use rustfft::num_complex::Complex32;
use tokio::sync::mpsc::Sender;

use crate::{
    event::Event,
    widgets::{bar_graph::BarGraph, graph::GraphWidget},
};

pub struct Visualizer<'a> {
    pub meta: &'a VisualizerMeta,
}
#[derive(Debug)]
pub struct VisualizerMeta {
    pub data: Vec<Vec<f32>>,
    pub amp_average: Vec<f32>,
    pub sample_rate: u32,
}

struct UserData {
    format: spa::param::audio::AudioInfoRaw,
}
/// iteration state of an iterator for a cosine window
#[derive(Clone, Debug)]
pub struct CosineWindowIter {
    /// coefficient `a` of the cosine window
    pub a: f32,
    /// coefficient `b` of the cosine window
    pub b: f32,
    /// coefficient `c` of the cosine window
    pub c: f32,
    /// coefficient `d` of the cosine window
    pub d: f32,
    /// the current `index` of the iterator
    pub index: usize,
    /// `size` of the cosine window
    pub size: usize,
}

impl VisualizerMeta {
    pub fn new(average: usize, amp_average: usize) -> Self {
        Self {
            data: vec![Vec::new(); average],
            amp_average: vec![10.0; amp_average],
            sample_rate: 1,
        }
    }
}
fn window_iter(len: usize) -> impl Iterator<Item = f32> {
    let a = 0.35875;
    let b = 0.48829;
    let c = 0.14128;
    let d = 0.01168;
    let cosine = move |i: usize| {
        let x = (std::f32::consts::PI * i as f32) / (len - 1) as f32;
        let b_ = b * (2.0 * x).cos();
        let c_ = c * (4.0 * x).cos();
        let d_ = d * (6.0 * x).cos();
        (a - b_) + (c_ - d_)
    };

    (0..len).map(cosine)
}
pub async fn visualizer_events(
    sender: Sender<Event>,
    running: Arc<AtomicBool>,
) -> color_eyre::Result<()> {
    let (sample_sender, mut sample_receiver) = tokio::sync::mpsc::channel(32);

    tokio::task::spawn_blocking(move || -> color_eyre::Result<()> {
        let mainloop = MainLoopRc::new(None)?;
        let context = ContextBox::new(mainloop.loop_(), None)?;
        let core = context.connect(None)?;
        // let registry = core.get_registry()?;

        let mut props = properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Music",
            // *pw::keys::NODE_NAME=> "rat_bar_visualizer",
        };

        props.insert(*pw::keys::STREAM_CAPTURE_SINK, "true");
        let stream = pw::stream::StreamBox::new(&core, "audio-capture", props)?;
        let _listener = stream
            .add_local_listener_with_user_data(UserData {
                format: AudioInfoRaw::default(),
            })
            .param_changed(|_, user_data, id, param| {
                // NULL means to clear the format
                let Some(param) = param else {
                    return;
                };
                if id != pw::spa::param::ParamType::Format.as_raw() {
                    return;
                }

                let (media_type, media_subtype) = match format_utils::parse_format(param) {
                    Ok(v) => v,
                    Err(_) => return,
                };

                // only accept raw audio
                if media_type != MediaType::Audio || media_subtype != MediaSubtype::Raw {
                    return;
                }

                // call a helper function to parse the format for us.
                let _ = user_data.format.parse(param);
            })
            .process(move |stream, user_data| {
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return;
                };
                let datas = buffer.datas_mut();
                if datas.is_empty() {
                    return;
                }
                let sample_size = std::mem::size_of::<f32>();
                let data = &mut datas[0];
                let n_channels = user_data.format.channels();
                let n_samples = data.chunk().size() / (sample_size as u32);

                let Some(samples) = data.data() else { return };
                let samples = (0..n_samples)
                    .chunks(n_channels as usize)
                    .into_iter()
                    .map(|channels| {
                        channels
                            .map(|n| {
                                let start = n as usize * sample_size;
                                let end = start + sample_size;
                                let chan = &samples[start..end];
                                f32::from_le_bytes(chan.try_into().unwrap())
                            })
                            .sum()
                    })
                    .collect::<Vec<_>>();
                let _ = sample_sender.blocking_send((samples, user_data.format.rate()));
            })
            .register()?;

        let mut audio_info = spa::param::audio::AudioInfoRaw::new();
        audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
        let obj = pw::spa::pod::Object {
            type_: pw::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
            id: pw::spa::param::ParamType::EnumFormat.as_raw(),
            properties: audio_info.into(),
        };
        let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
            std::io::Cursor::new(Vec::new()),
            &pw::spa::pod::Value::Object(obj),
        )
        .unwrap()
        .0
        .into_inner();

        let mut params = [Pod::from_bytes(&values).unwrap()];

        /* Now connect this stream. We ask that our process function is
         * called in a realtime thread. */
        stream.connect(
            spa::utils::Direction::Input,
            None,
            pw::stream::StreamFlags::AUTOCONNECT
                | pw::stream::StreamFlags::MAP_BUFFERS
                | pw::stream::StreamFlags::RT_PROCESS,
            &mut params,
        )?;

        let loop_ = mainloop.loop_();
        unsafe {
            loop_.enter();

            while running.load(Ordering::Relaxed) && Arc::strong_count(&running) > 1 {
                loop_.iterate_unguarded(Duration::from_millis(100));
            }

            loop_.leave();
        }
        Ok(())
    });

    let mut scratch_buffer = sample_receiver
        .recv()
        .await
        .ok_or_else(|| eyre!("channel closed"))?
        .0
        .iter()
        .map(|_| Complex32 { re: 0.0, im: 0.0 })
        .collect::<Vec<_>>();

    let mut sample_buffer = scratch_buffer.clone();
    let mut planner = rustfft::FftPlanner::new();
    let update_frequency = Duration::from_secs(1) / 60;
    let mut last_update = Instant::now();
    let mut acc_buffer = Vec::<f32>::new();

    loop {
        let (frequencies, sample_rate) = sample_receiver
            .recv()
            .await
            .ok_or_else(|| eyre!("channel closed"))?;
        acc_buffer.extend(frequencies.iter());
        if acc_buffer.len() < 1024 {
            continue;
        }

        let now = Instant::now();

        if now.duration_since(last_update) <= update_frequency {
            continue;
        }

        last_update = now;

        sample_buffer.clear();
        sample_buffer.extend(acc_buffer.drain(..).map(|re| Complex32 { re, im: 0.0 }));
        scratch_buffer.resize(sample_buffer.len(), Complex32 { re: 0.0, im: 0.0 });

        (sample_buffer, scratch_buffer, planner) = tokio::task::spawn_blocking(|| {
            let mut sample_buffer = sample_buffer;
            let mut scratch_buffer = scratch_buffer;
            let mut planner = planner;

            let plan = planner.plan_fft_forward(scratch_buffer.len());
            let han = window_iter(sample_buffer.len());

            sample_buffer
                .iter_mut()
                .zip(han)
                .for_each(|(sample, han)| sample.re *= han);

            plan.process_with_scratch(&mut sample_buffer, &mut scratch_buffer);

            (sample_buffer, scratch_buffer, planner)
        })
        .await?;

        let frequencies = sample_buffer
            .iter()
            .take(sample_buffer.len())
            .map(|c| c.re.powi(2) * 75.0)
            .collect::<Vec<_>>();

        sender
            .send(Event::SendAudioSample {
                frequencies,
                sample_rate,
            })
            .await?;
    }
}
impl Widget for &Visualizer<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let Some(max_len) = self.meta.data.iter().map(Vec::len).max() else {
            return;
        };
        if max_len == 0 {
            return;
        }

        let summed = (0..max_len)
            .take(area.width as usize)
            .tuple_windows()
            .map(|(a, b)| {
                let deltas = self.meta.data.iter().flat_map(|freqs| {
                    let a = freqs.get(a)?;
                    let b = freqs.get(b)?;

                    Some((a - b).abs())
                });
                let average = self
                    .meta
                    .data
                    .iter()
                    .flat_map(|freqs| freqs.get(a))
                    .sum::<f32>()
                    / max_len as f32;
                let deltas = deltas.max_by(|a, b| a.total_cmp(b)).unwrap_or(0.0);
                deltas + average
            })
            .collect::<Vec<_>>();

        let average =
            self.meta.amp_average.iter().sum::<f32>() / self.meta.amp_average.len() as f32;

        let summed = summed
            .into_iter()
            // .take(area.width as usize)
            .enumerate()
            .map(|(i, v)| v * i as f32)
            .map(|v| (v / average))
            .collect::<Vec<_>>();

        GraphWidget {
            percentages: summed.as_slice(),
            datapoint_count: summed.len(),
        }
        .render(area, buf);
    }
}
