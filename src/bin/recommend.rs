use clap::Parser;
use rusqlite::{Connection, Result};
use std::path::Path;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    query_path: String,
}

#[allow(dead_code)]
struct TrackFeatures {
    filepath: String,
    title: Option<String>,
    artist: Option<String>,
    genre: Option<String>,
    rms_energy: f64,
    zero_crossing_rate: f64,
    spectral_centroid: f64,
    spectral_variance: f64,
    play_count: i64,
    user_rating: i64,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let query_abs_path = Path::new(&args.query_path).canonicalize()
        .expect("Could not resolve path")
        .to_string_lossy()
        .to_string();

    let conn = Connection::open("music_brain.db")?;

    let mut stmt = conn.prepare(
        "SELECT filepath, title, artist, genre, rms_energy, zero_crossing_rate, spectral_centroid, spectral_variance, play_count, user_rating 
         FROM tracks WHERE filepath = ?1"
    )?;
    
    let target = stmt.query_row([&query_abs_path], |row| {
        Ok(TrackFeatures {
            filepath: row.get(0)?,
            title: row.get(1)?,
            artist: row.get(2)?,
            genre: row.get(3)?,
            rms_energy: row.get(4)?,
            zero_crossing_rate: row.get(5)?,
            spectral_centroid: row.get(6)?,
            spectral_variance: row.get(7)?,
            play_count: row.get(8)?,
            user_rating: row.get(9)?,
        })
    }).expect("Track not found in DB.");

    println!("\n🔍 Querying Intelligent Framework for: {} - {:?}", target.artist.clone().unwrap_or_default(), target.title.clone().unwrap_or_default());
    println!("   Detected Tag Category: {}", target.genre.as_deref().unwrap_or("None/Untagged"));
    println!("------------------------------------------------------------");

    let mut stmt = conn.prepare(
        "SELECT filepath, title, artist, genre, rms_energy, zero_crossing_rate, spectral_centroid, spectral_variance, play_count, user_rating 
         FROM tracks WHERE filepath != ?1"
    )?;

    let track_iter = stmt.query_map([&query_abs_path], |row| {
        Ok(TrackFeatures {
            filepath: row.get(0)?,
            title: row.get(1)?,
            artist: row.get(2)?,
            genre: row.get(3)?,
            rms_energy: row.get(4)?,
            zero_crossing_rate: row.get(5)?,
            spectral_centroid: row.get(6)?,
            spectral_variance: row.get(7)?,
            play_count: row.get(8)?,
            user_rating: row.get(9)?,
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
            let play_bonus = (track.play_count as f64 * 0.02).min(0.20);
            distance -= play_bonus;
        }

        if track.user_rating == 1 {
            distance -= 0.30;
        } else if track.user_rating == -1 {
            distance += 5.00;
        }

        ranked_tracks.push((distance, track));
    }

    ranked_tracks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    for (i, (dist, track)) in ranked_tracks.iter().take(5).enumerate() {
        println!(
            "{}. [{:.4} combined score] {} - {} ({})",
            i + 1, dist,
            track.artist.as_deref().unwrap_or("Unknown"),
            track.title.as_deref().unwrap_or("Unknown"),
            track.genre.as_deref().unwrap_or("Untagged")
        );
    }

    Ok(())
}
