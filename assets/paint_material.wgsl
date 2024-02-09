#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    let brightness = 10.0;
    let color = vec3(1.0, 1.0, 0.0);

    var pbr_input = pbr_input_from_standard_material(in, is_front);

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color.a *= max(max(out.color.r, out.color.g), out.color.b);
    out.color.r = mix(out.color.r, color.r, 0.5) * brightness;
    out.color.g = mix(out.color.g, color.g, 0.5) * brightness;
    out.color.b = mix(out.color.b, color.b, 0.5) * brightness;
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    // out.color.r = pbr_input.material.base_color.r;
    // out.color.g = pbr_input.material.base_color.g;
    // out.color.b = pbr_input.material.base_color.b;

    return out;
}