use color_eyre::eyre::eyre;
use serde::Deserialize;
use serde_json::value::RawValue;

use crate::Provider;
use std::{
    borrow::Cow,
    io::{self, BufRead, BufReader, Write},
    net::Shutdown,
    os::unix::net::UnixStream,
};

pub struct Niri {
    event_stream: BufReader<UnixStream>,
    requests: UnixStream,
    buffer: String,
}

#[derive(clap::Args)]
pub struct NiriArgs {
    #[arg(long)]
    /// Niri socket address
    socket: Option<Cow<'static, str>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Focus<'a> {
    #[serde(borrow)]
    focused_window: Option<&'a RawValue>,
}

impl Provider for Niri {
    type Args = NiriArgs;
    type Fmt<'a> = &'a str;
    fn init(args: Self::Args) -> color_eyre::Result<Niri> {
        let addr = args
            .socket
            .or_else(|| std::env::var("NIRI_SOCKET").map(Cow::Owned).ok())
            .ok_or_else(|| {
                eyre!("can't find socket addr, either pass --socket or set NIRI_SOCKET")
            })?;

        let mut event_stream = UnixStream::connect(&*addr)?;
        let requests = UnixStream::connect(&*addr)?;

        writeln!(event_stream, r#""EventStream""#)?;
        event_stream.shutdown(Shutdown::Write)?;

        let mut event_stream = BufReader::new(event_stream);
        event_stream.skip_until(b'\n')?;

        Ok(Niri {
            event_stream,
            requests,
            buffer: String::new(),
        })
    }
    fn run(mut self) -> color_eyre::Result<()> {
        let buffer = &mut self.buffer;
        let mut stdout = io::stdout().lock();

        let format = get_window(&mut self.requests, buffer)?;
        writeln!(&mut stdout, "{}", &format)?;
        stdout.flush()?;

        loop {
            buffer.clear();
            self.event_stream.read_line(buffer)?;
            let value: serde_json::Value = serde_json::from_str(buffer)?;

            if value.get("WindowFocusChanged").is_some() {
                let format = get_window(&mut self.requests, buffer)?;
                writeln!(&mut stdout, "{}", &format)?;
                stdout.flush()?;
            }
        }
    }
    fn update(&mut self) -> color_eyre::Result<()> {
        unreachable!()
    }

    fn duration(&self) -> Option<std::time::Duration> {
        unreachable!()
    }

    fn format<'a>(&'a self) -> color_eyre::Result<Self::Fmt<'a>> {
        unreachable!()
    }
}

fn get_window<'a>(
    requests: &mut UnixStream,
    buffer: &'a mut String,
) -> color_eyre::Result<&'a str> {
    writeln!(requests, r#""FocusedWindow""#)?;
    buffer.clear();
    BufReader::new(requests).read_line(buffer)?;
    let focus = serde_json::from_str::<Result<Focus, String>>(buffer)?.map_err(|e| eyre!("{e}"))?;
    Ok(focus.focused_window.map(RawValue::get).unwrap_or("{}"))
}
