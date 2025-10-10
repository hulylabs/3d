#language slang 2026

module signatures;

/*

Body of the functions below is generated at runtime.

*/

// >>> BEGIN: DUMMY STUBS
public func procedural_texture_select(index: int32_t, position: float3, normal: float3, time: float, dp_dx: float3, dp_dy: float3) -> float3 {
    __intrinsic_asm "procedural_texture_select";
}

public func sdf_select(int32_t index, float3 position, float time, uint32_t payload) -> float {
    __intrinsic_asm "sdf_select";
}

public func sdf_apply_animation(index: int32_t, position: float3, time: float) -> float3 {
    __intrinsic_asm "sdf_apply_animation";
}
// >>> END: DUMMY STUBS