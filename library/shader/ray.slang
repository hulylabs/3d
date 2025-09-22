#language slang 2026

module signatures;

public struct Ray {
    public float3 origin;
    public float3 direction;

    public __init(float3 origin, float3 direction) {
        this.origin = origin;
        this.direction = direction;
    }

    public func at(t: float) -> float3 {
        return origin + t * direction;
    }
};