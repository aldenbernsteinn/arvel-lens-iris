<script lang="ts">
  import { invoke, Channel } from "@tauri-apps/api/core";

  interface DownloadProgress {
    file: string;
    file_index: number;
    total_files: number;
    bytes_downloaded: number;
    total_bytes: number | null;
    percent: number;
  }

  interface Props {
    onModelReady: () => void;
  }

  let { onModelReady }: Props = $props();

  let downloading = $state(false);
  let progress: DownloadProgress | null = $state(null);
  let error: string | null = $state(null);

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1048576) return `${(bytes / 1024).toFixed(0)} KB`;
    if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`;
    return `${(bytes / 1073741824).toFixed(2)} GB`;
  }

  let statusText = $derived.by(() => {
    if (!progress) return "";
    const fileLabel = `File ${progress.file_index + 1}/${progress.total_files}`;
    const downloaded = formatBytes(progress.bytes_downloaded);
    const total = progress.total_bytes ? ` / ${formatBytes(progress.total_bytes)}` : "";
    return `${fileLabel}: ${progress.file} - ${downloaded}${total}`;
  });

  let overallPercent = $derived.by(() => {
    if (!progress) return 0;
    const fileWeight = 100 / progress.total_files;
    return (progress.file_index * fileWeight) + (progress.percent / progress.total_files);
  });

  async function startDownload() {
    downloading = true;
    error = null;

    try {
      const channel = new Channel<DownloadProgress>();
      channel.onmessage = (msg: DownloadProgress) => {
        progress = msg;
      };

      await invoke("download_model", { progress: channel });
      onModelReady();
    } catch (e: any) {
      error = typeof e === "string" ? e : e.message || "Download failed";
      downloading = false;
    }
  }
</script>

<div class="setup">
  <div class="card">
    <h2>First-Time Setup</h2>
    <p class="desc">
      MacTranscribe needs to download the Parakeet TDT speech recognition model (~1.2 GB).
      This only happens once.
    </p>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    {#if downloading}
      <div class="progress-area">
        <div class="bar-track">
          <div class="bar-fill" style="width: {overallPercent}%"></div>
        </div>
        <p class="status">{statusText}</p>
      </div>
    {:else}
      <button class="download-btn" onclick={startDownload}>
        Download Model
      </button>
    {/if}
  </div>
</div>

<style>
  .setup {
    display: flex;
    justify-content: center;
    padding: 40px 0;
  }

  .card {
    background: #222;
    border-radius: 16px;
    padding: 32px;
    max-width: 440px;
    width: 100%;
    text-align: center;
  }

  h2 {
    font-size: 20px;
    font-weight: 600;
    margin: 0 0 8px;
  }

  .desc {
    color: #999;
    font-size: 13px;
    line-height: 1.5;
    margin: 0 0 24px;
  }

  .error {
    color: #f5a5a5;
    background: #3d1f1f;
    border: 1px solid #6b2c2c;
    padding: 8px 12px;
    border-radius: 8px;
    font-size: 13px;
    margin-bottom: 16px;
  }

  .download-btn {
    background: #1e3530;
    border: 1px solid #2a5548;
    color: #6edcb8;
    padding: 10px 24px;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    transition: all 0.15s ease;
  }

  .download-btn:hover {
    background: #264540;
  }

  .progress-area {
    padding-top: 8px;
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

  .status {
    color: #888;
    font-size: 11px;
    margin: 8px 0 0;
  }
</style>
