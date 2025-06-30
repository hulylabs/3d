#[cfg(test)]
pub(crate) mod tests {
    use std::rc::Rc;
    use crate::geometry::aabb::Aabb;
    use crate::sdf::framework::sdf_base::Sdf;
    use crate::sdf::framework::stack::Stack;
    use crate::shader::code::{FunctionBody, ShaderCode};

    pub(crate) struct DummySdf {
        payload: String,
    }

    impl DummySdf {
        #[must_use]
        pub(crate) fn new(return_value: &str) -> Self {
            Self {
                payload: format!("return {value};", value = return_value),
            }
        }
    }

    impl Default for DummySdf {
        #[must_use]
        fn default() -> Self {
            Self::new("value")
        }
    }
    
    impl Sdf for DummySdf {
        #[must_use]
        fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
            ShaderCode::<FunctionBody>::new(self.payload.clone())
        }

        #[must_use]
        fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
            vec![]
        }

        #[must_use]
        fn aabb(&self) -> Aabb {
            Aabb::make_null()
        }
    }
    
    #[must_use]
    pub(crate) fn make_dummy_sdf() -> Rc<dyn Sdf> {
        Rc::new(DummySdf::default())
    }
}