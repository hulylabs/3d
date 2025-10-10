#language slang 2026

import output_on_screen_pixel;
import glyph_sdf;
import glyphs_storage;

namespace tests_for_pixel_global_index {
    func test_pixel_global_index() -> int32_t {
        let actual_index = pixel_global_index(float2(1.2, 3.4), 800);
        let expected_index = 3 * 800 + 1;

        if (actual_index != expected_index) {
            printf("Test failed: actual_index = %d, expected_index = %d\n", actual_index, expected_index);
            return 1;
        }
        return 0;
    }
}

namespace tests_for_median {
    func test_median_evaluation(x: float, y: float, z: float, expected: float) -> int32_t {
        let actual = median(x, y, z);
        if (expected != actual) {
            printf("Test failed: actual median(%f, %f, %f) = %f, expected %f\n", x, y, z, actual, expected);
            return 1;
        }
        return 0;
    }

    func test_median() -> int32_t {
        var result: int32_t = 0;

        result += test_median_evaluation(5.0, 2.0, 8.0, 5.0);
        result += test_median_evaluation(2.0, 8.0, 5.0, 5.0);
        result += test_median_evaluation(8.0, 2.0, 5.0, 5.0);
        
        result += test_median_evaluation(1.0, 3.0, 7.0, 3.0);
        result += test_median_evaluation(7.0, 1.0, 3.0, 3.0);
        result += test_median_evaluation(3.0, 7.0, 1.0, 3.0);

        return result;
    }
}

namespace tests_for_glyph_location {
    func check_uv_evaluation(system_under_test: GlyphLocation, position: float2, expected: float2) -> int32_t {
        let actual_uv = system_under_test.evaluate_uv(position);
        if (any(actual_uv != expected)) {
            printf("Test failed (tests_for_glyph_location::evaluate_uv): actual = (%f, %f), expected (%f, %f)\n", actual_uv.x, actual_uv.y, expected.x, expected.y);
            return 1;
        }
        return 0;
    }
    func test_evaluate_uv() -> int32_t {
        let system_under_test = GlyphLocation(float2(0.2, 0.1), float2(0.1, 0.3));
        var result: int32_t = 0;
        
        result += check_uv_evaluation(system_under_test, float2(0.0, 0.0), float2(0.25, 0.25));
        result += check_uv_evaluation(system_under_test, float2(0.05, -0.15), float2(0.3, 0.4));
        result += check_uv_evaluation(system_under_test, float2(-0.05, 0.15), float2(0.2, 0.1));

        result += check_uv_evaluation(system_under_test, float2(-0.15, 0.15), float2(0.2, 0.1));
        result += check_uv_evaluation(system_under_test, float2(-0.05, 0.25), float2(0.2, 0.1));

        result += check_uv_evaluation(system_under_test, float2(0.15, -0.15), float2(0.3, 0.4));
        result += check_uv_evaluation(system_under_test, float2(0.05, -0.25), float2(0.3, 0.4));

        return result;
    }
}

namespace tests_for_sdf_glyph {
    
    struct MockGlyphStorage : IGlyphsStorage {
        static func texture_to_world_scale() -> float {
            return 3.0;
        }

        static func locate_glyph(glyph_index: uint) -> GlyphLocation {
            if (glyph_index == 0) {
                return GlyphLocation(float2(0.1, 0.1), float2(0.1, 0.3));
            }
            if (glyph_index == 1) {
                return GlyphLocation(float2(0.6, 0.5), float2(0.2, 0.1));
            }
            return GlyphLocation(float2(0.2, 0.4), float2(0.4, 0.1));
        }

        static func sample(float2 uv) -> float3 {
            return float3(uv.x, uv.y, 0.5);
        }
    }

    func assert_glyph_sdf_value(actual_sdf_value: float, expected_sdf_value: float) -> int32_t {
        if (abs(actual_sdf_value - expected_sdf_value) > 1e-6) {
            printf("Test failed: test_sdf_glyph actual value = %f, expected %f\n", actual_sdf_value, expected_sdf_value);
            return 1;
        }
        return 0;
    }

    func test_sdf_glyph() -> int32_t {
        var result: int32_t = 0;

        result += assert_glyph_sdf_value(sdf_glyph_xy<MockGlyphStorage>(0, float2(-0.05, 0.15)), 1.2);
        result += assert_glyph_sdf_value(sdf_glyph_xy<MockGlyphStorage>(1, float2(-0.1, 0.05)), 0.0);
        result += assert_glyph_sdf_value(sdf_glyph_xy<MockGlyphStorage>(0, float2(0.0)), 0.75);
        result += assert_glyph_sdf_value(sdf_glyph_xy<MockGlyphStorage>(1, float2(0.0)), -0.15);
        result += assert_glyph_sdf_value(sdf_glyph_xy<MockGlyphStorage>(0, float2(1.0)), 0.9);
        result += assert_glyph_sdf_value(sdf_glyph_xy<MockGlyphStorage>(1, float2(1.0)), 0.0);
        
        return result;
    }
}

export __extern_cpp int main(int argc, NativeString* argv) {
    var result: int32_t = 0;
    
    let test_pixel_global_index_result = tests_for_pixel_global_index::test_pixel_global_index();
    result += test_pixel_global_index_result;
    if (test_pixel_global_index_result == 0) {
        printf("OK: test_pixel_global_index\n");
    }

    let test_median_result = tests_for_median::test_median();
    result += test_median_result;
    if (test_median_result == 0) {
        printf("OK: test_median\n");
    }

    let test_sdf_glyph_result = tests_for_sdf_glyph::test_sdf_glyph();
    result += test_sdf_glyph_result;
    if (test_sdf_glyph_result == 0) {
        printf("OK: test_sdf_glyph\n");
    }
    
    let test_glyph_location_evaluate_uv = tests_for_glyph_location::test_evaluate_uv();
    result += test_glyph_location_evaluate_uv;
    if (test_glyph_location_evaluate_uv == 0) {
        printf("OK: test_glyph_location::evaluate_uv\n");
    }

    return result;
}
