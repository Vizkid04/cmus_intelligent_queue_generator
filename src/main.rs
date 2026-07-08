use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;
use rusqlite::{Connection, Result};
use rayon::prelude::*;

use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use lofty::prelude::*;
use lofty::probe::Probe;

use rustfft::{FftPlanner, num_complex::Complex};

fn decode_to_mono(path: &Path) -> Option<(Vec<f32>, u32)> {
    let file = File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .ok()?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)?;
    
    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .ok()?;

    let mut samples = Vec::new();

    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id { continue; }
        if let Ok(decoded) = decoder.decode(&packet) {
            match decoded {
                AudioBufferRef::F32(buf) => {
                    let n_ch = buf.spec().channels.bits().count_ones() as usize;
                    let ch0 = buf.chan(0);
                    if n_ch > 1 {
                        let ch1 = buf.chan(1);
                        samples.extend((0..ch0.len()).map(|i| (ch0[i] + ch1[i]) / 2.0));
                    } else {
                        samples.extend_from_slice(ch0);
                    }
                }
                AudioBufferRef::S16(buf) => {
                    let n_ch = buf.spec().channels.bits().count_ones() as usize;
                    let ch0 = buf.chan(0);
                    if n_ch > 1 {
                        let ch1 = buf.chan(1);
                        samples.extend((0..ch0.len()).map(|i| {
                            let l = ch0[i] as f32 / i16::MAX as f32;
                            let r = ch1[i] as f32 / i16::MAX as f32;
                            (l + r) / 2.0
                        }));
                    } else {
                        samples.extend(ch0.iter().map(|&s| s as f32 / i16::MAX as f32));
                    }
                }
                _ => continue,
            }
        }
    }

    Some((samples, sample_rate))
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() { return 0.0; }
    let sum_sq: f32 = samples.iter().map(|x| x * x).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

fn zero_crossing_rate(samples: &[f32]) -> f32 {
    if samples.len() < 2 { return 0.0; }
    let crossings = samples
        .windows(2)
        .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
        .count();
    crossings as f32 / samples.len() as f32
}

fn compute_spectral_features(samples: &[f32], sample_rate: u32) -> (f32, f32) {
    if samples.len() < 2048 { return (0.0, 0.0); }
    
    let window_size = 2048;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);
    
    let mut centroids = Vec::new();
    
    for chunk in samples.chunks_exact(window_size) {
        let mut buffer: Vec<Complex<f32>> = chunk.iter().map(|&x| Complex::new(x, 0.0)).collect();
        fft.process(&mut buffer);
        
        let mut num = 0.0;
        let mut den = 0.0;
        
        for i in 0..(window_size / 2) {
            let magnitude = buffer[i].norm();
            let frequency = (i as f32 * sample_rate as f32) / window_size as f32;
            
            num += frequency * magnitude;
            den += magnitude;
        }
        
        if den > 0.0 {
            centroids.push(num / den);
        }
    }
    
    if centroids.is_empty() { return (0.0, 0.0); }
    
    let mean_centroid: f32 = centroids.iter().sum::<f32>() / centroids.len() as f32;
    let variance: f32 = centroids.iter().map(|x| (x - mean_centroid).powi(2)).sum::<f32>() / centroids.len() as f32;
    
    (mean_centroid, variance.sqrt())
}

fn init_db() -> Result<Connection> {
    let conn = Connection::open("music_brain.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tracks (
            filepath TEXT PRIMARY KEY,
            title TEXT,
            artist TEXT,
            album TEXT,
            genre TEXT,
            duration REAL,
            sample_rate INTEGER,
            rms_energy REAL,
            zero_crossing_rate REAL,
            spectral_centroid REAL,
            spectral_variance REAL,
            play_count INTEGER DEFAULT 0,
            user_rating INTEGER DEFAULT 0
        )",
        [],
    )?;
    Ok(conn)
}

fn track_exists(conn: &Connection, filepath: &str) -> bool {
    let mut stmt = conn.prepare("SELECT 1 FROM tracks WHERE filepath = ?1").unwrap();
    stmt.exists([filepath]).unwrap_or(false)
}

fn main() -> Result<()> {
    let conn = init_db()?;
    let music_dir = "/home/vizkid/Music"; 

    println!("Collecting audio files from {}...", music_dir);
    let mut files_to_process = Vec::new();

    for entry in WalkDir::new(music_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "mp3" || ext == "flac") {
            let filepath_str = path.to_string_lossy().to_string();
            if !track_exists(&conn, &filepath_str) {
                files_to_process.push(path.to_path_buf());
            }
        }
    }

    if files_to_process.is_empty() {
        println!("Everything is up to date. No new files found.");
        return Ok(());
    }

    println!("Found {} new files to index. Processing in parallel...", files_to_process.len());

    let conn_mutex = Arc::new(Mutex::new(conn));

    files_to_process.par_iter().for_each(|path| {
        let mut title = None;
        let mut artist = None;
        let mut album = None;
        let mut genre = None;

        if let Ok(tagged_file) = Probe::open(path).and_then(|p| p.read()) {
            if let Some(primary_tag) = tagged_file.primary_tag() {
                title = primary_tag.title().map(|s| s.to_string());
                artist = primary_tag.artist().map(|s| s.to_string());
                album = primary_tag.album().map(|s| s.to_string());
                genre = primary_tag.genre().map(|s| s.to_string());
            }
        }

        if let Some((samples, sample_rate)) = decode_to_mono(path) {
            let duration = samples.len() as f32 / sample_rate as f32;
            let rms_energy = rms(&samples);
            let zcr = zero_crossing_rate(&samples);
            let (centroid, variance) = compute_spectral_features(&samples, sample_rate);
            let filepath_str = path.to_string_lossy().to_string();

            let lock = conn_mutex.lock().unwrap();
            let _ = lock.execute(
                "INSERT OR REPLACE INTO tracks 
                (filepath, title, artist, album, genre, duration, sample_rate, rms_energy, zero_crossing_rate, spectral_centroid, spectral_variance) 
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                (
                    &filepath_str,
                    &title,
                    &artist,
                    &album,
                    &genre,
                    duration as f64,
                    sample_rate as i64,
                    rms_energy as f64,
                    zcr as f64,
                    centroid as f64,
                    variance as f64,
                ),
            );
            println!("Indexed: {} - {}", artist.unwrap_or_else(|| "Unknown".to_string()), title.unwrap_or_else(|| "Unknown".to_string()));
        }
    });

    println!("Scanning complete.");
    Ok(())
}
