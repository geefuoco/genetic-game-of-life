import { worldWindow, type Camera, type Size } from './camera.ts';
import { paletteTable } from './palette.ts';
import { CARRYING } from '../protocol.ts';

/** What to draw and where: the camera and the viewport in CSS pixels. */
export interface View {
  camera: Camera;
  viewport: Size;
  pixelRatio: number;
}

/** The Phenotypes of one Generation, and any Carrier highlight of it. */
export interface Cells {
  width: number;
  height: number;
  phenotypes: Float32Array;
  highlight: {
    /** Hue of the highlighted Allele, in degrees. */
    hue: number;
    /** One `CARRYING` number per Cell in row-major order. */
    carriers: Uint8Array;
  } | null;
}

/** Page background, `#06070a`, as sRGB channels in `0..1`. */
const BACKGROUND = [6 / 255, 7 / 255, 10 / 255] as const;
/** Colour of a live Cell a Carrier highlight leaves out, `#181a21`. */
const DIMMED = [24 / 255, 26 / 255, 33 / 255] as const;
/** Width of a hidden Carrier's ring as a fraction of the drawn square, and at least one device pixel. */
const RING_FRACTION = 0.2;
/** Distance over which the World window's edges fade into the background, in CSS pixels. */
const FADE_PX = 70;
/** Zoom, in CSS pixels per Cell, from which Cells have a gap between them. */
const GAP_FROM = 4;
/** Zoom from which Cells have rounded corners. */
const ROUNDED_FROM = 5;
/** The gap as a fraction of the Cell size. */
const GAP_FRACTION = 0.12;
/** The corner radius as a fraction of the drawn square. */
const RADIUS_FRACTION = 0.34;
/** Hues sampled into the palette lookup texture. */
const PALETTE_SIZE = 1024;

const VERTEX = `#version 300 es
void main() {
  vec2 corner = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  gl_Position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);
}
`;

const FRAGMENT = `#version 300 es
precision highp float;
precision highp int;
precision highp sampler2D;

uniform sampler2D u_cells;
uniform sampler2D u_palette;
uniform highp usampler2D u_carriers;
uniform bool u_highlight;
uniform float u_highlightHue;
uniform vec3 u_dimmed;
uniform float u_ring;
uniform float u_paletteSize;
uniform vec2 u_world;
uniform float u_scale;
uniform vec2 u_origin;
uniform vec4 u_window;
uniform vec2 u_fadeAxes;
uniform float u_fade;
uniform float u_gap;
uniform float u_radius;
uniform float u_canvasHeight;
uniform vec3 u_background;

out vec4 colour;

float edgeFade(float start, float low, float size) {
  const float slack = 0.001;
  if (start < low - slack || start + u_scale > low + size + slack) return 0.0;
  float centre = start + u_scale * 0.5;
  float distance = min(centre - low, low + size - centre);
  return smoothstep(0.0, 1.0, min(1.0, distance / u_fade));
}

void main() {
  vec2 pixel = vec2(gl_FragCoord.x, u_canvasHeight - gl_FragCoord.y);
  colour = vec4(u_background, 1.0);
  if (pixel.x < u_window.x || pixel.y < u_window.y || pixel.x >= u_window.x + u_window.z || pixel.y >= u_window.y + u_window.w) return;
  vec2 cell = floor((pixel - u_origin) / u_scale);
  vec2 start = u_origin + cell * u_scale;
  float alpha = 1.0;
  if (u_fadeAxes.x > 0.5) alpha *= edgeFade(start.x, u_window.x, u_window.z);
  if (u_fadeAxes.y > 0.5) alpha *= edgeFade(start.y, u_window.y, u_window.w);
  if (alpha <= 0.02) return;
  ivec2 texel = ivec2(mod(cell, u_world));
  float hue = texelFetch(u_cells, texel, 0).r;
  if (hue < 0.0) return;
  uint mark = u_highlight ? texelFetch(u_carriers, texel, 0).r : ${CARRYING.shows}u;
  float coverage = 1.0;
  if (u_gap > 0.0) {
    float half_side = (u_scale - u_gap) * 0.5;
    vec2 offset = abs(pixel - start - half_side) - (half_side - u_radius);
    float edge = length(max(offset, 0.0)) + min(max(offset.x, offset.y), 0.0) - u_radius;
    coverage = clamp(0.5 - edge, 0.0, 1.0);
    if (mark == ${CARRYING.hides}u) coverage *= clamp(0.5 + edge + u_ring, 0.0, 1.0);
  }
  vec3 shade = mark == ${CARRYING.none}u ? u_dimmed : texture(u_palette, vec2((mark == ${CARRYING.hides}u ? u_highlightHue : hue) / 360.0 + 0.5 / u_paletteSize, 0.5)).rgb;
  colour = vec4(mix(u_background, shade, coverage * alpha), 1.0);
}
`;

