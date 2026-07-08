# 🧠 music-brain

An intelligent, self-learning local music recommendation engine built from scratch in Rust. It utilizes **Digital Signal Processing (DSP)** and **Fast Fourier Transforms (FFT)** to extract mathematical acoustic fingerprints from your audio files, matching tracks via a customized 4D feature vector space and integrating natively with the **cmus** console player.

As you listen, `music-brain` dynamically updates your player queue with acoustically complementary tracks while factoring in implicit play history and explicit user favoriting.

---

## 🚀 Features

* **Acoustic Feature Extraction:** Uses `symphonia` and `rustfft` to analyze:
    * **RMS Energy:** Perceived track volume and density.
    * **Zero-Crossing Rate (ZCR):** Temporal noisiness, sharpness, and percussive traits.
    * **Spectral Centroid:** Atmospheric brightness and timbre center of mass.
    * **Spectral Variance:** Structural sonic texture and frequency spread.
* **Semantic Hybrid Clustering:** Merges underlying DSP signals with metadata tags (Genre validation) to avoid sudden musical jarring.
* **Continuous Infinite Queueing:** Automatically hooks into `cmus` to load consecutive blocks of 5 recommended songs only when the queue runs dry.
* **Stochastic Flavoring:** Shuffles the top 30 matching acoustic neighbors with micro-variance variables so the recommendation stream never gets stagnant.
* **Real-time Learning Loop:** Tracks your global play counts and honors explicit context-keybindings (`Shift+L` to favorite, `Shift+D` to banish).

---

## 🗺️ System Architecture

```text
                 ┌────────────────────────────────┐
                 │          Your Music            │
                 │   (/home/$USER/Music/*.mp3)   │
                 └───────────────┬────────────────┘
                                 │
                        [cargo run main.rs]
                                 ▼
                 ┌────────────────────────────────┐
                 │      music_brain.db (SQLite)   │
                 │  (Vectors, Genres, PlayCounts) │
                 └───────────────┬────────────────┘
                                 │
                         [Tracks Loaded]
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                           CMUS PLAYER                           │
│                                                                 │
│  [Plays Song]  ───►  Calls observe.rs  ───► Updates Artwork     │
│                            │                                    │
│                            ▼                                    │
│                      update_queue.rs                            │
│                            │                                    │
│     Is Queue Empty? ───────┴───────► No  ──► Let it play out.   │
│            │                                                    │
│            ▼ Yes                                                │
│   Calculates Distance                                           │
│   Applies History Weight                                        │
│   Injects Noise Variance                                        │
│   Feeds 5 Tracks to Queue ──► [Populates View 4]                │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🛠️ Prerequisites

Ensure you have the core system dependencies installed on your Linux environment:

```bash
# Ubuntu/Debian core dependencies
sudo apt update
sudo apt install build-essential alsa-utils libasound2-dev cmus
```

---

## 📦 Installation & Setup

### 1. Clone & Build the Core Engine
Clone the repository and compile the optimized release binaries:

```bash
cd ~/Documents/Projects
git clone [https://github.com/yourusername/music-brain.git](https://github.com/yourusername/music-brain.git)
cd music-brain

# Build the workspace binaries
cargo build --release
```

### 2. Populate the Database Engine
Run the primary scanner indexer to extract acoustic properties from your music collection (`/home/$USER/Music`). 

```bash
cargo run --release --bin music-brain
```

### 3. Connect to your CMUS Artwork Utility
Incorporate the callback chain into your existing `cmus` album art extractor program (`observe.rs`) to bridge the communication loop:

```rust
// Inside your cmus_cover_art observe.rs project main()
let _ = Command::new("/home/$USER/Documents/Projects/music-brain/target/release/update_queue")
    .arg(file_path)
    .status();
```
*Recompile your cover art watcher project directory with `cargo build --release` after pasting this structural hook.*

---

## 🕹️ Usage & Keybindings

Open `cmus` and enter command-line mode by pressing `:` to configure your media player setup:

### Active Player Triggers
Bind your global listener hook (if you haven't already done so for the artwork watcher utility):
```text
:set status_display_program=/home/$USER/.config/cmus/cmus_cover_art/target/release/observe
```

### Interactive Feedback Loops
Map behavioral training macros directly to your terminal key binds so `music-brain` dynamically learns your tastes:
```text
:bind common L shell /home/$USER/Documents/Projects/music-brain/target/release/interact --action like --filepath "{file}"
:bind common D shell /home/$USER/Documents/Projects/music-brain/target/release/interact --action dislike --filepath "{file}"
```

* **`Shift + L`**: Mark the currently playing track as a **Favorite** (Gives it a permanent distance bonus boost).
* **`Shift + D`**: **Banish** the track (Adds a catastrophic penalty value so it is filtered out of all future recommendation loops).

---

## 📊 Evaluation Matrix

You can manually inspect matching scores for any file on your drive by passing it to the evaluation tool:

```bash
cargo run --release --bin recommend -- --query-path "/home/$USER/Music/Favs/A.R. Rahman - Ella Pugazhum.mp3"
```

#### Terminal Sample Output:
```text
🔍 Querying Intelligent Framework for: A.R. Rahman - "Ella Pugazhum"
   Detected Tag Category: Tamil Pop
------------------------------------------------------------
1. [0.0295 combined score] A.R. Rahman/Srinivas - Ae Maanpuru Mangaiyae (Tamil Pop)
2. [0.0330 combined score] G. V. Prakash/Saindhavi - Yaar Indha Saalai Oram (Tamil Pop)
3. [0.0410 combined score] A.R. Rahman/Santhosh Narayanan - Water Packet (Tamil Pop)
4. [0.0436 combined score] A.R. Rahman/Shreya Ghoshal - Ratchasa Maamaney (Tamil Pop)
5. [0.0466 combined score] G. V. Prakash/Haricharan - Aariro (Tamil Pop)
```

---

## 🛠️ Mathematical Specifics

The distance calculation matches multi-dimensional vectors across the target file and candidate space:

$$\text{Distance} = \sqrt{\Delta \text{RMS}^2 + \Delta \text{ZCR}^2 + \Delta \text{Centroid}_{\text{norm}}^2 + \Delta \text{Variance}_{\text{norm}}^2} + \text{Penalty}_{\text{Genre}} - \text{Bonus}_{\text{History}}$$

Where:
* $\text{Centroid}_{\text{norm}}$ is normalized against a ceiling of **5000.0 Hz**.
* $\text{Variance}_{\text{norm}}$ is normalized against a ceiling of **2000.0 Hz**.
* Mismatched genres receive a structural penalty scalar of **+1.5**.
* Frequent items get an automated listening incentive of up to **-0.20**.
