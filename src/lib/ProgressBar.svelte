<script lang="ts">
  interface TranscriptionProgress {
    stage: string;
    chunk_index: number;
    total_chunks: number;
    percent: number;
    partial_text: string;
  }

  interface Props {
    progress: TranscriptionProgress | null;
  }

  let { progress }: Props = $props();

  let stageLabel = $derived.by(() => {
    if (!progress) return "Preparing...";
    switch (progress.stage) {
      case "extracting_audio":
        return "Extracting audio...";
      case "transcribing":
        if (progress.total_chunks > 1) {
          return `Transcribing chunk ${progress.chunk_index + 1} of ${progress.total_chunks}...`;
        }
        return "Transcribing...";
      case "complete":
        return "Done!";
      default:
        return "Processing...";
    }
  });

  let percent = $derived(progress?.percent ?? 0);
</script>

<div class="progress-container">
  <div class="spinner-row">
    {#if progress?.stage !== "complete"}
      <div class="spinner"></div>
    {/if}
    <span class="stage">{stageLabel}</span>
  </div>

  <div class="bar-track">
    <div class="bar-fill" style="width: {percent}%"></div>
  </div>

  {#if progress?.partial_text}
    <div class="partial">
      <p class="partial-label">Transcript so far:</p>
      <p class="partial-text">{progress.partial_text.slice(-300)}</p>
    </div>
  {/if}
</div>

<style>
  .progress-container {
    padding: 40px 0;
  }

  .spinner-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
  }

  .spinner {
    width: 18px;
    height: 18px;
    border: 2px solid #444;
    border-top-color: #44b89a;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .stage {
    font-size: 14px;
    color: #ccc;
  }

  .bar-track {
    width: 100%;
    height: 6px;
    background: #333;
    border-radius: 3px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    background: #44b89a;
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .partial {
    margin-top: 24px;
    padding: 16px;
    background: #222;
    border-radius: 8px;
    max-height: 200px;
    overflow-y: auto;
  }

  .partial-label {
    font-size: 11px;
    color: #666;
    margin: 0 0 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .partial-text {
    font-size: 13px;
    color: #aaa;
    margin: 0;
    line-height: 1.5;
  }
</style>
