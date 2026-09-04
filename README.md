# gprof

Ultra-fast SpectorJS WebGL / WebGPU frame capture analyzer and profiler written in Rust.

Parses massive SpectorJS `.json` captures (100–500+ MB) in **sub-100 milliseconds** and immediately surfaces actionable GPU performance bottlenecks, memory hogs, and redundant render states.

Accepts standard SpectorJS frame capture JSON files.

## Philosophy

Like [`chperf`](../chperf) (for Chrome traces) and [`hprof`](../hprof) (for V8 heap snapshots), `gprof` is designed for **instant actionable insight**. Instead of manually clicking through hundreds of commands in the SpectorJS UI, `gprof` computes the full frame profile and outputs clean, GitHub-flavored Markdown reports ready to be fed into LLMs or pasted directly into audit docs.

## What `gprof` Identifies Instantly

1. **GlowLayer Occluder Waste:** Detects meshes drawn into GlowLayer FBOs with black `[0, 0, 0, 1]` color without emissive textures (often >90% of glow geometry is wasted occluders).
2. **WebGL State Thrashing & Redundancy:** Audits redundant `depthMask()`, `enable/disable(BLEND)`, `enable/disable(DEPTH_TEST)`, and `useProgram()` calls that bypass engine caching.
3. **Dynamic Buffer & Texture Uploads:** Catches per-frame `bufferSubData` (e.g. duplicate matrix updates from `DThinInstanceManager`) and `texImage2D` calls, extracting origin stack traces.
4. **32-Bit Index Buffer Meshes:** Pinpoints meshes exceeding 65,535 vertices forcing `UNSIGNED_INT` (32-bit) index buffers, triggering shader recompilations (`VERTEX_PULLING_INDEX_BUFFER_32BITS`) and doubling IBO bandwidth.
5. **Instancing & Batching Candidates:** Finds draw calls sharing identical vertex buffers and shaders to suggest Babylon.js Thin Instances or Hardware Instancing.
6. **Texture Memory Hogs & Skybox Bloat:** Categorizes VRAM usage (8K, 4K, 2K, Float/VAT, FBOs), spots uncompressed PNG/JPEG textures, oversized 8K skyboxes, and 2K emissive maps.
7. **Single-Draw Textures:** Spots 2K+ textures consuming >=16 MB VRAM that are bound in only 1 draw call.
8. **GPU Particle Overdraw:** Analyzes Transform Feedback particle systems, active point counts, and quad billboard triangle volume.
9. **Shader Permutations & Cache Fragmentation:** Detects why PBR shader variants split (32-bit index buffers, ORM channel packing divergence, alpha test cutoffs, UV channels).
10. **Render Pass Pipeline:** Reconstructs the exact sequence of FBOs and passes (VAT animation, Glow, Blur, Main Color, Weapon Overlay).
11. **A/B Comparison (Diff Mode):** Compares two captures (e.g. before and after optimization) with metric deltas.
12. **Transparent Gzip Support:** Automatically reads `.json.gz` files on the fly without manual decompression.
13. **Babylon.js Performance Linter (`gprof lint`):** 10 automated lint rules (GlowLayer whitelist mode, thin instancing, KTX2 transcoding, ping-pong shader sorting, 32-bit index buffers, dynamic buffer dirty checks) with copy-paste TypeScript code fixes.
14. **Material & Shader Sorting Audit (`gprof sorting`):** Detects ping-pong shader switches (A -> B -> A), counts redundant program binds, calculates pass efficiency score, and warns when opaque meshes render after blended meshes (broken Early-Z rejection).
15. **Texture Compression & NPOT Audit (`gprof compression`):** Calculates exact VRAM savings when transcoding uncompressed RGBA8 textures to KTX2 (ASTC 4x4 / BC7, -75%) or half-float VAT (-50%), and lists non-power-of-two (NPOT) textures.
16. **Render Target (FBO) Footprint (`gprof fbos`):** Calculates offscreen render target VRAM allocations and flags post-processing passes running at 100% viewport resolution that can be downscaled to 0.5x.
17. **Interactive Standalone HTML Dashboard (`--html report.html`):** Fully offline, single-file HTML report with zero external CDN dependencies, KPI metrics cards, interactive tabs, searchable tables, and syntax-highlighted code recommendations.
18. **Mimalloc Global Allocator:** Built-in high-performance memory allocator for fast multi-threaded allocations and minimal parsing overhead.

## Build & Install

```bash
cargo build --release
```

Binary is placed at `target/release/gprof`.

Install globally:
```bash
cargo install --path crates/gprof-cli
```

## Quick Start

```bash
# Full executive report (Markdown output to stdout)
gprof capture.json

# Generate standalone interactive HTML dashboard
gprof capture.json --html report.html

# Supports compressed captures directly
gprof capture.json.gz

# Compare two captures (before vs after optimization)
gprof before.json --compare after.json
gprof compare before.json after.json

# Export to file
gprof capture.json --export=report.md

# Output machine-readable JSON
gprof capture.json --json
```

## Targeted Inspection Views

```bash
# Babylon.js Performance Linter & TypeScript code fixes
gprof capture.json --lint
gprof lint capture.json

# Material & shader sorting efficiency & ping-pong switches
gprof capture.json --sorting
gprof sorting capture.json

# Texture compression (KTX2 / ASTC / BC7) & NPOT audit
gprof capture.json --compression
gprof compression capture.json

# Render Targets (FBO) VRAM footprint & oversized viewports
gprof capture.json --fbos
gprof fbos capture.json

# Instancing & batching candidates (potential saved draw calls)
gprof capture.json --instancing
gprof instancing capture.json

# WebGL state thrashing & redundancy audit (depthMask, blend, depthTest)
gprof capture.json --states
gprof states capture.json

# Geometry buffers, dynamic uploads (bufferSubData) & 32-bit index meshes
gprof capture.json --buffers
gprof buffers capture.json

# GlowLayer & occluder waste audit
gprof capture.json --glow
gprof glow capture.json

# Texture memory breakdown & warnings
gprof capture.json --textures --top 20
gprof textures capture.json

# Heaviest draw calls by triangle count
gprof capture.json --draws --top 15
gprof draws capture.json

# Shaders & PBR variant diffs
gprof capture.json --shaders
gprof shaders capture.json

# GPU particle systems (Transform Feedback)
gprof capture.json --particles
gprof particles capture.json

# Per-frame dynamic uploads (texImage2D stack traces)
gprof capture.json --uploads
gprof uploads capture.json

# Render passes sequence
gprof capture.json --passes
gprof passes capture.json
```

## Architecture

- **`crates/gprof-core`**: Core library. High-performance JSON parser via memory-mapped I/O (`memmap2`) with transparent gzip decompression (`flate2`), WebGL state machine reconstruction, and domain-specific analysis passes (`compression`, `fbo`, `sorting`, `lint`, `html`).
- **`crates/gprof-cli`**: CLI binary (`gprof`) built with `clap`, `mimalloc` global allocator, markdown report generation, and formatting.

## Benchmark

On a **160 MB** SpectorJS capture (2,092 commands, 262 draw calls, 191 textures, 43 shader programs):
- **Parse time:** ~75 ms
- **Analysis time:** < 2 ms
- **Total time:** < 80 ms
