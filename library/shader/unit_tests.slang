import "output_on_screen_pixel";

export __extern_cpp int main(int argc, NativeString* argv)
{
    let actual_index = pixel_global_index(float2(1.2, 3.4), 800);
    let expected_index = 3 * 800 + 1;

    if (actual_index != expected_index) {
        printf("Test failed: actual_index = %d, expected_index = %d\n", actual_index, expected_index);
        return 1;
    }

    return 0;
}
