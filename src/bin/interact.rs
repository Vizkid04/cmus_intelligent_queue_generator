use clap::Parser;
use rusqlite::{Connection, Result};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    filepath: String,
    #[arg(short, long)]
    action: String, 
}

fn main() -> Result<()> {
    let args = Args::parse();
    let conn = Connection::open("/home/vizkid/Documents/Projects/music-brain/music_brain.db")?;

    match args.action.as_str() {
        "like" => {
            let _ = conn.execute("UPDATE tracks SET user_rating = 1 WHERE filepath = ?1", [&args.filepath]);
            println!("Favorited!");
        }
        "dislike" => {
            let _ = conn.execute("UPDATE tracks SET user_rating = -1 WHERE filepath = ?1", [&args.filepath]);
            println!("Disliked.");
        }
        _ => {}
    }
    Ok(())
}
