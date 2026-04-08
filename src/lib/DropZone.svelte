<script lang="ts">
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";

  interface Props {
    onFileDrop: (path: string) => void;
  }

  let { onFileDrop }: Props = $props();
  let isDragging = $state(false);
  let unlisten: (() => void) | null = null;

  const VIDEO_EXTS = [
    "mp4", "mov", "mkv", "avi", "webm", "m4v", "flv", "wmv",
    "mpg", "mpeg", "ts", "3gp", "ogv",
    "mp3", "wav", "flac", "aac", "ogg", "m4a", "wma",
  ];

  onMount(async () => {
    const appWindow = getCurrentWebviewWindow();
    unlisten = await appWindow.onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        isDragging = true;
      } else if (event.payload.type === "drop") {
        isDragging = false;
        const paths = event.payload.paths;
        if (paths.length > 0) {
          onFileDrop(paths[0]);
        }
      } else {
        isDragging = false;
      }
    });

    return () => {
      if (unlisten) unlisten();
    };
  });

  async function handleClick() {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "Media Files",
          extensions: VIDEO_EXTS,
        },
      ],
    });
    if (selected) {
      onFileDrop(selected);
    }
  }
</script>

<button
  class="dropzone"
  class:dragging={isDragging}
  onclick={handleClick}
>
  <div class="icon">
    <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
      <polyline points="7 10 12 15 17 10" />
      <line x1="12" y1="15" x2="12" y2="3" />
    </svg>
  </div>
  <p class="label">Drop a video or audio file here</p>
  <p class="hint">or click to browse</p>
  <p class="formats">MP4, MOV, MKV, AVI, WebM, MP3, WAV, and more</p>
</button>

<style>
  .dropzone {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    min-height: 260px;
    border: 2px dashed #444;
    border-radius: 16px;
    background: #222;
    cursor: pointer;
    transition: all 0.2s ease;
    padding: 40px;
    font-family: inherit;
    color: inherit;
  }

  .dropzone:hover {
    border-color: #44b89a;
    background: #1a2e28;
  }

  .dropzone.dragging {
    border-color: #44b89a;
    background: #1a2e28;
    border-style: solid;
    transform: scale(1.01);
  }

  .icon {
    color: #666;
    margin-bottom: 8px;
  }

  .dragging .icon {
    color: #44b89a;
  }

  .label {
    font-size: 16px;
    font-weight: 500;
    margin: 0;
    color: #ccc;
  }

  .hint {
    font-size: 13px;
    color: #666;
    margin: 0;
  }

  .formats {
    font-size: 11px;
    color: #555;
    margin: 8px 0 0;
  }
</style>
