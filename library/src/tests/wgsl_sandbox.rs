#[cfg(test)]
mod tests {
    use test_context::test_context;
    use crate::serialization::pod_vector::PodVector;
    use crate::tests::scaffolding::gpu_code_execution::tests::{ExecutionConfig, GpuCodeExecutionContext};

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn execute_some_gpu_code(fixture: &mut GpuCodeExecutionContext) {
        let input_points = [
            PodVector { x:  1.0 , y:  2.0 , z:   3.0 , w: 4.0 },
            PodVector { x:  5.0 , y:  6.0 , z:   7.0 , w: 8.0 },
        ];
        let result = fixture.get().execute_code::<PodVector, f32>(&input_points, GPU_CODE.to_string(), ExecutionConfig::default());
        
        assert_eq!(result[0], input_points[0].y);
    }
    
    const GPU_CODE: &str = include_str!("wgsl_sandbox.wgsl");
}