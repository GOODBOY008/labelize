/**
 * Parse ZPL/EPL and render to PNG (or PDF when `want_pdf` is true).
 *
 * `src` is the raw label data (ZPL by default; pass `is_epl = true` for EPL
 * input, matching the server's Content-Type detection).
 *
 * Errors throw a string prefixed with the stage code: `1:` is a parse error
 * (HTTP 400 equivalent), `2:` a rendering error (HTTP 500 equivalent).
 */
export function lz_render(
  src: Uint8Array,
  width_mm: number,
  height_mm: number,
  dpmm: number,
  grayscale: boolean,
  want_pdf: boolean,
  is_epl: boolean,
): Uint8Array;

/** The playground HTML page served by the labelize HTTP server. */
export function lz_playground_html(): string;