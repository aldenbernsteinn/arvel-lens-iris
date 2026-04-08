<script lang="ts">
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
  }

  let { transcript, showTimestamps }: Props = $props();

  function formatTime(secs: number): string {
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s.toString().padStart(2, "0")}`;
  }
</script>

<div class="transcript-view">
  {#if showTimestamps && transcript.segments.length > 0}
    <div class="segments">
      {#each transcript.segments as seg}
        <div class="segment">
          <span class="ts">{formatTime(seg.start)}</span>
          <span class="text">{seg.text}</span>
        </div>
      {/each}
    </div>
  {:else}
    <div class="plain-text">{transcript.text}</div>
  {/if}
</div>

<style>
  .transcript-view {
    background: #222;
    border-radius: 12px;
    padding: 20px;
    max-height: 400px;
    overflow-y: auto;
    user-select: text;
    line-height: 1.6;
  }

  .plain-text {
    font-size: 14px;
    color: #ddd;
    white-space: pre-wrap;
  }

  .segments {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .segment {
    display: flex;
    gap: 12px;
    font-size: 14px;
    padding: 2px 0;
  }

  .ts {
    color: #44b89a;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    min-width: 48px;
    flex-shrink: 0;
    padding-top: 2px;
  }

  .text {
    color: #ddd;
  }
</style>
