// Types for matenadar.js and client.js. Lengths are points; positions are
// from the trim's top-left corner, x to the right and y down.

export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

/** A Matenadar document (docs/model.md). */
export type MatenadarDocument = { matenadar: "1"; [key: string]: Json };

/** A Matenadar operation (docs/model.md, "Operations"). Nodes are addressed by `id`. */
export type Op =
  | { op: "set"; id?: string; at?: string; path: string; value: Json }
  | { op: "unset"; id?: string; at?: string; path: string }
  | { op: "insert"; id?: string; at?: string; field?: string; index?: number; before?: string; after?: string; value: Json }
  | { op: "remove"; id?: string; at?: string }
  | { op: "move"; id?: string; at?: string; to: { id?: string; at?: string }; field: string; index?: number }
  | { op: "insert-text"; id?: string; at?: string; offset: number; text: string }
  | { op: "delete-text"; id?: string; at?: string; start: number; end: number };

export interface Rect {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

export interface PageInfo {
  page: number;
  /** The page number as printed ("iv", "12"). */
  label: string;
  side: "recto" | "verso";
  /** The section or page it belongs to, and that item's master. */
  item: string | null;
  master: string | null;
  /** The trim box is [0, 0, w, h]. */
  trim: { w: number; h: number };
  /** How far the bleed reaches past each edge of the trim. */
  bleed: { top: number; right: number; bottom: number; left: number };
  /** The master's margins: the text area's distance from the trim. */
  margins: { top: number; bottom: number; inside: number; outside: number } | null;
}

export interface NodeRef {
  id: string;
  kind: string;
}

export interface Hit {
  /** The innermost node: a paragraph, heading, figure, element. */
  id: string;
  kind: string;
  page: number;
  /** The node's extent on this page. */
  rect: Rect;
  /** A byte offset into the node's plain text, when the point is on words. */
  offset?: number;
  /** The innermost placed element around it (a text frame, an image). */
  element?: NodeRef;
  ancestors: NodeRef[];
}

export interface Issue {
  severity: "error" | "warning" | "info";
  code: "overflow" | "text-in-margin" | "low-resolution" | "missing-glyph" | "fallback-font" | "widow" | "orphan" | "missing-file" | "compiler";
  page?: number;
  node?: string;
  message: string;
}

/** What an edit, undo or redo did. */
export interface Edited {
  /** Pages to draw again (1-based), including new ones. */
  changed_pages: number[];
  page_count: number;
  /** Layout time; 0 in WebAssembly, which has no clock (time the call instead). */
  layout_ms: number;
  diagnostics: Issue[];
  can_undo: boolean;
  can_redo: boolean;
}

export interface Nudged extends Edited {
  lines_before: number;
  lines_after: number;
  /** The paragraph's tracking now, in ems. */
  tracking: number;
}

/** The edits since the document opened (or the last rebase). */
export interface Patch {
  /** Changes to the words, for the manuscript (in Aldus, the chapter's document). */
  words: Op[];
  /** The nodes whose words changed; `node(id)` gives their words now (null: removed). */
  touched: string[];
  /** Changes to the layout, each field folded to its last setting: what `aldus-engine layout apply` keeps. */
  layout: Op[];
}

export interface Opened {
  pages: number;
  problems: string[];
  warnings: string[];
  diagnostics: Issue[];
}

export interface CheckOptions {
  /** Live text this far inside the trim (default 0.25in). */
  safe?: string | number;
  /** The spine-side margin (default KDP's for the page count). */
  gutter?: string | number;
  min_ppi?: number;
}

export interface RenderOptions {
  /** Pixels per point (1 = 72 dpi); or give `dpi`. */
  scale?: number;
  dpi?: number;
  /** Draw the bleed around the trim. */
  bleed?: boolean;
}

export interface Pixels {
  width: number;
  height: number;
  data: Uint8ClampedArray;
}

type Bytes = ArrayBuffer | ArrayBufferView;
type BreakTo = "page" | "recto" | "verso" | null;

/** The engine's methods; client.js gives each as a promise. */
export interface MatenadarApi {
  assets(doc: MatenadarDocument): { fonts: string[]; files: string[] };
  addFont(data: Bytes): number;
  addFile(path: string, data: Bytes): void;
  open(doc: MatenadarDocument, check?: CheckOptions): Opened;
  fonts(): string[];
  close(): void;

  pages(): PageInfo[];
  spreads(): number[][];
  svg(page: number, options?: { bleed?: boolean }): string;
  png(page: number, options?: RenderOptions): Uint8Array;
  rgba(page: number, options?: RenderOptions): Pixels;
  pdf(): Uint8Array;
  fingerprint(): string[];

