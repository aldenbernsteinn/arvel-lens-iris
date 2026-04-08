use crate::transcriber::TimedSegment;

pub fn to_srt(segments: &[TimedSegment]) -> String {
    let mut out = String::new();
    for (i, seg) in segments.iter().enumerate() {
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            format_ts(seg.start),
            format_ts(seg.end),
            seg.text.trim(),
        ));
    }
    out
}

fn format_ts(secs: f32) -> String {
    let total_ms = (secs * 1000.0) as u64;
    let ms = total_ms % 1000;
    let s = (total_ms / 1000) % 60;
    let m = (total_ms / 60_000) % 60;
    let h = total_ms / 3_600_000;
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}
