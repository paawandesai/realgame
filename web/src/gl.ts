// Optional WebGL1 world renderer. The simulation still draws every pixel's
// base colour (one palette, shared with Canvas2D); the GPU adds what is cheap
// there and costly on the CPU: flame flicker and heat shimmer from per-pixel
// species and temperature, the Insight lens as a palette lookup, and a soft
// bloom computed at 160 x 90. Shaders are plain strings in the bundle, so the
// Content Security Policy is unchanged. If anything here is unavailable the
// caller keeps Canvas2D.

/** Species ids from `crates/sim/src/species.rs` (`Species as u8`). */
const SPECIES_FLAME = 11;
const SPECIES_CRYSTAL = 13;
/** Species byte the sim writes for sprite pixels (player, torch, crosshair). */
const SPRITE = 255;
/** Backing store cap, in pixels per world cell. Two keeps fill cost low on
 * weak GPUs; CSS scales the rest of the way with crisp pixels. */
const MAX_PX_PER_CELL = 2;
const BLOOM_W = 160;
const BLOOM_H = 90;

const VERT = `
attribute vec2 a_pos;
varying vec2 v_uv;
void main() {
  v_uv = a_pos * 0.5 + 0.5;
  gl_Position = vec4(a_pos, 0.0, 1.0);
}`;

/** Shared by the passes that read the sim's textures. Rows arrive top first,
 * so world texture v runs opposite to GL's y. */
const WORLD_COMMON = `
precision mediump float;
uniform sampler2D u_base;
uniform sampler2D u_data;
uniform vec2 u_cells;
varying vec2 v_uv;
const float FLAME = ${SPECIES_FLAME}.0;
const float CRYSTAL = ${SPECIES_CRYSTAL}.0;
const float SPRITE = ${SPRITE}.0;
vec2 worldUv(vec2 uv) { return vec2(uv.x, 1.0 - uv.y); }
float species(vec4 d) { return floor(d.a * 255.0 + 0.5); }
// Inverse of palette::temp_byte: square-root spaced over -60..2800 C.
float tempC(vec4 d) { return d.r * d.r * 2860.0 - 60.0; }
float hash(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453); }
`;

/** Bright pass: what glows, averaged into the 160 x 90 bloom buffer. */
const BRIGHT = `${WORLD_COMMON}
uniform float u_lens;
vec3 emission(vec2 uv) {
  vec4 d = texture2D(u_data, uv);
  vec3 c = texture2D(u_base, uv).rgb;
  float s = species(d);
  float t = tempC(d);
  if (s == SPRITE) return vec3(0.0);
  if (s == FLAME) return c * 1.8;
  if (s == CRYSTAL) return c * 0.9;
  if (t > 400.0) return vec3(1.0, 0.43, 0.12) * clamp((t - 400.0) / 1000.0, 0.25, 1.4);
  return vec3(0.0);
}
void main() {
  vec2 uv = worldUv(v_uv);
  vec2 px = 0.5 / u_cells;
  vec3 e = emission(uv + vec2(-px.x, -px.y)) + emission(uv + vec2(px.x, -px.y))
         + emission(uv + vec2(-px.x, px.y)) + emission(uv + vec2(px.x, px.y));
  gl_FragColor = vec4(e * 0.5 * (1.0 - 0.7 * u_lens), 1.0);
}`;

/** Separable 9-tap Gaussian blur over the bloom buffer. */
const BLUR = `
precision mediump float;
uniform sampler2D u_src;
uniform vec2 u_step;
varying vec2 v_uv;
void main() {
  vec3 c = texture2D(u_src, v_uv).rgb * 0.227027;
  c += texture2D(u_src, v_uv + u_step * 1.384615).rgb * 0.316216;
  c += texture2D(u_src, v_uv - u_step * 1.384615).rgb * 0.316216;
  c += texture2D(u_src, v_uv + u_step * 3.230769).rgb * 0.070270;
  c += texture2D(u_src, v_uv - u_step * 3.230769).rgb * 0.070270;
  gl_FragColor = vec4(c, 1.0);
}`;

