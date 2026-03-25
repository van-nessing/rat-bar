use std::{
    io::{self, Write},
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

use crossbeam::atomic::AtomicCell;
use itertools::Itertools;
use pipewire::{
    self as pw,
    main_loop::MainLoopRc,
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
use rustfft::num_complex::Complex32;

use crate::Provider;

pub struct Visualizer {
    duration: Duration,
    sample_count: usize,
    bin_count: usize,
    amps: Vec<Vec<f32>>,
    amp_average: Vec<f32>,
    cell: Arc<AtomicCell<Option<(Vec<f32>, u32)>>>,
    running: Arc<()>,
}

#[derive(clap::Args)]
pub struct VisualizerArgs {
    #[arg(value_parser = humantime::parse_duration)]
    /// Amount of time between writing to stdout
    duration: Duration,
    #[arg(long, short, default_value_t = 1024)]
    /// Minimum number of samples before processing (should probably be a power of 2)
    sample_count: usize,
    #[arg(long, short, default_value_t = 8)]
    /// Number of past bins to store for smooth falloff
    history_count: usize,
    #[arg(long, short, default_value_t = 32)]
    /// Number of total bins to store
    bin_count: usize,
    #[arg(long, short, default_value_t = 256)]
    /// Number of average amplitudes to store for amplitude averaging
    average_count: usize,
}

#[derive(serde::Serialize)]
pub struct VisualizerFormat<'a> {
    bins: &'a [f32],
}

struct UserData {
    format: spa::param::audio::AudioInfoRaw,
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

impl Provider for Visualizer {
    type Args = VisualizerArgs;
    type Fmt<'a> = VisualizerFormat<'a>;

    fn init(args: Self::Args) -> color_eyre::Result<Self> {
        let visualizer = Visualizer {
            sample_count: args.sample_count,
            duration: args.duration,
            bin_count: args.bin_count,
            amp_average: vec![0.0; args.average_count],
            amps: vec![Vec::new(); args.history_count],
            cell: Arc::new(AtomicCell::new(None)),
            running: Arc::new(()),
        };

        let cell = Arc::clone(&visualizer.cell);
        let running = Arc::clone(&visualizer.running);

        std::thread::spawn(move || -> color_eyre::Result<()> {
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
                        .batching(|a| {
                            a.take(n_channels as usize)
                                .map(|n| {
                                    let start = n as usize * sample_size;
                                    let end = start + sample_size;
                                    let chan = &samples[start..end];
                                    f32::from_le_bytes(chan.try_into().unwrap())
                                })
                                .reduce(std::ops::Add::add)
                        })
                        .collect::<Vec<_>>();
                    cell.store(Some((samples, user_data.format.rate())));
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

                while Arc::strong_count(&running) > 1 {
                    loop_.iterate_unguarded(Duration::from_millis(100));
                }

                loop_.leave();
            }
            Ok(())
        });
        Ok(visualizer)
    }
    fn run(mut self) -> color_eyre::Result<()> {
        let (buffer, _) = loop {
            if let Some(buffer) = self.cell.swap(None) {
                break buffer;
            }
            sleep(Duration::from_secs_f32(0.1));
        };

        let mut scratch_buffer = buffer
            .iter()
            .map(|_| Complex32 { re: 0.0, im: 0.0 })
            .collect::<Vec<_>>();
        let mut sample_buffer = scratch_buffer.clone();
        let mut planner = rustfft::FftPlanner::new();
        let mut last_update = Instant::now();
        let mut acc_buffer = Vec::<f32>::new();
        let mut stdout = io::stdout().lock();

        loop {
            let now = Instant::now();
            let Some((samples, _rate)) = self.cell.swap(None) else {
                sleep(self.duration / 10);
                continue;
            };
            acc_buffer.extend(samples.iter());

            if acc_buffer.len() < self.sample_count {
                continue;
            }
            if now.duration_since(last_update) <= self.duration {
                continue;
            }
            last_update = now;

            sample_buffer.clear();
            sample_buffer.extend(acc_buffer.drain(..).map(|re| Complex32 { re, im: 0.0 }));
            scratch_buffer.resize(sample_buffer.len(), Complex32 { re: 0.0, im: 0.0 });

            let plan = planner.plan_fft_forward(scratch_buffer.len());
            let han = window_iter(sample_buffer.len());

            sample_buffer
                .iter_mut()
                .zip(han)
                .for_each(|(sample, han)| sample.re *= han);

            plan.process_with_scratch(&mut sample_buffer, &mut scratch_buffer);

            let mut frequencies = sample_buffer
                .iter()
                .take(sample_buffer.len())
                .map(|c| c.re.powi(2) * 75.0)
                .collect::<Vec<_>>();

            frequencies.truncate(128);
            self.amp_average.rotate_right(1);
            self.amp_average[0] = frequencies.iter().sum::<f32>() / frequencies.len() as f32;

            let scale = 0.9;
            self.amps.rotate_right(1);
            self.amps
                .iter_mut()
                .for_each(|bins| bins.iter_mut().for_each(|bin| *bin *= scale));
            self.amps[0] = frequencies;

            let len = self.amps.iter().map(Vec::len).max().unwrap_or(0);

            let summed = (0..len)
                .tuple_windows()
                .map(|(a, b)| {
                    let deltas = self.amps.iter().flat_map(|freqs| {
                        let a = freqs.get(a)?;
                        let b = freqs.get(b)?;

                        Some((a - b).abs())
                    });
                    let average =
                        self.amps.iter().flat_map(|freqs| freqs.get(a)).sum::<f32>() / len as f32;
                    let deltas = deltas.max_by(|a, b| a.total_cmp(b)).unwrap_or(0.0);
                    deltas + average
                })
                .collect::<Vec<_>>();

            let average = self.amp_average.iter().sum::<f32>() / self.amp_average.len() as f32;

            let bins = summed
                .into_iter()
                .take(self.bin_count)
                .enumerate()
                .map(|(i, v)| v * i as f32)
                .map(|v| v / average)
                .collect::<Vec<_>>();

            let format = VisualizerFormat { bins: &bins };

            serde_json::to_writer(&mut stdout, &format)?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
    }
    fn duration(&self) -> Option<Duration> {
        Some(self.duration)
    }
    fn update(&mut self) -> color_eyre::Result<()> {
        unreachable!()
    }
    fn format<'a>(&'a self) -> color_eyre::Result<Self::Fmt<'a>> {
        unreachable!()
    }
}
