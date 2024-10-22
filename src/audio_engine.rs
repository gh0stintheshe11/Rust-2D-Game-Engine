use rodio::{OutputStream, OutputStreamHandle, Sink, Decoder};
use std::fs::File;
use std::io::BufReader;

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

    pub fn play_sound(&self, file_path: &str) {
        let file = BufReader::new(File::open(file_path).unwrap());
        let source = Decoder::new(file).unwrap();
        self.sink.append(source);
    }

    pub fn is_playing(&self) -> bool {
        !self.sink.empty()
    }
}