/** Final pass at screen resolution: crisp cells, effects, bloom on top. */
const COMPOSITE = `${WORLD_COMMON}
uniform sampler2D u_bloom;
uniform sampler2D u_lut;
uniform float u_lens;
uniform float u_time;
uniform float u_motion;
uniform float u_view_x;
void main() {
  vec2 uv = worldUv(v_uv);
  vec2 cell = floor(uv * u_cells);
  vec2 cuv = (cell + 0.5) / u_cells;
  vec4 d = texture2D(u_data, cuv);
  vec3 c = texture2D(u_base, cuv).rgb;
  float s = species(d);
  float t = tempC(d);
  // World-space cell, so noise sticks to the world as the camera scrolls.
  vec2 wc = vec2(cell.x + u_view_x, cell.y);
  if (s != SPRITE) {
    if (u_lens > 0.5) {
      vec3 heat = texture2D(u_lut, vec2(d.r * (255.0 / 256.0) + 0.5 / 256.0, 0.5)).rgb;
      // Solids keep their hue but darker, so shapes stay readable (palette::lens_solid).
      bool solid = s == 1.0 || s == 2.0 || s == 4.0 || s == 12.0 || s == CRYSTAL;
      c = solid ? mix(heat, vec3(0.039, 0.047, 0.078), 0.62) : heat;
    } else if (s == FLAME) {
      // Flicker: each cell picks a new brightness about six times a second.
      float n = hash(wc + floor(u_time * 6.0) * vec2(7.13, 3.71));
      c *= mix(1.0, 0.8 + 0.4 * n, u_motion);
    } else if (t > 400.0) {
      float n = hash(wc + floor(u_time * 4.0) * vec2(1.7, 9.2));
      c += vec3(0.12, 0.05, 0.0) * n * u_motion * clamp((t - 400.0) / 1000.0, 0.0, 1.0);
    }
  }
  vec3 bloom = texture2D(u_bloom, v_uv).rgb;
  gl_FragColor = vec4(c + bloom * 3.0, 1.0);
}`;

interface Pass {
  prog: WebGLProgram;
  uni: Record<string, WebGLUniformLocation | null>;
}

interface Target {
  tex: WebGLTexture;
  fbo: WebGLFramebuffer;
}

export interface GlFrame {
  /** VIEW_W * HEIGHT RGBA bytes from the sim. */
  rgba: Uint8Array;
  /** VIEW_W * HEIGHT (temperature byte, species) pairs from the sim. */
  data: Uint8Array;
  lens: boolean;
  /** Seconds, for flicker. */
  time: number;
  /** 0 with reduced motion (no flicker or shimmer), else 1. */
  motion: number;
  /** Camera left edge in world cells. */
  viewX: number;
}

export class GlRenderer {
  /** Set when the browser drops the context; the caller falls back to 2D. */
  lost = false;

  private constructor(
    readonly canvas: HTMLCanvasElement,
    private readonly gl: WebGLRenderingContext,
    private readonly w: number,
    private readonly h: number,
    private readonly base: WebGLTexture,
    private readonly data: WebGLTexture,
    private readonly lut: WebGLTexture,
    private readonly bloomA: Target,
    private readonly bloomB: Target,
    private readonly bright: Pass,
    private readonly blur: Pass,
    private readonly composite: Pass,
  ) {
    canvas.addEventListener("webglcontextlost", (e) => {
      e.preventDefault();
      this.lost = true;
    });
    const resize = () => this.resize();
    new ResizeObserver(resize).observe(canvas);
    resize();
  }

