// Interactive unit playground, backed by the real `unitarrow-core` compiled to
// WebAssembly. Nothing here reimplements the rules — every answer on screen is
// the same code path the Rust tests and the conformance suite exercise.
//
// The ABI is raw (no wasm-bindgen): strings cross as UTF-8 in linear memory,
// and every call returns a packed (ptr << 32) | len handle to a JSON reply.

(() => {
  "use strict";

  // mkdocs serve does not fingerprint assets, and a rebuilt module is a
// different module — without this a stale wasm silently outlives a rebuild.
const cacheBust = `?v=${Math.floor(Date.now() / 1000)}`;
const WASM_URL = new URL("../../wasm/unitarrow_wasm.wasm", document.baseURI).href + cacheBust;
  const REGISTRY_URL = new URL("../../registry/conformance-core.toml", document.baseURI).href + cacheBust;

  let wasm = null;
  let ready = null;

  const enc = new TextEncoder();
  const dec = new TextDecoder();

  function mem() {
    return new Uint8Array(wasm.memory.buffer);
  }

  /** Copy a JS string into linear memory; returns [ptr, len]. */
  function put(str) {
    const bytes = enc.encode(str);
    const ptr = wasm.ua_alloc(bytes.length);
    mem().set(bytes, ptr);
    return [ptr, bytes.length];
  }

  /** Unpack a (ptr << 32) | len reply, decode it, and free it. */
  function take(packed) {
    const ptr = Number(packed >> 32n);
    const len = Number(packed & 0xffffffffn);
    const text = dec.decode(mem().subarray(ptr, ptr + len));
    wasm.ua_free(ptr, len);
    return JSON.parse(text);
  }

  function call(fn, str) {
    const [ptr, len] = put(str);
    try {
      return take(fn(ptr, len));
    } finally {
      wasm.ua_free(ptr, len);
    }
  }

  async function boot() {
    if (ready) return ready;
    ready = (async () => {
      const [mod, toml] = await Promise.all([
        WebAssembly.instantiateStreaming(fetch(WASM_URL), {}).catch(async () => {
          // Some dev servers serve .wasm without the right MIME type.
          const bytes = await (await fetch(WASM_URL)).arrayBuffer();
          return WebAssembly.instantiate(bytes, {});
        }),
        fetch(REGISTRY_URL).then((r) => r.text()),
      ]);
      wasm = mod.instance.exports;
      const loaded = call(wasm.ua_load_registry, toml);
      if (!loaded.ok) throw new Error(`${loaded.code}: ${loaded.message}`);
      return loaded;
    })();
    return ready;
  }

  const canonicalize = (s) => call(wasm.ua_canonicalize, s);
  const convert = (from, to) => call(wasm.ua_convert, `${from}\n${to}`);

  // ---------------------------------------------------------------- rendering

  function dimText(dim) {
    const parts = Object.entries(dim);
    if (!parts.length) return "dimensionless";
    return parts.map(([n, e]) => (e === 1 ? n : `${n}^${e}`)).join(" · ");
  }

  function renderCanonical(input, box) {
    if (!input.trim()) {
      box.className = "ua-out ua-idle";
      box.textContent = "Type a unit string above.";
      return;
    }
    const r = canonicalize(input);
    if (r.ok) {
      box.className = "ua-out ua-ok";
      box.innerHTML =
        `<div class="ua-row"><span class="ua-label">canonical</span>` +
        `<code class="ua-big">${escapeHtml(r.canonical)}</code></div>` +
        `<div class="ua-row"><span class="ua-label">dimension</span>` +
        `<code>${escapeHtml(dimText(r.dimension))}</code></div>` +
        (r.describes
          ? `<div class="ua-row"><span class="ua-label">reads as</span>` +
            `<span>${escapeHtml(r.describes)}</span></div>`
          : "");
    } else {
      box.className = "ua-out ua-err";
      const caret =
        r.offset !== null && r.offset !== undefined
          ? `<pre class="ua-caret">${escapeHtml(input)}\n${" ".repeat(r.offset)}^</pre>`
          : "";
      box.innerHTML =
        `<div class="ua-row"><span class="ua-label">error</span>` +
        `<code class="ua-code">${escapeHtml(r.code)}</code></div>` +
        caret +
        `<div class="ua-msg">${escapeHtml(r.message)}</div>`;
    }
  }

  function renderConvert(from, to, box) {
    if (!from.trim() || !to.trim()) {
      box.className = "ua-out ua-idle";
      box.textContent = "Enter both units.";
      return;
    }
    const r = convert(from, to);
    if (!r.ok) {
      box.className = "ua-out ua-err";
      box.innerHTML =
        `<div class="ua-row"><span class="ua-label">error</span>` +
        `<code class="ua-code">${escapeHtml(r.code)}</code></div>` +
        `<div class="ua-msg">${escapeHtml(r.message)}</div>`;
      return;
    }
    const num = BigInt(r.num);
    const den = BigInt(r.den);
    const offNum = BigInt(r.offNum);
    const offDen = BigInt(r.offDen);
    const factorF = Number(num) / Number(den);
    const offsetF = Number(offNum) / Number(offDen);
    const sample = [0, 1, 20, 100, -40];
    const rows = sample
      .map((v) => {
        const out = v * factorF + offsetF;
        const shown = Number.isInteger(out) ? out : Number(out.toFixed(6));
        return `<tr><td><code>${v}</code></td><td><code>${shown}</code></td></tr>`;
      })
      .join("");
    box.className = "ua-out ua-ok";
    box.innerHTML =
      `<div class="ua-row"><span class="ua-label">exact factor</span>` +
      `<code class="ua-big">${escapeHtml(r.num)}${den === 1n ? "" : " / " + escapeHtml(r.den)}</code></div>` +
      (offNum === 0n
        ? ""
        : `<div class="ua-row"><span class="ua-label">exact offset</span>` +
          `<code>${escapeHtml(r.offNum)}${offDen === 1n ? "" : " / " + escapeHtml(r.offDen)}</code></div>`) +
      `<div class="ua-row"><span class="ua-label">formula</span><code>${escapeHtml(r.formula)}</code></div>` +
      `<table class="ua-table"><thead><tr><th>${escapeHtml(r.from)}</th><th>${escapeHtml(r.to)}</th></tr></thead>` +
      `<tbody>${rows}</tbody></table>`;
  }

  function escapeHtml(s) {
    return String(s).replace(/[&<>"']/g, (c) => ({
      "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
    }[c]));
  }

  // ------------------------------------------------------------------- wiring

  function wire(root) {
    const kind = root.dataset.playground;
    if (kind === "canonical") {
      const input = root.querySelector("input");
      const out = root.querySelector(".ua-out");
      const run = () => renderCanonical(input.value, out);
      input.addEventListener("input", run);
      root.querySelectorAll("[data-example]").forEach((btn) => {
        btn.addEventListener("click", () => {
          input.value = btn.dataset.example;
          run();
          input.focus();
        });
      });
      run();
    } else if (kind === "convert") {
      const [from, to] = root.querySelectorAll("input");
      const out = root.querySelector(".ua-out");
      const run = () => renderConvert(from.value, to.value, out);
      from.addEventListener("input", run);
      to.addEventListener("input", run);
      root.querySelectorAll("[data-pair]").forEach((btn) => {
        btn.addEventListener("click", () => {
          const [a, b] = btn.dataset.pair.split("|");
          from.value = a;
          to.value = b;
          run();
        });
      });
      run();
    }
  }

  async function init() {
    const roots = document.querySelectorAll("[data-playground]");
    if (!roots.length) return;
    try {
      const info = await boot();
      document.querySelectorAll("[data-registry-info]").forEach((el) => {
        el.textContent = `${info.name}@${info.version} — ${info.units} units, ${info.aliases} aliases`;
      });
      roots.forEach(wire);
    } catch (err) {
      roots.forEach((r) => {
        const out = r.querySelector(".ua-out");
        if (out) {
          out.className = "ua-out ua-err";
          out.textContent = `Could not load the WebAssembly module: ${err.message}`;
        }
      });
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }

  // mkdocs-material's instant navigation swaps the body without a reload.
  if (window.document$ && typeof window.document$.subscribe === "function") {
    window.document$.subscribe(init);
  }
})();
