# Video Frame Selection for API LLMs

## The Approach: Two-Pass Edge Density Gate

Select the minimum frames from a video worth sending to an API LLM (Claude, GPT-4o, Gemini), maximizing visual information while minimizing token cost.

**Zero ML models. Zero GPU. ~600ms. ~93MB RAM.**

### How It Works

```
Video
  │
  ├─ (Long videos) Chunk into 15-min segments
  │
  ├─ ffmpeg scene detection (threshold 0.3)
  │   → ~57 scene-change frames
  │
  ├─ PASS 1: Edge density gate (Canny, threshold 5%)
  │   → Frames with ≥5% edge pixels = MUST KEEP
  │   → These are diagrams, slides, code, tables, UI — structured content
  │
  ├─ PASS 1b: pHash dedup within must-keep (Hamming ≤4)
  │   → For screencasts: drops near-identical code frames
  │   → Keeps only frames where the screen ACTUALLY changed
  │
  ├─ PASS 2: Adaptive k-means on remaining low-edge frames
  │   → k = max(3, sqrt(n)) — scales with video length
  │   → Talking heads, smooth backgrounds, transitions
  │   → Pick one representative per cluster
  │
  ├─ (Long videos) Cross-chunk pHash dedup
  │   → Catches repeated diagrams/slides across chunks
  │   → Speaker revisits same slide? Kept once, not 3x
  │
  └─ OUTPUT: deduplicated frames + transcript → API LLM
```

### Why Two-Pass?

Plain k-means clustering on thumbnails groups frames by **color distribution**, not content. A whiteboard diagram (light background + dark text) has the same color distribution as a talking head (light wall + dark face). k-means throws away the diagram.

Edge density separates them perfectly:
- **Diagrams/slides/code**: 5-10% edge density (lines, text strokes, borders)
- **Talking heads**: 1-4% edge density (smooth face, uniform background)
- **Transitions/blur**: 0-2% edge density

### Benchmark Results (12.5min test video, 640x360)

```
No chunking:
  Input:  57 scene-change frames
  Output: 25 frames (19 must-keep + 6 cluster reps)
  Time:   608ms | RAM: 93MB | Models: 0 bytes
  Diagrams lost: 0

With 5-min chunking (simulating long video):
  Input:  57 scene-change frames across 3 chunks
  Output: 29 frames (19 must-keep + 11 cluster reps - 1 cross-chunk dupe)
  Time:   283ms | RAM: 92MB
  Cross-chunk dedup caught 1 repeated diagram
```

### Comparison With Other Approaches We Tested

| Approach | Frames | Time | RAM | Models | Lost diagrams? |
|---|---|---|---|---|---|
| TextTiling + scene detect | 90 | 3.9ms | 10.8MB | 0 | No, but way too many frames |
| MiniLM semantic scoring | 52 | 1.5s | 554MB | 22MB | No, but heavy RAM |
| Face detection + OCR | 305 | 13.9s | 74MB | ~1MB | No, but no filtering at all |
| YOLO + YAMNet | 282 | 44.2s | 1,270MB | ~20MB | No, but massive RAM |
| k-means only (no gate) | 10 | 54ms | 57MB | 0 | **YES — lost 3 diagrams** |
| **Two-pass v2 (this one)** | **25** | **608ms** | **93MB** | **0** | **No — all preserved** |

### What's Included vs Not

| Layer | Status | Notes |
|---|---|---|
| Visual frame selection | Done | Two-pass edge + pHash + k-means |
| Screencast support | Done | pHash dedup within must-keep frames |
| Long video support | Done | Chunking + cross-chunk pHash dedup |
| Transcript (speech) | Done | Parakeet ASR in the main app |
| On-screen text (OCR) | Not yet | Tesseract failed at 640x360; needs higher-res frames |
| Audio events (music/SFX) | Dropped | Background music + speech overlap too common; unreliable |

### Dependencies

- `ffmpeg` (scene detection + frame extraction)
- `opencv-python-headless` (Canny edge detection, k-means clustering)
- `numpy` (array ops)
- `imagehash` + `Pillow` (perceptual hashing for dedup)

No ML models. No GPU. No TensorFlow/PyTorch.

---

## Edge Cases & Further Benchmarking

### Video types to test

| Type | Potential issue | What to watch for |
|---|---|---|
| **Screencast / coding tutorial** | Entire video is high-edge | May keep ALL frames since code always has high edge density. Need a secondary dedup layer (pHash between must-keep frames) |
| **Lecture with slides** | Slides change but have similar layouts | Same template → similar thumbnails → k-means might merge different slides. Edge gate protects them individually |
| **Music video** | Rapid cuts, no text/diagrams | Lots of scene changes, most are "interesting" visually. Edge gate may not help (all artistic shots have moderate edges). May need higher k or different threshold |
| **Podcast (2+ people)** | Multiple camera angles of faces | All low-edge, k-means handles this well. But different people should each get one rep |
| **Sports broadcast** | Fast motion, overlays, scoreboards | Scoreboards have high edges (kept). Action shots are moderate. Motion blur frames get discarded (good) |
| **Documentary with B-roll** | Mix of interviews + nature/city footage | B-roll has varied edge density. Should work well — interviews clustered, B-roll diversity preserved |
| **Animation / cartoon** | Flat colors, clean lines | Edge density may be mid-range throughout. May need threshold tuning |
| **Silent film / no speech** | No transcript to pair with frames | Visual-only pipeline still works. Transcript layer just contributes nothing |
| **Very long video (2+ hrs)** | Scene detection produces 500+ frames | k-means k should scale with input. Maybe k = sqrt(n) or adaptive |

### Threshold sensitivity

- **Edge threshold too low (3%)**: Keeps too many talking head variants that have slight edges from hair/glasses
- **Edge threshold too high (8%)**: Only keeps the most structured content, may miss simpler slides
- **5% sweet spot** validated on educational/tutorial content — needs testing on other genres

### Possible improvements

1. **Higher-res frame extraction**: Extract at native resolution instead of 640x360 for better OCR
2. **Temporal spacing**: If two must-keep frames are <2 seconds apart, keep only the sharper one (higher Laplacian variance)
3. **Adaptive edge threshold**: Auto-detect threshold based on edge density distribution (e.g. use the natural gap between high and low clusters)

## Running the Benchmark

```bash
# Extract scene-change frames
ffmpeg -i VIDEO.mp4 -vf "select='gt(scene,0.3)',showinfo" -vsync vfr scenes/scene_%04d.jpg

# Save scene timestamps
ffmpeg -i VIDEO.mp4 -vf "select='gt(scene,0.3)',showinfo" -f null - 2>&1 | \
  grep "pts_time" | sed 's/.*pts_time:\([0-9.]*\).*/\1/' > scene_times.txt

# Run two-pass selection (no chunking)
python3 approach_twopass.py

# Run with chunking for long videos (15-min chunks)
python3 approach_twopass.py --chunk-minutes 15
```

## Files

- `approach_twopass.py` — The two-pass pipeline implementation
- `results_twopass.json` — Benchmark results from test video
- `frames_twopass/` — The 24 selected frames
- `scenes/` — Raw scene-change frames from ffmpeg
- `scene_times.txt` — Scene change timestamps
- `transcript.json` — Whisper transcript of test video