const UNIFORMS = [
  'u_cells',
  'u_palette',
  'u_carriers',
  'u_highlight',
  'u_highlightHue',
  'u_dimmed',
  'u_ring',
  'u_paletteSize',
  'u_world',
  'u_scale',
  'u_origin',
  'u_window',
  'u_fadeAxes',
  'u_fade',
  'u_gap',
  'u_radius',
  'u_canvasHeight',
  'u_background',
] as const;

type Uniform = (typeof UNIFORMS)[number];

/**
 * Draws the World with WebGL2: one copy of the torus inside a window whose
 * edges fade into the background, each live Cell a rounded square in its
 * Phenotype's palette colour. Under a Carrier highlight, Cells showing the
 * Allele keep their colour, Carriers hiding it are rings in its colour, and
 * every other live Cell is dimmed.
 */
export class Renderer {
  private readonly gl: WebGL2RenderingContext;
  private readonly canvas: HTMLCanvasElement;
  private readonly uniforms: Record<Uniform, WebGLUniformLocation | null>;
  private readonly cells: WebGLTexture;
  private readonly carriers: WebGLTexture;
  private world: Size = { width: 0, height: 0 };
  /** The size of the carriers texture, which is only allocated once a highlight needs it. */
  private carriersSize: Size = { width: 0, height: 0 };

  /** A renderer for `canvas`, or `null` where WebGL2 is unavailable or cannot run the shaders. */
  static create(canvas: HTMLCanvasElement): Renderer | null {
    const gl = canvas.getContext('webgl2', { antialias: false, alpha: false });
    if (!gl) return null;
    try {
      return new Renderer(canvas, gl);
    } catch (error) {
      console.error(error);
      return null;
    }
  }

