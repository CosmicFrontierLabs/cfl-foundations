use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use v4l::buffer::Type;
use v4l::io::mmap::Stream as MmapStream;
use v4l::io::traits::CaptureStream;
use v4l::prelude::*;
use v4l::video::Capture;

#[derive(Debug, Clone)]
pub struct CameraConfig {
    pub device_path: String,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub gain: i32,
    pub exposure: i32,
    pub black_level: i32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            device_path: "/dev/video0".to_string(),
            width: 1024,
            height: 1024,
            framerate: 23_000_000,
            gain: 360,
            exposure: 140,
            black_level: 4095,
        }
    }
}

pub struct V4L2Capture {
    config: CameraConfig,
}

impl V4L2Capture {
    pub fn new(config: CameraConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub fn open_device(&self) -> Result<Device> {
        Device::with_path(&self.config.device_path)
            .with_context(|| format!("Failed to open device: {}", self.config.device_path))
    }

    pub fn configure_device(&self, device: &mut Device) -> Result<()> {
        let mut format = device.format()?;
        format.width = self.config.width;
        format.height = self.config.height;
        format.fourcc = v4l::FourCC::new(b"Y16 ");
        device.set_format(&format)?;

        if let Ok(controls) = device.query_controls() {
            for control_desc in controls {
                match control_desc.name.as_str() {
                    "Gain" | "gain" => {
                        let ctrl = v4l::Control {
                            id: control_desc.id,
                            value: v4l::control::Value::Integer(self.config.gain as i64),
                        };
                        let _ = device.set_control(ctrl);
                    }
                    "Exposure" | "exposure" => {
                        let ctrl = v4l::Control {
                            id: control_desc.id,
                            value: v4l::control::Value::Integer(self.config.exposure as i64),
                        };
                        let _ = device.set_control(ctrl);
                    }
                    "Black Level" | "black_level" => {
                        let ctrl = v4l::Control {
                            id: control_desc.id,
                            value: v4l::control::Value::Integer(self.config.black_level as i64),
                        };
                        let _ = device.set_control(ctrl);
                    }
                    "Frame Rate" | "frame_rate" => {
                        let ctrl = v4l::Control {
                            id: control_desc.id,
                            value: v4l::control::Value::Integer(self.config.framerate as i64),
                        };
                        let _ = device.set_control(ctrl);
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    pub fn capture_single_frame(&self) -> Result<Vec<u8>> {
        let mut device = self.open_device()?;
        self.configure_device(&mut device)?;

        let mut stream = MmapStream::new(&device, Type::VideoCapture)?;
        let (buf, _meta) = stream.next()?;
        Ok(buf.to_vec())
    }

    pub fn capture_frames_with_skip(&self, count: usize, skip: usize) -> Result<Vec<Vec<u8>>> {
        let mut device = self.open_device()?;
        self.configure_device(&mut device)?;

        let mut stream = MmapStream::new(&device, Type::VideoCapture)?;
        let mut frames = Vec::new();

        for _ in 0..skip {
            let _ = stream.next()?;
        }

        for _ in 0..count {
            let (buf, _meta) = stream.next()?;
            frames.push(buf.to_vec());
        }

        Ok(frames)
    }
}

pub struct CaptureSession<'a> {
    device: Device,
    stream: Option<MmapStream<'a>>,
}

impl<'a> CaptureSession<'a> {
    pub fn new(config: &CameraConfig) -> Result<Self>
    where
        Self: 'a,
    {
        let capture = V4L2Capture::new(config.clone())?;
        let mut device = capture.open_device()?;
        capture.configure_device(&mut device)?;

        Ok(Self {
            device,
            stream: None,
        })
    }

    pub fn start_stream(&mut self) -> Result<()> {
        let stream = MmapStream::new(&self.device, Type::VideoCapture)?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn capture_frame(&mut self) -> Result<Vec<u8>> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Stream not started"))?;

        let (buf, _meta) = stream.next()?;
        Ok(buf.to_vec())
    }

    pub fn stop_stream(&mut self) {
        self.stream.take();
    }

    pub fn save_raw_frame(&mut self, path: &Path) -> Result<()> {
        let frame = self.capture_frame()?;
        std::fs::write(path, frame)?;
        Ok(())
    }
}

pub struct FrameBuffer {
    frames: Arc<Mutex<Vec<Vec<u8>>>>,
    max_frames: usize,
}

impl FrameBuffer {
    pub fn new(max_frames: usize) -> Self {
        Self {
            frames: Arc::new(Mutex::new(Vec::new())),
            max_frames,
        }
    }

    pub async fn push(&self, frame: Vec<u8>) {
        let mut frames = self.frames.lock().await;
        if frames.len() >= self.max_frames {
            frames.remove(0);
        }
        frames.push(frame);
    }

    pub async fn get_latest(&self) -> Option<Vec<u8>> {
        let frames = self.frames.lock().await;
        frames.last().cloned()
    }

    pub async fn get_all(&self) -> Vec<Vec<u8>> {
        self.frames.lock().await.clone()
    }

    pub async fn clear(&self) {
        self.frames.lock().await.clear();
    }

    pub async fn len(&self) -> usize {
        self.frames.lock().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.frames.lock().await.is_empty()
    }
}

pub struct ResolutionProfile {
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub test_frames: u32,
}

impl ResolutionProfile {
    pub fn standard_profiles() -> Vec<Self> {
        vec![
            Self {
                width: 128,
                height: 128,
                framerate: 133_000_000,
                test_frames: 134,
            },
            Self {
                width: 256,
                height: 256,
                framerate: 83_000_000,
                test_frames: 84,
            },
            Self {
                width: 512,
                height: 512,
                framerate: 44_000_000,
                test_frames: 45,
            },
            Self {
                width: 1024,
                height: 1024,
                framerate: 23_000_000,
                test_frames: 24,
            },
            Self {
                width: 2048,
                height: 2048,
                framerate: 12_000_000,
                test_frames: 13,
            },
            Self {
                width: 4096,
                height: 4096,
                framerate: 6_000_000,
                test_frames: 7,
            },
            Self {
                width: 8096,
                height: 6324,
                framerate: 3_700_000,
                test_frames: 4,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_config_default() {
        let config = CameraConfig::default();
        assert_eq!(config.device_path, "/dev/video0");
        assert_eq!(config.width, 1024);
        assert_eq!(config.height, 1024);
    }

    #[test]
    fn test_resolution_profiles() {
        let profiles = ResolutionProfile::standard_profiles();
        assert_eq!(profiles.len(), 7);
        assert_eq!(profiles[0].width, 128);
        assert_eq!(profiles[6].width, 8096);
    }

    #[tokio::test]
    async fn test_frame_buffer() {
        let buffer = FrameBuffer::new(3);

        buffer.push(vec![1, 2, 3]).await;
        buffer.push(vec![4, 5, 6]).await;
        buffer.push(vec![7, 8, 9]).await;

        assert_eq!(buffer.len().await, 3);

        buffer.push(vec![10, 11, 12]).await;
        assert_eq!(buffer.len().await, 3);

        let latest = buffer.get_latest().await;
        assert_eq!(latest, Some(vec![10, 11, 12]));
    }
}
