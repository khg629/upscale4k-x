<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount, onDestroy } from "svelte";

  type Status =
    | { kind: "pending" }
    | { kind: "processing"; progress: number }
    | { kind: "done"; outputPath: string; durationMs: number }
    | { kind: "failed"; message: string };

  type QueueItem = {
    id: string;
    inputPath: string;
    fileName: string;
    status: Status;
  };

  let queue = $state<QueueItem[]>([]);
  let model = $state<"realesrgan-x4plus" | "realesrgan-x4plus-anime">("realesrgan-x4plus-anime");
  let scale = $state<number>(4);
  let outputDir = $state<string | null>(null);

  let isRunning = $state(false);
  let cancelRequested = $state(false);
  let currentItemId = $state<string | null>(null);
  let isHovering = $state(false);
  let errorMsg = $state<string>("");

  let pendingCount = $derived(queue.filter((q) => q.status.kind === "pending").length);
  let doneCount = $derived(queue.filter((q) => q.status.kind === "done").length);

  let unlistens: UnlistenFn[] = [];

  onMount(async () => {
    unlistens.push(
      await listen<{ itemId: string; progress: number }>("upscale-progress", (e) => {
        const idx = queue.findIndex((i) => i.id === e.payload.itemId);
        if (idx >= 0) {
          queue[idx] = {
            ...queue[idx],
            status: { kind: "processing", progress: e.payload.progress },
          };
        }
      }),
    );

    const win = getCurrentWindow();
    unlistens.push(
      await win.onDragDropEvent((event) => {
        const t = event.payload.type;
        if (t === "enter" || t === "over") isHovering = true;
        else if (t === "leave") isHovering = false;
        else if (t === "drop") {
          isHovering = false;
          if (!isRunning) addPaths(event.payload.paths);
        }
      }),
    );
  });

  onDestroy(() => {
    unlistens.forEach((f) => f());
  });

  async function addPaths(paths: string[]) {
    const expanded = await invoke<string[]>("expand_paths", { paths });
    const existing = new Set(queue.map((q) => q.inputPath));
    const newItems: QueueItem[] = [];
    for (const p of expanded) {
      if (existing.has(p)) continue;
      const slash = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
      newItems.push({
        id: crypto.randomUUID(),
        inputPath: p,
        fileName: p.slice(slash + 1),
        status: { kind: "pending" },
      });
    }
    queue = [...queue, ...newItems];
    errorMsg = "";
  }

  async function pickFiles() {
    const result = await open({
      multiple: true,
      directory: false,
      filters: [
        { name: "이미지", extensions: ["png", "jpg", "jpeg", "webp", "tiff", "heic", "heif", "bmp"] },
      ],
    });
    if (Array.isArray(result)) addPaths(result);
    else if (typeof result === "string") addPaths([result]);
  }

  async function pickFolder() {
    const result = await open({ directory: true, multiple: false });
    if (typeof result === "string") addPaths([result]);
  }

  async function pickOutputDir() {
    const result = await open({ directory: true, multiple: false });
    if (typeof result === "string") outputDir = result;
  }

  function removeItem(id: string) {
    queue = queue.filter((q) => q.id !== id);
  }

  function clearQueue() {
    queue = [];
    errorMsg = "";
  }

  async function runBatch() {
    if (isRunning || pendingCount === 0) return;
    isRunning = true;
    cancelRequested = false;
    errorMsg = "";

    for (let i = 0; i < queue.length; i++) {
      if (cancelRequested) break;
      if (queue[i].status.kind !== "pending") continue;

      currentItemId = queue[i].id;
      queue[i] = { ...queue[i], status: { kind: "processing", progress: 0 } };
      const startMs = performance.now();

      try {
        const outPath = await invoke<string>("upscale_image", {
          args: {
            itemId: queue[i].id,
            inputPath: queue[i].inputPath,
            outputDir,
            model,
            scale,
          },
        });
        queue[i] = {
          ...queue[i],
          status: {
            kind: "done",
            outputPath: outPath,
            durationMs: performance.now() - startMs,
          },
        };
      } catch (e) {
        const msg = String(e);
        if (msg.includes("CANCELLED")) {
          queue[i] = { ...queue[i], status: { kind: "pending" } };
          break;
        }
        queue[i] = { ...queue[i], status: { kind: "failed", message: msg } };
      }
    }

    currentItemId = null;
    isRunning = false;

    // 완료 후 결과 폴더 자동 열기
    const firstDone = queue.find((q) => q.status.kind === "done");
    if (firstDone && firstDone.status.kind === "done") {
      try {
        await revealItemInDir(firstDone.status.outputPath);
      } catch (_) {
        /* ignore */
      }
    }
  }

  async function cancelBatch() {
    cancelRequested = true;
    try {
      await invoke("cancel_upscale");
    } catch {
      /* ignore */
    }
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Escape" && isRunning) cancelBatch();
    else if (e.key === "Enter" && !isRunning && pendingCount > 0) runBatch();
  }

  function statusLabel(s: Status): string {
    switch (s.kind) {
      case "pending":
        return "대기";
      case "processing":
        return `${Math.round(s.progress * 100)}%`;
      case "done":
        return `${(s.durationMs / 1000).toFixed(1)}초`;
      case "failed":
        return "실패";
    }
  }

  function statusIcon(s: Status): string {
    switch (s.kind) {
      case "pending":
        return "⏳";
      case "processing":
        return "🔄";
      case "done":
        return "✅";
      case "failed":
        return "❌";
    }
  }
