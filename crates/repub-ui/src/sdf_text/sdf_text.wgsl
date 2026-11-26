// Distance-field text for the Repub canvas.
//
// The atlas is an MTSDF: RGB hold three signed distances to differently
// coloured edge sets, whose median reconstructs the outline with sharp
// corners; alpha holds the true signed distance. All are 0.5 on the outline,
// larger inside, encoding +-SPREAD atlas texels. The fragment shader converts
// distance to *screen pixels* using the screen-space derivatives of the atlas
// coordinates, then covers exactly one pixel of antialiasing ramp centered on
// the outline. This stays sharp at any zoom, with no per-size rasterization.
//
// Output matches egui-wgpu: premultiplied alpha in gamma (sRGB-encoded) space,
// written to a non-sRGB framebuffer.

struct VertexInput {
    // Normalized device coordinates within the callback viewport
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    // Straight-alpha sRGB color
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct Params {
    // Distance range on each side of the outline, in atlas texels
    spread: f32,
    // Outline offset in screen pixels; > 0 thickens (used to compensate thin
    // dark-on-light text in gamma-space blending)
    dilate_px: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var atlas_texture: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;
@group(0) @binding(2) var<uniform> params: Params;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

fn median(r: f32, g: f32, b: f32) -> f32 {
    return max(min(r, g), min(max(r, g), b));
}

// Coverage of one sample: an antialiasing ramp `ramp_px` screen pixels wide,
// centered on the outline.
fn sample_coverage(uv: vec2<f32>, texels_per_pixel: f32, ramp_px: f32) -> f32 {
    let texel = textureSampleLevel(atlas_texture, atlas_sampler, uv, 0.0);
    // Sharp corners come from the multi-channel median. Once a screen pixel
    // spans several texels, corners are sub-pixel anyway and interpolating
    // the three channels across neighbouring edges can misplace the median, so
    // fade over to the true distance in alpha.
    let true_weight = smoothstep(2.0, 6.0, texels_per_pixel);
    let value = mix(median(texel.r, texel.g, texel.b), texel.a, true_weight);
    // Signed distance to the outline in atlas texels (positive inside)
    let distance_texels = (value - 0.5) * 2.0 * params.spread;
    let distance_px = distance_texels / texels_per_pixel + params.dilate_px;
    return clamp(distance_px / ramp_px + 0.5, 0.0, 1.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Screen-space AA: how many atlas texels one screen pixel spans, from the
    // derivatives of the texel coordinate along screen x and y.
    let size = vec2<f32>(textureDimensions(atlas_texture));
    let duv_dx = dpdx(in.uv);
    let duv_dy = dpdy(in.uv);
    let dx = duv_dx * size;
    let dy = duv_dy * size;
    let texels_per_pixel = max(sqrt(0.5 * (dot(dx, dx) + dot(dy, dy))), 1e-5);

    var coverage: f32;
    if texels_per_pixel <= 1.0 {
        // Magnified or 1:1: one sample is exact
        coverage = sample_coverage(in.uv, texels_per_pixel, 1.0);
    } else {
        // Minified (small text, page thumbnails): one pixel spans many texels and
        // strokes thinner than a pixel would be hit or missed by a single sample,
        // so text breaks up and shimmers. Average four samples on a rotated grid,
        // each covering a quarter of the pixel with a half-pixel ramp.
        let a = duv_dx * 0.125 + duv_dy * 0.375;
        let b = duv_dx * 0.375 - duv_dy * 0.125;
        coverage = 0.25 * (
            sample_coverage(in.uv + a, texels_per_pixel, 0.5)
            + sample_coverage(in.uv - a, texels_per_pixel, 0.5)
            + sample_coverage(in.uv + b, texels_per_pixel, 0.5)
            + sample_coverage(in.uv - b, texels_per_pixel, 0.5)
        );
    }

    let alpha = in.color.a * coverage;
    return vec4<f32>(in.color.rgb * alpha, alpha);
}
