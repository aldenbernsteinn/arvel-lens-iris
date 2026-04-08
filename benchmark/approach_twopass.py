"""
Two-Pass Pipeline v3: Edge density gate + pHash dedup + leader algorithm
- Pass 1: Protect high-edge frames (diagrams, slides, code)
- Pass 1b: pHash dedup within must-keep (for screencasts)
- Pass 2: Leader algorithm on low-edge frames (replaces k-means)
  → Centroids persist across chunks → same output chunked or not
- Cross-chunk pHash dedup on must-keep frames
"""
import cv2
import numpy as np
import imagehash
from PIL import Image
import json, time, os, resource, math, sys

EDGE_THRESHOLD = 0.05   # 5% edge density
PHASH_THRESHOLD = 4      # Hamming distance ≤4 = near-duplicate
LEADER_THRESHOLD = 15000  # L2 distance threshold for leader algorithm on 64x64 thumbnails

def get_mem_mb():
    return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / (1024 * 1024)

def compute_phash(img_path):
    return imagehash.phash(Image.open(img_path))


class LeaderClusterer:
    """Single-pass leader algorithm. Centroids persist across calls."""

    def __init__(self, threshold):
        self.threshold = threshold
        self.centroids = []      # list of numpy vectors
        self.representatives = [] # list of (entry, vector) for the kept frame

    def process(self, entry, vector):
        """Returns True if this frame is a new cluster representative."""
        if not self.centroids:
            self.centroids.append(vector)
            self.representatives.append(entry)
            return True

        dists = [np.linalg.norm(vector - c) for c in self.centroids]
        min_dist = min(dists)

        if min_dist >= self.threshold:
            self.centroids.append(vector)
            self.representatives.append(entry)
            return True
        return False

    def get_representatives(self):
        return list(self.representatives)


def process_frames(scenes_dir, scene_files, scene_times, leader, must_keep_hashes):
    """Process a batch of scene frames. Leader and must_keep_hashes persist across calls."""
    must_keep = []
    must_keep_phash_dropped = 0
    leader_kept = 0
    leader_skipped = 0

    for i, fname in enumerate(scene_files):
        fpath = os.path.join(scenes_dir, fname)
        img = cv2.imread(fpath)
        if img is None:
            continue
        gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
        ts = scene_times[i] if i < len(scene_times) else i * 2.0
        ts_str = f"{int(ts//60)}:{int(ts%60):02d}"

        edges = cv2.Canny(gray, 50, 150)
        edge_pct = np.count_nonzero(edges) / edges.size
        lap_var = cv2.Laplacian(gray, cv2.CV_64F).var()
        jpeg_kb = os.path.getsize(fpath) / 1024

        entry = {
            "fname": fname,
            "fpath": fpath,
            "timestamp": ts,
            "ts_str": ts_str,
            "edge_pct": round(edge_pct * 100, 1),
            "lap_var": round(lap_var, 0),
            "jpeg_kb": round(jpeg_kb, 1),
        }

        if edge_pct >= EDGE_THRESHOLD:
            # Must-keep: pHash dedup against all previously seen must-keep frames
            h = compute_phash(fpath)
            is_dup = False
            for seen_h in must_keep_hashes:
                if abs(h - seen_h) <= PHASH_THRESHOLD:
                    is_dup = True
                    break
            if not is_dup:
                entry["reason"] = "high_edge"
                must_keep.append(entry)
                must_keep_hashes.append(h)
            else:
                must_keep_phash_dropped += 1
        else:
            # Low-info: leader algorithm
            thumb = cv2.resize(img, (64, 64)).flatten().astype(np.float32)
            if leader.process(entry, thumb):
                entry["reason"] = "leader_new_cluster"
                leader_kept += 1
            else:
                leader_skipped += 1

    return must_keep, {
        "must_keep": len(must_keep),
        "phash_dropped": must_keep_phash_dropped,
        "leader_kept": leader_kept,
        "leader_skipped": leader_skipped,
    }


