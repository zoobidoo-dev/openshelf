<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import type { Book } from "$lib/reader/types";

  interface Diagnostic {
    book_id: string;
    title: string;
    file_size_bytes: number | null;
    file_size_human: string;
    range_header_received: string | null;
    range_forwarded_by_proxy: boolean;
    via_cloudflare: boolean;
    cf_ray: string | null;
    verdict: string;
  }

  let books = $state<Book[]>([]);
  let results = $state<Record<string, Diagnostic>>({});
  let loading = $state(false);
  let tested = $state<Record<string, boolean>>({});

  onMount(async () => {
    const res = await api("/api/books");
    if (res.ok) {
      books = await res.json();
    }
  });

  async function testBook(book: Book) {
    tested[book.id] = true;
    // Send a Range header so we can see if it reaches the server
    const res = await fetch(`/api/debug/book/${book.id}`, {
      credentials: "include",
      headers: { Range: "bytes=0-1023" },
    });
    if (res.ok) {
      results[book.id] = await res.json();
    }
  }

  async function testAll() {
    loading = true;
    for (const book of books) {
      await testBook(book);
    }
    loading = false;
  }

  function verdictColor(verdict: string): string {
    if (verdict.startsWith("PROBLEM")) return "#ef4444";
    if (verdict.startsWith("OK")) return "#22c55e";
    return "#f59e0b";
  }

  function mb(bytes: number | null): string {
    if (!bytes) return "unknown";
    return (bytes / (1024 * 1024)).toFixed(1) + " MB";
  }
</script>

<div style="font-family: monospace; padding: 2rem; max-width: 900px; margin: 0 auto;">
  <h1 style="font-size: 1.4rem; margin-bottom: 0.5rem;">OpenShelf — Book Diagnostics</h1>
  <p style="color: #6b7280; margin-bottom: 1.5rem; font-size: 0.85rem;">
    Tests whether Range requests survive the proxy (Cloudflare / Zscaler) and reports file sizes.
    Run this from your <strong>office browser</strong> to diagnose loading failures.
  </p>

  <button
    onclick={testAll}
    disabled={loading || books.length === 0}
    style="background:#2563eb;color:#fff;border:none;padding:0.5rem 1.2rem;border-radius:4px;cursor:pointer;margin-bottom:1.5rem;font-family:monospace;"
  >
    {loading ? "Testing..." : `Test All ${books.length} Books`}
  </button>

  {#if books.length === 0}
    <p style="color:#6b7280;">Loading book list…</p>
  {/if}

  <table style="width:100%;border-collapse:collapse;font-size:0.82rem;">
    <thead>
      <tr style="border-bottom:2px solid #e5e7eb;text-align:left;">
        <th style="padding:0.5rem;">Title</th>
        <th style="padding:0.5rem;">Size (DB)</th>
        <th style="padding:0.5rem;">Cloudflare</th>
        <th style="padding:0.5rem;">Range Forwarded</th>
        <th style="padding:0.5rem;">Verdict</th>
        <th style="padding:0.5rem;"></th>
      </tr>
    </thead>
    <tbody>
      {#each books as book}
        {@const diag = results[book.id]}
        <tr style="border-bottom:1px solid #f3f4f6;">
          <td style="padding:0.5rem;max-width:240px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;" title={book.title}>
            {book.title}
          </td>
          <td style="padding:0.5rem;color:#6b7280;">
            {diag ? diag.file_size_human : mb(book.file_size)}
          </td>
          <td style="padding:0.5rem;">
            {#if diag}
              <span style="color:{diag.via_cloudflare ? '#22c55e' : '#ef4444'}">
                {diag.via_cloudflare ? "✓ yes" : "✗ no"}
              </span>
            {:else}
              —
            {/if}
          </td>
          <td style="padding:0.5rem;">
            {#if diag}
              <span style="color:{diag.range_forwarded_by_proxy ? '#22c55e' : '#ef4444'}">
                {diag.range_forwarded_by_proxy ? "✓ yes" : "✗ no"}
              </span>
            {:else}
              —
            {/if}
          </td>
          <td style="padding:0.5rem;color:{diag ? verdictColor(diag.verdict) : '#9ca3af'};max-width:320px;">
            {diag ? diag.verdict : (tested[book.id] ? "error fetching" : "not tested")}
          </td>
          <td style="padding:0.5rem;">
            <button
              onclick={() => testBook(book)}
              style="background:#f3f4f6;border:1px solid #d1d5db;padding:0.2rem 0.6rem;border-radius:3px;cursor:pointer;font-family:monospace;font-size:0.75rem;"
            >
              Test
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  {#if Object.keys(results).length > 0}
    <div style="margin-top:2rem;padding:1rem;background:#f9fafb;border-radius:6px;font-size:0.8rem;">
      <strong>What to look for:</strong><br/>
      • <span style="color:#ef4444">PROBLEM + Range NOT forwarded</span> → Cloudflare is stripping the Range header; chunked download won't work<br/>
      • <span style="color:#22c55e">OK + Range forwarded</span> → Range requests work; if the book still fails, the issue is something else<br/>
      • File size &gt; 15 MB + no Range forwarding = the most likely cause of Zscaler blocks
    </div>
  {/if}
</div>
