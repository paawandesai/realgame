import { CanvasTexture, Mesh, MeshBasicMaterial, PlaneGeometry, SRGBColorSpace } from '@iwsdk/core';

/**
 * A flat text panel drawn on a canvas. Cheap to redraw, crisp at desk
 * distance, and needs no font assets.
 */
export class Label {
  readonly mesh: Mesh;
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private texture: CanvasTexture;
  private last = '';

  constructor(
    private widthM: number,
    private heightM: number,
    private opts: { px?: number; bg?: string; color?: string; font?: number; align?: CanvasTextAlign } = {},
  ) {
    const px = opts.px ?? 1024;
    this.canvas = document.createElement('canvas');
    this.canvas.width = px;
    this.canvas.height = Math.round((px * heightM) / widthM);
    this.ctx = this.canvas.getContext('2d')!;
    this.texture = new CanvasTexture(this.canvas);
    this.texture.colorSpace = SRGBColorSpace;
    this.texture.anisotropy = 4;
    const mat = new MeshBasicMaterial({ map: this.texture, transparent: true, depthWrite: false });
    this.mesh = new Mesh(new PlaneGeometry(widthM, heightM), mat);
  }

  /** Lines are wrapped to the panel width; a line starting with `#` is drawn large. */
  set(text: string): void {
    if (text === this.last) return;
    this.last = text;
    const { ctx, canvas } = this;
    const W = canvas.width;
    const H = canvas.height;
    ctx.clearRect(0, 0, W, H);
    if (this.opts.bg) {
      ctx.fillStyle = this.opts.bg;
      const r = Math.min(W, H) * 0.12;
      ctx.beginPath();
      ctx.roundRect(2, 2, W - 4, H - 4, r);
      ctx.fill();
    }
    const base = this.opts.font ?? Math.round(H / 5);
    const align = this.opts.align ?? 'center';
    ctx.textAlign = align;
    ctx.textBaseline = 'top';
    ctx.fillStyle = this.opts.color ?? '#ffffff';
    const pad = W * 0.04;
    const x = align === 'center' ? W / 2 : pad;
    const lines: { text: string; size: number }[] = [];
    for (const raw of text.split('\n')) {
      const big = raw.startsWith('#');
      const size = big ? base * 1.35 : base;
      ctx.font = `${big ? 700 : 500} ${size}px system-ui, -apple-system, "Segoe UI", sans-serif`;
      let line = '';
      for (const word of (big ? raw.slice(1).trim() : raw).split(' ')) {
        const next = line ? line + ' ' + word : word;
        if (ctx.measureText(next).width > W - pad * 2 && line) {
          lines.push({ text: line, size });
          line = word;
        } else line = next;
      }
      lines.push({ text: line, size });
    }
    const total = lines.reduce((s, l) => s + l.size * 1.25, 0);
    let y = Math.max(pad * 0.5, (H - total) / 2);
    for (const l of lines) {
      ctx.font = `${l.size === base ? 500 : 700} ${l.size}px system-ui, -apple-system, "Segoe UI", sans-serif`;
      ctx.fillText(l.text, x, y);
      y += l.size * 1.25;
    }
    this.texture.needsUpdate = true;
  }
}