  /** A renderer on `canvas`, or null if WebGL or any part of the setup is
   * unavailable (the caller then uses Canvas2D). Never throws or logs. */
  static create(canvas: HTMLCanvasElement, w: number, h: number, lut: Uint8Array): GlRenderer | null {
    try {
      const gl = canvas.getContext("webgl", {
        alpha: false,
        antialias: false,
        depth: false,
        stencil: false,
        premultipliedAlpha: false,
        preserveDrawingBuffer: false,
        powerPreference: "low-power",
      });
      if (!gl) return null;
      const bright = program(gl, BRIGHT, ["u_base", "u_data", "u_cells", "u_lens"]);
      const blur = program(gl, BLUR, ["u_src", "u_step"]);
      const composite = program(gl, COMPOSITE, [
        "u_base",
        "u_data",
        "u_cells",
        "u_bloom",
        "u_lut",
        "u_lens",
        "u_time",
        "u_motion",
        "u_view_x",
      ]);
      if (!bright || !blur || !composite) return null;
      // One big triangle covers the screen.
      const buf = gl.createBuffer();
      if (!buf) return null;
      gl.bindBuffer(gl.ARRAY_BUFFER, buf);
      gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
      for (const p of [bright, blur, composite]) {
        const loc = gl.getAttribLocation(p.prog, "a_pos");
        if (loc < 0) return null;
        gl.enableVertexAttribArray(loc);
        gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
      }
      gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
      const base = texture(gl, gl.NEAREST);
      const data = texture(gl, gl.NEAREST);
      const lutTex = texture(gl, gl.LINEAR);
      if (!base || !data || !lutTex) return null;
      gl.bindTexture(gl.TEXTURE_2D, base);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      gl.bindTexture(gl.TEXTURE_2D, data);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.LUMINANCE_ALPHA, w, h, 0, gl.LUMINANCE_ALPHA, gl.UNSIGNED_BYTE, null);
      gl.bindTexture(gl.TEXTURE_2D, lutTex);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 256, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, lut);
      const a = target(gl);
      const b = target(gl);
      if (!a || !b) return null;
      if (gl.getError() !== gl.NO_ERROR) return null;
      return new GlRenderer(canvas, gl, w, h, base, data, lutTex, a, b, bright, blur, composite);
    } catch {
      return null;
    }
  }

  private resize(): void {
    const r = this.canvas.getBoundingClientRect();
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const k = Math.max(1, Math.min((r.width * dpr) / this.w, MAX_PX_PER_CELL));
    this.canvas.width = Math.round(this.w * k);
    this.canvas.height = Math.round(this.h * k);
  }

  draw(f: GlFrame): void {
    const gl = this.gl;
    if (this.lost || gl.isContextLost()) {
      this.lost = true;
      return;
    }
    // Upload this frame straight from WebAssembly memory.
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.base);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, this.w, this.h, gl.RGBA, gl.UNSIGNED_BYTE, f.rgba);
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.data);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, this.w, this.h, gl.LUMINANCE_ALPHA, gl.UNSIGNED_BYTE, f.data);
    gl.activeTexture(gl.TEXTURE2);
    gl.bindTexture(gl.TEXTURE_2D, this.lut);
    const lens = f.lens ? 1 : 0;

    // Bright pass into bloom A, then blur A -> B -> A.
    gl.viewport(0, 0, BLOOM_W, BLOOM_H);
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.bloomA.fbo);
    this.use(this.bright);
    gl.uniform1i(this.bright.uni.u_base!, 0);
    gl.uniform1i(this.bright.uni.u_data!, 1);
    gl.uniform2f(this.bright.uni.u_cells!, this.w, this.h);
    gl.uniform1f(this.bright.uni.u_lens!, lens);
    gl.drawArrays(gl.TRIANGLES, 0, 3);

    this.use(this.blur);
    gl.activeTexture(gl.TEXTURE3);
    gl.uniform1i(this.blur.uni.u_src!, 3);
    // Two rounds, the second twice as wide: a soft halo about 12 cells across.
    for (const spread of [1, 2]) {
      gl.bindFramebuffer(gl.FRAMEBUFFER, this.bloomB.fbo);
      gl.bindTexture(gl.TEXTURE_2D, this.bloomA.tex);
      gl.uniform2f(this.blur.uni.u_step!, spread / BLOOM_W, 0);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
      gl.bindFramebuffer(gl.FRAMEBUFFER, this.bloomA.fbo);
      gl.bindTexture(gl.TEXTURE_2D, this.bloomB.tex);
      gl.uniform2f(this.blur.uni.u_step!, 0, spread / BLOOM_H);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    }

    // Composite to the screen.
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.bindTexture(gl.TEXTURE_2D, this.bloomA.tex);
    const c = this.composite;
    this.use(c);
    gl.uniform1i(c.uni.u_base!, 0);
    gl.uniform1i(c.uni.u_data!, 1);
    gl.uniform1i(c.uni.u_lut!, 2);
    gl.uniform1i(c.uni.u_bloom!, 3);
    gl.uniform2f(c.uni.u_cells!, this.w, this.h);
    gl.uniform1f(c.uni.u_lens!, lens);
    gl.uniform1f(c.uni.u_time!, f.time);
    gl.uniform1f(c.uni.u_motion!, f.motion);
    gl.uniform1f(c.uni.u_view_x!, f.viewX);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  private use(p: Pass): void {
    this.gl.useProgram(p.prog);
  }
}

function shader(gl: WebGLRenderingContext, type: number, src: string): WebGLShader | null {
  const s = gl.createShader(type);
  if (!s) return null;
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
    gl.deleteShader(s);
    return null;
  }
  return s;
}

function program(gl: WebGLRenderingContext, frag: string, uniforms: string[]): Pass | null {
  const vs = shader(gl, gl.VERTEX_SHADER, VERT);
  const fs = shader(gl, gl.FRAGMENT_SHADER, frag);
  const prog = gl.createProgram();
  if (!vs || !fs || !prog) return null;
  gl.attachShader(prog, vs);
  gl.attachShader(prog, fs);
  // Every program reads the triangle from attribute 0.
  gl.bindAttribLocation(prog, 0, "a_pos");
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) return null;
  const uni: Record<string, WebGLUniformLocation | null> = {};
  for (const u of uniforms) uni[u] = gl.getUniformLocation(prog, u);
  return { prog, uni };
}

function texture(gl: WebGLRenderingContext, filter: number): WebGLTexture | null {
  const t = gl.createTexture();
  if (!t) return null;
  gl.bindTexture(gl.TEXTURE_2D, t);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  return t;
}

function target(gl: WebGLRenderingContext): Target | null {
  const tex = texture(gl, gl.LINEAR);
  const fbo = gl.createFramebuffer();
  if (!tex || !fbo) return null;
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, BLOOM_W, BLOOM_H, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
  gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
  gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
  const ok = gl.checkFramebufferStatus(gl.FRAMEBUFFER) === gl.FRAMEBUFFER_COMPLETE;
  gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  return ok ? { tex, fbo } : null;
}
