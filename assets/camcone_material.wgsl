#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}

struct CamConeMaterial {
    amount: f32,
};

@group(1) @binding(100)
var<uniform> material: CamConeMaterial;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var color_1 = vec3(0.0, 1.0, 1.0);
    var color_2 = vec3(1.0, 1.0, 0.0);
    var color_3 = vec3(1.0, 0.0, 0.0);
    var color = mix(color_1, color_2, clamp(material.amount * 3.0, 0.0, 1.0));
    color = mix(color, color_3, clamp(material.amount * 3.0 - 1.0, 0.0, 2.0) / 2.0);

    var pbr_input = pbr_input_from_standard_material(in, is_front);

    var out: FragmentOutput;
    out.color = pbr_input.material.base_color;
    out.color = vec4<f32>(color, (0.05 + material.amount * 0.15) * out.color.a);

    return out;
}