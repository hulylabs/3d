#[cfg(test)]
mod tests {
    use crate::serialization::pod_vector::PodVector;
    use crate::tests::gpu_code_execution::tests::execute_code;

    #[test]
    fn execute_some_gpu_code() {
        let input_points = [
            PodVector { x:  1.0 , y:  2.0 , z:   3.0 , w: 4.0 },
        ];
        let result = execute_code(&input_points, GPU_CODE);
        
        assert_eq!(result[0], input_points[0].y);
    }
    
    const GPU_CODE: &str = include_str!("wgsl_sandbox.wgsl");
}