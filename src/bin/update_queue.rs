use rusqlite::{Connection, Result};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(dead_code)]
struct TrackFeatures {
    filepath: String,
    genre: Option<String>,
    rms_energy: f64,
    zero_crossing_rate: f64,
    spectral_centroid: f64,
    spectral_variance: f64,
    play_count: i64,
    user_rating: i64,
}

fn lightweight_random(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*seed as f64) / (u64::MAX as f64)
}

fn get_cmus_queue_count() -> usize {
    let output = Command::new("cmus-remote")
        .arg("-C")
        .arg("echo <q>")
        .output();

    if let Ok(out) = output {
        let stdout_str = String::from_utf8_lossy(&out.stdout);
        let count = stdout_str.lines().filter(|line| !line.trim().is_empty()).count();
        return count;
    }
    0
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        std::process::exit(1);
    }
    let query_abs_path = &args[1];

    let conn = Connection::open("/home/vizkid/Documents/music-brain/music_brain.db")?;

    let _ = conn.execute(
        "UPDATE tracks SET play_count = play_count + 1 WHERE filepath = ?1",
        [query_abs_path],
    );

    if get_cmus_queue_count() > 0 {
        return Ok(());
    }

    let mut rng_seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;

    let mut stmt = conn.prepare(
        "SELECT filepath, genre, rms_energy, zero_crossing_rate, spectral_centroid, spectral_variance, play_count, user_rating 
         FROM tracks WHERE filepath = ?1"
    )?;
    
    let target = match stmt.query_row([query_abs_path], |row| {
        Ok(TrackFeatures {
            filepath: row.get(0)?,
            genre: row.get(1)?,
            rms_energy: row.get(2)?,
            zero_crossing_rate: row.get(3)?,
            spectral_centroid: row.get(4)?,
            spectral_variance: row.get(5)?,
            play_count: row.get(6)?,
            user_rating: row.get(7)?,
        })
    }) {
        Ok(t) => t,
        Err(_) => return Ok(()), 
    };

    let mut stmt = conn.prepare(
        "SELECT filepath, genre, rms_energy, zero_crossing_rate, spectral_centroid, spectral_variance, play_count, user_rating 
         FROM tracks WHERE filepath != ?1"
    )?;

    let track_iter = stmt.query_map([query_abs_path], |row| {
        Ok(TrackFeatures {
            filepath: row.get(0)?,
            genre: row.get(1)?,
            rms_energy: row.get(2)?,
            zero_crossing_rate: row.get(3)?,
            spectral_centroid: row.get(4)?,
            spectral_variance: row.get(5)?,
            play_count: row.get(6)?,
            user_rating: row.get(7)?,
        })
    })?;

    let mut ranked_tracks = Vec::new();
    for track_res in track_iter {
        let track = track_res?;
        let norm_target_centroid = target.spectral_centroid / 5000.0;
        let norm_track_centroid = track.spectral_centroid / 5000.0;
        let norm_target_var = target.spectral_variance / 2000.0;
        let norm_track_var = track.spectral_variance / 2000.0;

        let rms_diff = target.rms_energy - track.rms_energy;
        let zcr_diff = target.zero_crossing_rate - track.zero_crossing_rate;
        let centroid_diff = norm_target_centroid - norm_track_centroid;
        let var_diff = norm_target_var - norm_track_var;

        let mut distance = (rms_diff.powi(2) + zcr_diff.powi(2) + centroid_diff.powi(2) + var_diff.powi(2)).sqrt();

        if let (Some(g_target), Some(g_track)) = (&target.genre, &track.genre) {
            if g_target.to_lowercase().trim() != g_track.to_lowercase().trim() {
                distance += 1.5;
            }
        } else if target.genre.is_some() != track.genre.is_some() {
            distance += 0.5;
        }

        if track.play_count > 0 {
            distance -= (track.play_count as f64 * 0.02).min(0.20);
        }
        if track.user_rating == 1 {
            distance -= 0.30;
        } else if track.user_rating == -1 {
            distance += 5.00;
        }

        ranked_tracks.push((distance, track.filepath));
    }

    ranked_tracks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let mut candidate_pool: Vec<(f64, String)> = ranked_tracks.into_iter().take(30).collect();

    for item in &mut candidate_pool {
        let random_factor = lightweight_random(&mut rng_seed) * 0.15;
        item.0 += random_factor;
    }

    candidate_pool.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let _ = Command::new("cmus-remote").arg("-q").arg("-c").status();

    for (_, path) in candidate_pool.iter().take(5) {
        let _ = Command::new("cmus-remote").arg("-q").arg(path).status();
    }

    Ok(())
}