  private constructor(canvas: HTMLCanvasElement, gl: WebGL2RenderingContext) {
    this.canvas = canvas;
    this.gl = gl;
    const program = link(gl, compile(gl, gl.VERTEX_SHADER, VERTEX), compile(gl, gl.FRAGMENT_SHADER, FRAGMENT));
    gl.useProgram(program);
    gl.bindVertexArray(gl.createVertexArray());
    this.uniforms = Object.fromEntries(UNIFORMS.map((name) => [name, gl.getUniformLocation(program, name)])) as Record<
      Uniform,
      WebGLUniformLocation | null
    >;
    this.cells = texture(gl, 0, gl.NEAREST, gl.CLAMP_TO_EDGE);
    texture(gl, 1, gl.LINEAR, gl.REPEAT);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, PALETTE_SIZE, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, paletteTable(PALETTE_SIZE));
    this.carriers = texture(gl, 2, gl.NEAREST, gl.CLAMP_TO_EDGE);
    gl.uniform1i(this.uniforms.u_cells, 0);
    gl.uniform1i(this.uniforms.u_palette, 1);
    gl.uniform1i(this.uniforms.u_carriers, 2);
    gl.uniform3f(this.uniforms.u_dimmed, ...DIMMED);
    gl.uniform1f(this.uniforms.u_paletteSize, PALETTE_SIZE);
    gl.uniform3f(this.uniforms.u_background, ...BACKGROUND);
  }

  /** Replaces the drawn Cells with `cells`. */
  upload(cells: Cells): void {
    const { gl } = this;
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.cells);
    if (cells.width !== this.world.width || cells.height !== this.world.height) {
      this.world = { width: cells.width, height: cells.height };
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.R32F, cells.width, cells.height, 0, gl.RED, gl.FLOAT, cells.phenotypes);
    } else {
      gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, cells.width, cells.height, gl.RED, gl.FLOAT, cells.phenotypes);
    }
    gl.uniform1i(this.uniforms.u_highlight, cells.highlight ? 1 : 0);
    if (!cells.highlight) return;
    gl.uniform1f(this.uniforms.u_highlightHue, cells.highlight.hue);
    gl.activeTexture(gl.TEXTURE2);
    gl.bindTexture(gl.TEXTURE_2D, this.carriers);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    if (cells.width !== this.carriersSize.width || cells.height !== this.carriersSize.height) {
      this.carriersSize = { width: cells.width, height: cells.height };
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8UI, cells.width, cells.height, 0, gl.RED_INTEGER, gl.UNSIGNED_BYTE, cells.highlight.carriers);
    } else {
      gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, cells.width, cells.height, gl.RED_INTEGER, gl.UNSIGNED_BYTE, cells.highlight.carriers);
    }
  }

  /** Draws the last uploaded Cells as seen through `view`. */
  draw({ camera, viewport, pixelRatio }: View): void {
    const { gl, canvas, uniforms: u } = this;
    const width = Math.max(1, Math.round(viewport.width * pixelRatio));
    const height = Math.max(1, Math.round(viewport.height * pixelRatio));
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }
    gl.viewport(0, 0, width, height);
    if (this.world.width === 0) {
      gl.clearColor(...BACKGROUND, 1);
      gl.clear(gl.COLOR_BUFFER_BIT);
      return;
    }
    const window = worldWindow(camera, this.world, viewport);
    const gap = camera.scale >= GAP_FROM ? Math.max(1, GAP_FRACTION * camera.scale) : 0;
    const radius = camera.scale >= ROUNDED_FROM ? RADIUS_FRACTION * (camera.scale - gap) : 0;
    gl.uniform2f(u.u_world, this.world.width, this.world.height);
    gl.uniform1f(u.u_scale, camera.scale * pixelRatio);
    gl.uniform2f(u.u_origin, camera.x * pixelRatio, camera.y * pixelRatio);
    gl.uniform4f(u.u_window, window.x * pixelRatio, window.y * pixelRatio, window.width * pixelRatio, window.height * pixelRatio);
    gl.uniform2f(u.u_fadeAxes, window.width < viewport.width ? 1 : 0, window.height < viewport.height ? 1 : 0);
    gl.uniform1f(u.u_fade, FADE_PX * pixelRatio);
    gl.uniform1f(u.u_gap, gap * pixelRatio);
    gl.uniform1f(u.u_radius, radius * pixelRatio);
    gl.uniform1f(u.u_ring, Math.max(1, RING_FRACTION * (camera.scale - gap) * pixelRatio));
    gl.uniform1f(u.u_canvasHeight, height);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }
}

function compile(gl: WebGL2RenderingContext, type: number, source: string): WebGLShader {
  const shader = gl.createShader(type);
  if (!shader) throw new Error('could not create a shader');
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(`shader did not compile: ${gl.getShaderInfoLog(shader) ?? ''}`);
  }
  return shader;
}

function link(gl: WebGL2RenderingContext, vertex: WebGLShader, fragment: WebGLShader): WebGLProgram {
  const program = gl.createProgram();
  gl.attachShader(program, vertex);
  gl.attachShader(program, fragment);
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(`shader program did not link: ${gl.getProgramInfoLog(program) ?? ''}`);
  }
  return program;
}

/** A new texture bound to `unit`, with the given filter and horizontal wrap. */
function texture(gl: WebGL2RenderingContext, unit: number, filter: number, wrap: number): WebGLTexture {
  const created = gl.createTexture();
  gl.activeTexture(gl.TEXTURE0 + unit);
  gl.bindTexture(gl.TEXTURE_2D, created);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, wrap);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  return created;
}
