<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { save } from "@tauri-apps/plugin-dialog";
  import { writeTextFile } from "@tauri-apps/plugin-fs";

  interface TimedSegment {
    text: string;
    start: number;
    end: number;
  }

  interface TranscriptResult {
    text: string;
    segments: TimedSegment[];
    duration_secs: number;
  }

  interface Props {
    transcript: TranscriptResult;
    showTimestamps: boolean;
    onNewTranscription: () => void;
  }

  let { transcript, showTimestamps = $bindable(), onNewTranscription }: Props = $props();

  async function copyToClipboard() {
    try {
      await navigator.clipboard.writeText(transcript.text);
    } catch {}
  }

  async function exportTxt() {
    const path = await save({
      defaultPath: "transcript.txt",
      filters: [{ name: "Text", extensions: ["txt"] }],
    });
    if (path) {
      await writeTextFile(path, transcript.text);
    }
  }

  async function exportSrt() {
    const srt = await invoke<string>("export_srt", { segments: transcript.segments });
    const path = await save({
      defaultPath: "transcript.srt",
      filters: [{ name: "SubRip", extensions: ["srt"] }],
    });
    if (path) {
      await writeTextFile(path, srt);
    }
  }

  function formatDuration(secs: number): string {
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    if (m === 0) return `${s}s`;
    return `${m}m ${s}s`;
  }
</script>

<div class="toolbar">
  <div class="left">
    <span class="duration">{formatDuration(transcript.duration_secs)}</span>
    <span class="sep">|</span>
    <span class="count">{transcript.segments.length} segments</span>
  </div>

  <div class="actions">
    <label class="toggle">
      <input type="checkbox" bind:checked={showTimestamps} />
      <span>Timestamps</span>
    </label>
    <button onclick={copyToClipboard}>Copy</button>
    <button onclick={exportTxt}>.txt</button>
    <button onclick={exportSrt}>.srt</button>
    <button class="new-btn" onclick={onNewTranscription}>New</button>
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 0;
    margin-bottom: 8px;
    border-bottom: 1px solid #333;
  }

  .left {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: #888;
  }

  .sep {
    color: #444;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: #999;
    cursor: pointer;
    margin-right: 8px;
  }

  .toggle input {
    accent-color: #44b89a;
  }

  button {
    background: #2a2a2a;
    border: 1px solid #444;
    color: #ccc;
    padding: 5px 12px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
    transition: all 0.15s ease;
  }

  button:hover {
    background: #333;
    border-color: #555;
  }

  .new-btn {
    background: #1e3530;
    border-color: #2a5548;
    color: #6edcb8;
  }

  .new-btn:hover {
    background: #264540;
  }
</style>
