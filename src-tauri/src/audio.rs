// VoiceType - Simple audio module
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, SupportedStreamConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct RecordingSession {
    pub id: String,
    pub file_path: PathBuf,
    pub start_time: chrono::DateTime<chrono::Utc>,
    _stream: cpal::Stream,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopRecordingResult {
    pub audio_path: String,
    pub duration: u64,
}

pub struct AudioManager {
    pub current_session: Arc<Mutex<Option<RecordingSession>>>,
    pub temp_dir: PathBuf,
}

unsafe impl Send for AudioManager {}

impl AudioManager {
    pub fn new() -> Result<Self, String> {
        let temp_dir = std::env::temp_dir().join("voicetype");
        std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        eprintln!("[Audio] Temp dir: {}", temp_dir.display());

        // Test audio device availability
        let host = cpal::default_host();
        let _device = host.default_input_device()
            .ok_or_else(|| "找不到麥克風設備".to_string())?;
        eprintln!("[Audio] Default input device found");

        Ok(Self {
            current_session: Arc::new(Mutex::new(None)),
            temp_dir,
        })
    }

    pub fn start_recording(&self, _device_name: Option<String>) -> Result<String, String> {
        let mut session = self.current_session.lock().map_err(|e| e.to_string())?;
        if session.is_some() {
            return Err("已在錄音中".to_string());
        }

        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| "找不到麥克風".to_string())?;
        let config = device.default_input_config()
            .map_err(|e| format!("無法取得設備配置: {}", e))?;

        eprintln!("[Audio] Config: channels={}, rate={}, format={:?}",
            config.channels(), config.sample_rate().0, config.sample_format());

        let recording_id = format!("rec_{}", chrono::Utc::now().timestamp_millis());
        let file_path = self.temp_dir.join(format!("{}.wav", recording_id));
        eprintln!("[Audio] Recording to: {}", file_path.display());

        let spec = hound::WavSpec {
            channels: config.channels() as _,
            sample_rate: config.sample_rate().0,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let writer = Arc::new(Mutex::new(
            hound::WavWriter::create(&file_path, spec)
                .map_err(|e| format!("無法建立 WAV 檔案: {}", e))?
        ));

        let stream_config: cpal::StreamConfig = config.clone().into();
        let err_fn = |err| eprintln!("[Audio] Stream error: {}", err);

        let stream = match config.sample_format() {
            SampleFormat::I16 => {
                let writer = Arc::clone(&writer);
                device.build_input_stream(&stream_config, move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut w) = writer.lock() {
                        for &s in data { w.write_sample(s).ok(); }
                    }
                }, err_fn, None)
            }
            SampleFormat::U16 => {
                let writer = Arc::clone(&writer);
                device.build_input_stream(&stream_config, move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut w) = writer.lock() {
                        for &s in data { w.write_sample(s as i16).ok(); }
                    }
                }, err_fn, None)
            }
            SampleFormat::F32 => {
                let writer = Arc::clone(&writer);
                device.build_input_stream(&stream_config, move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut w) = writer.lock() {
                        for &s in data { w.write_sample((s * 32767.0) as i16).ok(); }
                    }
                }, err_fn, None)
            }
            fmt => return Err(format!("不支援的格式: {:?}", fmt)),
        }.map_err(|e| format!("無法建立錄音串流: {}", e))?;

        stream.play().map_err(|e| format!("無法開始錄音: {}", e))?;

        *session = Some(RecordingSession {
            id: recording_id.clone(),
            file_path,
            start_time: chrono::Utc::now(),
            _stream: stream,
        });

        eprintln!("[Audio] Recording started: {}", recording_id);
        Ok(recording_id)
    }

    pub fn stop_recording(&self) -> Result<StopRecordingResult, String> {
        let mut session = self.current_session.lock().map_err(|e| e.to_string())?;
        let rec = session.take().ok_or_else(|| "沒有在錄音".to_string())?;
        drop(rec._stream);
        let duration = (chrono::Utc::now() - rec.start_time).num_seconds() as u64;
        eprintln!("[Audio] Recording stopped: {}s", duration);
        Ok(StopRecordingResult {
            audio_path: rec.file_path.to_string_lossy().to_string(),
            duration,
        })
    }

    pub fn list_device_names(&self) -> Result<Vec<String>, String> {
        let host = cpal::default_host();
        let devices = host.input_devices().map_err(|e| e.to_string())?;
        Ok(devices.filter_map(|d| d.name().ok()).collect())
    }
}