  hit(page: number, x: number, y: number): Hit | null;
  rects(id: string): { page: number; rect: Rect }[];
  node(id: string): Json;
  lines(id: string): number;
  locate(page: number, x: number, y: number): { node: string; offset: number } | null;
  position(id: string): { page: number; x: number; y: number } | null;
  layout(pages?: number[]): { page: number; label: string; width: number; height: number; nodes: { id: string; kind: string; rect: Rect }[] }[];
  document(): MatenadarDocument;

  apply(ops: Op | Op[]): Edited;
  setWords(id: string, content: Json[]): Edited;
  insertText(id: string, offset: number, text: string): Edited;
  deleteText(id: string, start: number, end: number): Edited;
  insertBlock(where: { after: string } | { before: string }, block: Json): Edited;
  removeBlock(id: string): Edited;
  setFields(id: string, fields: Record<string, Json>, options?: { style?: boolean }): Edited;
  setPicture(id: string, fields: Record<string, Json>): Edited;
  setBlock(id: string, fields: Record<string, Json>): Edited;
  moveFrame(id: string, frame: { x?: Json; y?: Json; w?: Json; h?: Json; rotate?: Json }): Edited;
  breakBefore(id: string, to?: BreakTo): Edited;
  hideOnPage(id: string, hide: string[]): Edited;
  nudge(id: string, lines: -1 | 1): Nudged;
  undo(): Edited;
  redo(): Edited;

  diagnostics(): Issue[];
  check(options?: CheckOptions): Issue[];
  patch(): Patch;
  rebase(): void;
}

export declare class Matenadar implements MatenadarApi {
  static load(source: Response | Promise<Response> | BufferSource | WebAssembly.Module): Promise<Matenadar>;
  assets(doc: MatenadarDocument): { fonts: string[]; files: string[] };
  addFont(data: Bytes): number;
  addFile(path: string, data: Bytes): void;
  open(doc: MatenadarDocument, check?: CheckOptions): Opened;
  fonts(): string[];
  close(): void;
  pages(): PageInfo[];
  spreads(): number[][];
  svg(page: number, options?: { bleed?: boolean }): string;
  png(page: number, options?: RenderOptions): Uint8Array;
  rgba(page: number, options?: RenderOptions): Pixels;
  pdf(): Uint8Array;
  fingerprint(): string[];
  hit(page: number, x: number, y: number): Hit | null;
  rects(id: string): { page: number; rect: Rect }[];
  node(id: string): Json;
  lines(id: string): number;
  locate(page: number, x: number, y: number): { node: string; offset: number } | null;
  position(id: string): { page: number; x: number; y: number } | null;
  layout(pages?: number[]): { page: number; label: string; width: number; height: number; nodes: { id: string; kind: string; rect: Rect }[] }[];
  document(): MatenadarDocument;
  apply(ops: Op | Op[]): Edited;
  setWords(id: string, content: Json[]): Edited;
  insertText(id: string, offset: number, text: string): Edited;
  deleteText(id: string, start: number, end: number): Edited;
  insertBlock(where: { after: string } | { before: string }, block: Json): Edited;
  removeBlock(id: string): Edited;
  setFields(id: string, fields: Record<string, Json>, options?: { style?: boolean }): Edited;
  setPicture(id: string, fields: Record<string, Json>): Edited;
  setBlock(id: string, fields: Record<string, Json>): Edited;
  moveFrame(id: string, frame: { x?: Json; y?: Json; w?: Json; h?: Json; rotate?: Json }): Edited;
  breakBefore(id: string, to?: BreakTo): Edited;
  hideOnPage(id: string, hide: string[]): Edited;
  nudge(id: string, lines: -1 | 1): Nudged;
  undo(): Edited;
  redo(): Edited;
  diagnostics(): Issue[];
  check(options?: CheckOptions): Issue[];
  patch(): Patch;
  rebase(): void;
}

/** Every method of MatenadarApi as a promise, run in a worker. */
export type Async<T> = { [K in keyof T]: T[K] extends (...args: infer A) => infer R ? (...args: A) => Promise<R> : never };

export declare class MatenadarClient {
  /** `base`: the folder with worker.js, matenadar.js and matenadar.wasm. */
  static start(options?: { base?: string | URL; worker?: Worker }): Promise<MatenadarClient & Async<MatenadarApi>>;
  call(method: keyof MatenadarApi, args?: unknown[]): Promise<unknown>;
  terminate(): void;
}