</script>

<svelte:window onkeydown={handleKey} />

<main class="container">
  <header>
    <h1>UpScale4K</h1>
    <p class="subtitle">로컬 이미지 4× 업스케일</p>
  </header>

  <section class="queue-pane" class:hover={isHovering}>
    <div class="pane-header">
      <h2>입력 {queue.length > 0 ? `(${queue.length})` : ""}</h2>
      {#if queue.length > 0 && !isRunning}
        <button class="link" onclick={clearQueue}>모두 지우기</button>
      {/if}
    </div>

    {#if queue.length === 0}
      <div class="empty">
        <div class="empty-icon">📥</div>
        <p>이미지·폴더를 끌어다 놓거나<br />아래 버튼으로 추가하세요</p>
        <p class="hint">한 번에 100장 이상 가능</p>
      </div>
    {:else}
      <div class="queue-list">
        {#each queue as item (item.id)}
          <div class="queue-row" class:current={currentItemId === item.id}>
            <span class="row-icon">{statusIcon(item.status)}</span>
            <span class="filename" title={item.inputPath}>{item.fileName}</span>
            {#if item.status.kind === "processing"}
              <progress value={item.status.progress} max="1"></progress>
            {/if}
            <span class="status-label">{statusLabel(item.status)}</span>
            <button
              class="remove"
              onclick={() => removeItem(item.id)}
              disabled={isRunning}
              title="큐에서 제거"
              aria-label="제거">✕</button
            >
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <section class="control-row">
    <label>
      <span>모델</span>
      <select bind:value={model} disabled={isRunning}>
        <option value="realesrgan-x4plus">General (사진)</option>
        <option value="realesrgan-x4plus-anime">Anime (일러스트·텍스트)</option>
      </select>
    </label>
    <label>
      <span>배율</span>
      <select bind:value={scale} disabled={isRunning}>
        <option value={2}>2×</option>
        <option value={3}>3×</option>
        <option value={4}>4×</option>
      </select>
    </label>
  </section>

  <section class="output-row">
    <span class="folder-icon">📁</span>
    {#if outputDir}
      <span class="output-path" title={outputDir}>출력: {outputDir}</span>
      <button class="link" onclick={pickOutputDir} disabled={isRunning}>변경</button>
      <button class="link" onclick={() => (outputDir = null)} disabled={isRunning}>초기화</button>
    {:else}
      <span class="output-path">출력: 입력 파일과 같은 폴더</span>
      <button class="link" onclick={pickOutputDir} disabled={isRunning}>출력 폴더 선택…</button>
    {/if}
  </section>

  <section class="action-row">
    {#if queue.length > 0}
      <span class="counter" class:done={doneCount === queue.length && doneCount > 0}>
        {doneCount} / {queue.length} 완료
      </span>
    {/if}
    {#if isRunning}
      <progress value={doneCount} max={queue.length} class="overall"></progress>
    {/if}
    <span class="spacer"></span>
    <button onclick={pickFiles} disabled={isRunning}>파일 추가</button>
    <button onclick={pickFolder} disabled={isRunning}>폴더 추가</button>
    {#if isRunning}
      <button class="cancel" onclick={cancelBatch}>취소 (Esc)</button>
    {:else}
      <button class="primary" onclick={runBatch} disabled={pendingCount === 0}>
        {pendingCount > 0 ? `업스케일 시작 (${pendingCount}장)` : "업스케일 시작"}
      </button>
    {/if}
  </section>

  {#if errorMsg}
    <div class="error">⚠️ {errorMsg}</div>
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", sans-serif;
    background: #f5f5f7;
    color: #1c1c1e;
  }

  .container {
    max-width: 980px;
    margin: 16px auto;
    padding: 16px 20px;
    background: white;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: calc(100vh - 32px);
    box-sizing: border-box;
  }

  header {
    text-align: center;
  }
  h1 {
    margin: 0 0 2px;
    font-size: 1.4rem;
  }
  .subtitle {
    margin: 0;
    color: #6e6e73;
    font-size: 0.85rem;
  }

  /* 큐 패널 */
  .queue-pane {
    flex: 1;
    border: 2px dashed #d2d2d7;
    border-radius: 10px;
    padding: 8px 8px 8px 12px;
    display: flex;
    flex-direction: column;
    transition: border-color 0.15s, background 0.15s;
    min-height: 200px;
  }
  .queue-pane.hover {
    border-color: #0a84ff;
    background: #f0f8ff;
  }
  .pane-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 4px 4px;
  }
  .pane-header h2 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
  }

  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: #8e8e93;
    text-align: center;
  }
  .empty-icon {
    font-size: 2.5rem;
    margin-bottom: 8px;
  }
  .empty p {
    margin: 0;
    font-size: 0.9rem;
  }
  .hint {
    margin-top: 6px !important;
    font-size: 0.8rem !important;
    color: #aeaeb2;
  }

  .queue-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 4px 4px 0;
  }
  .queue-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 0.88rem;
  }
  .queue-row.current {
    background: rgba(10, 132, 255, 0.1);
  }
  .row-icon {
    flex: 0 0 18px;
  }
  .filename {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .queue-row progress {
    flex: 0 0 60px;
    height: 6px;
  }
  .status-label {
    flex: 0 0 60px;
    text-align: right;
    font-size: 0.78rem;
    color: #6e6e73;
    font-variant-numeric: tabular-nums;
  }
  .remove {
    flex: 0 0 24px;
    padding: 0;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: #aeaeb2;
    cursor: pointer;
    font-size: 0.85rem;
  }
  .remove:hover:not(:disabled) {
    background: #ff3b30;
    color: white;
  }
  .remove:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  /* 컨트롤 행들 */
  .control-row {
    display: flex;
    gap: 16px;
    align-items: center;
  }
  label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.85rem;
  }
  label span {
    color: #6e6e73;
  }
  select {
    padding: 4px 8px;
    border: 1px solid #d2d2d7;
    border-radius: 6px;
    background: white;
    font-size: 0.9rem;
  }

  .output-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 4px;
    font-size: 0.85rem;
    border-top: 1px solid #f0f0f3;
    border-bottom: 1px solid #f0f0f3;
  }
  .folder-icon {
    flex: 0 0 auto;
  }
  .output-path {
    flex: 1;
    color: #6e6e73;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }

  /* 액션 행 */
  .action-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .counter {
    font-size: 0.85rem;
    color: #6e6e73;
    font-variant-numeric: tabular-nums;
  }
  .counter.done {
    color: #34c759;
    font-weight: 600;
  }
  .overall {
    flex: 1;
    max-width: 200px;
    height: 6px;
  }
  .spacer {
    flex: 1;
  }

  button {
    padding: 6px 14px;
    border: 1px solid #d2d2d7;
    border-radius: 6px;
    background: white;
    cursor: pointer;
    font-size: 0.88rem;
    transition: all 0.1s;
  }
  button:hover:not(:disabled) {
    background: #f5f5f7;
  }
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  button.primary {
    background: #0a84ff;
    color: white;
    border-color: #0a84ff;
    font-weight: 500;
    padding: 8px 18px;
  }
  button.primary:hover:not(:disabled) {
    background: #006fdc;
  }
  button.cancel {
    background: white;
    color: #ff3b30;
    border-color: #ff3b30;
    font-weight: 500;
    padding: 8px 18px;
  }
  button.cancel:hover {
    background: #ff3b30;
    color: white;
  }
  button.link {
    border: none;
    background: transparent;
    color: #0a84ff;
    padding: 2px 6px;
    font-size: 0.82rem;
  }
  button.link:hover:not(:disabled) {
    text-decoration: underline;
    background: transparent;
  }

  .error {
    padding: 8px 12px;
    background: #fff0ef;
    color: #ff3b30;
    border-radius: 6px;
    font-size: 0.85rem;
  }

  @media (prefers-color-scheme: dark) {
    :global(body) {
      background: #1c1c1e;
      color: #f2f2f7;
    }
    .container {
      background: #2c2c2e;
      box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
    }
    .queue-pane {
      border-color: #3a3a3c;
    }
    .queue-pane.hover {
      background: rgba(10, 132, 255, 0.15);
    }
    .pane-header h2,
    .subtitle,
    label span,
    .output-path,
    .counter,
    .empty {
      color: #aeaeb2;
    }
    .empty p,
    .hint {
      color: #8e8e93;
    }
    .queue-row.current {
      background: rgba(10, 132, 255, 0.2);
    }
    .status-label {
      color: #aeaeb2;
    }
    .output-row {
      border-color: #3a3a3c;
    }
    select,
    button {
      background: #3a3a3c;
      color: #f2f2f7;
      border-color: #48484a;
    }
    button:hover:not(:disabled) {
      background: #48484a;
    }
    .error {
      background: #4a1f1c;
      color: #ff453a;
    }
  }
</style>
