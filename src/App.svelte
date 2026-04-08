<script lang="ts">
  import { invoke, Channel } from "@tauri-apps/api/core";
  import DropZone from "./lib/DropZone.svelte";
  import ProgressBar from "./lib/ProgressBar.svelte";
  import TranscriptView from "./lib/TranscriptView.svelte";
  import Toolbar from "./lib/Toolbar.svelte";
  import ModelSetup from "./lib/ModelSetup.svelte";

  type Stage = "checking" | "model_setup" | "idle" | "processing" | "result";

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

  interface TranscriptionProgress {
    stage: string;
    chunk_index: number;
    total_chunks: number;
    percent: number;
    partial_text: string;
  }

  let stage: Stage = $state("checking");
  let progress: TranscriptionProgress | null = $state(null);
  let transcript: TranscriptResult | null = $state(null);
  let showTimestamps = $state(true);
  let timestampMode = $state("words");
  let error: string | null = $state(null);
  let hasFfmpeg = $state(true);

  async function init() {
    try {
      const ready = await invoke<boolean>("check_model_status");
      const ffmpeg = await invoke<boolean>("check_ffmpeg");
      hasFfmpeg = ffmpeg;
      stage = ready ? "idle" : "model_setup";
    } catch (e) {
      stage = "model_setup";
    }
  }

  init();

  function onModelReady() {
    stage = "idle";
  }

  async function onFileDrop(path: string) {
    if (!hasFfmpeg) {
      error = "ffmpeg not found. Install with: brew install ffmpeg";
      return;
    }

    error = null;
    stage = "processing";
    progress = null;

    try {
      const progressChannel = new Channel<TranscriptionProgress>();
      progressChannel.onmessage = (msg: TranscriptionProgress) => {
        progress = msg;
      };

      const result = await invoke<TranscriptResult>("transcribe_file", {
        path,
        timestampMode: timestampMode,
        progress: progressChannel,
      });

      transcript = result;
      stage = "result";
    } catch (e: any) {
      error = typeof e === "string" ? e : e.message || "Transcription failed";
      stage = "idle";
    }
  }

  function onNewTranscription() {
    transcript = null;
    error = null;
    stage = "idle";
  }
</script>

<main>
  <header>
    <h1>MacTranscribe</h1>
    <p class="subtitle">Drag in a video, get a transcript. Powered by Parakeet.</p>
  </header>

  {#if error}
    <div class="error-banner">{error}</div>
  {/if}

  {#if stage === "checking"}
    <div class="center"><p class="loading">Loading...</p></div>
  {:else if stage === "model_setup"}
    <ModelSetup {onModelReady} />
  {:else if stage === "idle"}
    <DropZone {onFileDrop} />
    <div class="settings">
      <label>
        <span>Timestamp mode:</span>
        <select bind:value={timestampMode}>
          <option value="words">Words</option>
          <option value="sentences">Sentences</option>
        </select>
      </label>
      {#if !hasFfmpeg}
        <p class="warn">ffmpeg not found. Install: <code>brew install ffmpeg</code></p>
      {/if}
    </div>
  {:else if stage === "processing"}
    <ProgressBar {progress} />
  {:else if stage === "result" && transcript}
    <Toolbar
      {transcript}
      bind:showTimestamps
      {onNewTranscription}
    />
    <TranscriptView {transcript} {showTimestamps} />
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background: #1a1a1a;
    color: #e8e8e8;
    -webkit-font-smoothing: antialiased;
  }

  :global(*) {
    box-sizing: border-box;
  }

  main {
    max-width: 720px;
    margin: 0 auto;
    padding: 32px 24px;
    min-height: 100vh;
  }

  header {
    text-align: center;
    margin-bottom: 32px;
  }

  h1 {
    font-size: 28px;
    font-weight: 700;
    margin: 0 0 4px;
    letter-spacing: -0.5px;
  }

  .subtitle {
    color: #888;
    font-size: 14px;
    margin: 0;
  }

  .error-banner {
    background: #2d1f1f;
    border: 1px solid #5b2c2c;
    color: #f5a5a5;
    padding: 10px 16px;
    border-radius: 8px;
    margin-bottom: 16px;
    font-size: 13px;
  }

  .center {
    display: flex;
    justify-content: center;
    padding: 80px 0;
  }

  .loading {
    color: #666;
    font-size: 14px;
  }

  .settings {
    display: flex;
    justify-content: center;
    gap: 16px;
    margin-top: 16px;
  }

  .settings label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: #999;
  }

  .settings select {
    background: #2a2a2a;
    color: #e8e8e8;
    border: 1px solid #444;
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 13px;
  }

  .warn {
    color: #e8a838;
    font-size: 12px;
    margin: 0;
  }

  .warn code {
    background: #2a2a2a;
    padding: 2px 6px;
    border-radius: 4px;
  }
</style>
