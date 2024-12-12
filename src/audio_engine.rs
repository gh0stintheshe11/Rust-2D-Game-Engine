use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub struct AudioEngine {
    stream: OutputStream,
    stream_handle: OutputStreamHandle,
    pub sink: Sink,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        AudioEngine {
            stream,
            stream_handle,
            sink,
        }
    }

    pub fn play_sound(&self, file_path: &str) -> Result<(), String> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(format!("File not found: {}", file_path));
        }

        let file = BufReader::new(File::open(path).map_err(|e| e.to_string())?);
        let source = Decoder::new(file).map_err(|e| e.to_string())?;
        self.sink.append(source);
        Ok(())
    }

    pub fn is_playing(&self) -> bool {
        !self.sink.empty() && !self.sink.is_paused()
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn resume(&self) {
        self.sink.play();
    }
}