def main():
    import argparse
    parser = argparse.ArgumentParser(description="Two-pass video frame selector v3")
    parser.add_argument("--scenes-dir", default="scenes")
    parser.add_argument("--scene-times", default="scene_times.txt")
    parser.add_argument("--output-dir", default="frames_twopass")
    parser.add_argument("--chunk-minutes", type=int, default=0, help="Chunk size (0=no chunking)")
    parser.add_argument("--leader-threshold", type=float, default=LEADER_THRESHOLD)
    args = parser.parse_args()

    mem_start = get_mem_mb()
    t_start = time.time()

    with open(args.scene_times) as f:
        scene_times = [float(l.strip()) for l in f if l.strip()]

    scene_files = sorted(f for f in os.listdir(args.scenes_dir) if f.endswith(".jpg"))
    total_input = len(scene_files)
    print(f"Input: {total_input} scene-change frames")

    # Shared state across chunks
    leader = LeaderClusterer(args.leader_threshold)
    must_keep_hashes = []  # pHash values for cross-chunk dedup
    all_must_keep = []

    if args.chunk_minutes > 0 and scene_times:
        chunk_secs = args.chunk_minutes * 60
        max_time = max(scene_times)
        n_chunks = max(1, int(math.ceil(max_time / chunk_secs)))
        print(f"Chunking into {n_chunks} segments of {args.chunk_minutes} min each\n")

        for c in range(n_chunks):
            t_lo = c * chunk_secs
            t_hi = (c + 1) * chunk_secs
            chunk_indices = [i for i, t in enumerate(scene_times) if t_lo <= t < t_hi]
            chunk_files = [scene_files[i] for i in chunk_indices]
            chunk_times = [scene_times[i] for i in chunk_indices]

            if not chunk_files:
                continue

            print(f"--- Chunk {c+1}/{n_chunks} [{int(t_lo//60)}:{int(t_lo%60):02d} - {int(t_hi//60)}:{int(t_hi%60):02d}] ({len(chunk_files)} frames) ---")
            mk, stats = process_frames(args.scenes_dir, chunk_files, chunk_times, leader, must_keep_hashes)
            all_must_keep.extend(mk)
            print(f"  must-keep={stats['must_keep']} pHash-dropped={stats['phash_dropped']} leader-kept={stats['leader_kept']} leader-skipped={stats['leader_skipped']}")
    else:
        print()
        mk, stats = process_frames(args.scenes_dir, scene_files, scene_times, leader, must_keep_hashes)
        all_must_keep.extend(mk)

    # Combine must-keep + leader representatives
    leader_reps = leader.get_representatives()
    all_kept = all_must_keep + leader_reps
    all_kept.sort(key=lambda x: x["timestamp"])

    total_time = time.time() - t_start
    mem_end = get_mem_mb()

    # Save frames
    os.makedirs(args.output_dir, exist_ok=True)
    for f in os.listdir(args.output_dir):
        os.remove(os.path.join(args.output_dir, f))

    for i, entry in enumerate(all_kept):
        src = entry["fpath"]
        dst = os.path.join(args.output_dir, f"kept_{i:02d}_{entry['timestamp']:.0f}s.jpg")
        img = cv2.imread(src)
        if img is not None:
            cv2.imwrite(dst, img)

    # Results
    print(f"\n{'='*70}")
    print(f"TWO-PASS v3 RESULTS (Leader Algorithm)")
    print(f"{'='*70}")
    print(f"Input:              {total_input} scene-change frames")
    print(f"Output:             {len(all_kept)} frames to send to API")
    print(f"  - Must-keep:      {len(all_must_keep)} (high edge, pHash deduped)")
    print(f"  - Leader reps:    {len(leader_reps)} (low-info representatives)")
    print(f"  - Leader clusters: {len(leader.centroids)} total centroids created")
    print(f"Discarded:          {total_input - len(all_kept)} redundant frames")
    print(f"Reduction:          {total_input} -> {len(all_kept)} ({(1 - len(all_kept)/max(1,total_input))*100:.0f}% fewer)")
    print(f"")
    print(f"Total time:         {total_time*1000:.0f}ms")
    print(f"Peak RSS:           {mem_end:.1f}MB")
    print(f"Memory delta:       {mem_end - mem_start:.1f}MB")
    print(f"Model files:        0 bytes")

    print(f"\n--- Selected frames ---")
    for e in all_kept:
        print(f"  [{e['ts_str']}] edge={e['edge_pct']}% | {e.get('reason','')}")

    results = {
        "approach": "TWO_PASS_v3_leader",
        "input_frames": total_input,
        "output_frames": len(all_kept),
        "must_keep": len(all_must_keep),
        "leader_reps": len(leader_reps),
        "leader_clusters": len(leader.centroids),
        "leader_threshold": args.leader_threshold,
        "total_time_ms": round(total_time * 1000),
        "peak_rss_mb": round(mem_end, 1),
        "kept_frames": [{k: v for k, v in e.items() if k != "fpath"} for e in all_kept],
    }
    with open("results_twopass.json", "w") as f:
        json.dump(results, f, indent=2)

    print(f"\nFrames saved to {args.output_dir}/")


if __name__ == "__main__":
    os.chdir("/Users/aldenb/mactranscribe/benchmark")
    main()
