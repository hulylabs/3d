#[cfg(test)]
pub(crate) mod tests {
    use serde::Serialize;
    use serde_json::Value;
    use std::cell::RefCell;
    use std::fmt::Write;
    use tinytemplate::{format_unescaped, TinyTemplate};

    const ENTRY_TEMPLATE_TEXT: &str = include_str!("_function_execution_template.wgsl");
    const ENTRY_TEMPLATE_KEY: &str = "entry";

    #[must_use]
    fn load_template() -> TinyTemplate<'static> {
        let mut collection = TinyTemplate::new();
        collection.set_default_formatter(&format_unescaped);
        collection.add_template(ENTRY_TEMPLATE_KEY, ENTRY_TEMPLATE_TEXT).expect("entry template parsing failed");
        collection
    }

    thread_local! {
        static TEMPLATE: RefCell<TinyTemplate<'static>> = RefCell::new(load_template());
    }

    macro_rules! create_argument_formatter {
        ($argument_expression:expr) => {
            |data: &serde_json::Value, buffer: &mut String| -> tinytemplate::error::Result<()> {
                if let Some(s) = data.as_str() {
                    write!(buffer, $argument_expression, argument = s)?;
                } else {
                    write!(buffer, $argument_expression, argument = data)?;
                }
                Ok(())
            }
        };
    }

    pub(crate) use create_argument_formatter;

    #[must_use]
    pub(crate) fn make_executable<ArgumentFormatter>(data: &ShaderFunction, argument_formatter: ArgumentFormatter) -> String 
        where ArgumentFormatter : 'static + Fn(&Value, &mut String) -> tinytemplate::error::Result<()> 
    {
        let mut result: Option<String> = None;
        TEMPLATE.with(|template| {
            let mut template = template.borrow_mut();
            template.add_formatter("argument_deconstructor", argument_formatter);
            
            let instance = template.render(ENTRY_TEMPLATE_KEY, data);
            result = Some(instance.expect("shader entry template parsing failed"));
        });
        result.unwrap()
    }

    #[derive(Serialize)]
    struct Field {
        name: String,
        r#type: String,
    }

    impl Field {
        #[must_use]
        fn new(name: impl Into<String>, r#type: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                r#type: r#type.into(),
            }
        }
    }

    #[derive(Serialize)]
    pub(crate) struct TypeDeclaration {
        name: String,
        fields: Vec<Field>,
    }

    impl TypeDeclaration {
        #[must_use]
        pub(crate) fn new(name: impl Into<String>, field_name: impl Into<String>, field_type: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                fields: vec![Field::new(field_name, field_type)],
            }
        }

        pub(crate) fn with_field(mut self, name: impl Into<String>, r#type: impl Into<String>) -> Self {
            self.add_field(name, r#type);
            self
        }

        pub(crate) fn add_field(&mut self, name: impl Into<String>, r#type: impl Into<String>) {
            self.fields.push(Field::new(name, r#type));
        }
    }
    
    #[derive(Serialize)]
    pub(crate) struct ShaderFunction
    {
        input_type: String,
        output_type: String,
        function_name: String,
        argument: &'static str,
        custom_types: Vec<TypeDeclaration>,
        additional_code: Vec<String>,
        binding_group: u32,
    }

    impl ShaderFunction {
        #[must_use]
        pub(crate) fn new(
            input_type: impl Into<String>,
            output_type: impl Into<String>,
            function_name: impl Into<String>,
        ) -> Self {
            Self {
                input_type: input_type.into(),
                output_type: output_type.into(),
                function_name: function_name.into(),
                argument: "argument",
                custom_types: Vec::new(),
                additional_code: Vec::new(),
                binding_group: 0,
            }
        }

        pub(crate) fn with_custom_type(mut self, type_declaration: TypeDeclaration) -> Self {
            self.add_custom_type(type_declaration);
            self
        }

        pub(crate) fn with_additional_shader_code(mut self, code: impl Into<String>) -> Self {
            self.add_additional_shader_code(code);
            self
        }

        pub(crate) fn with_binding_group(mut self, binding_group: u32) -> Self {
            self.binding_group = binding_group;
            self
        }

        pub(crate) fn add_custom_type(&mut self, type_declaration: TypeDeclaration) {
            self.custom_types.push(type_declaration);
        }

        pub(crate) fn add_additional_shader_code(&mut self, code: impl Into<String>) {
            self.additional_code.push(code.into());
        }
    }
    
    #[test]
    fn test_complex_template() {
        let vertex_type 
            = TypeDeclaration::new("Vertex", "position", "vec3<f32>");
        
        let material_type 
            = TypeDeclaration::new("Material", "diffuse", "vec3<f32>")
                .with_field("shininess", "f32");
    
        let template = ShaderFunction::new("Vertex", "vec4<f32>", "shade_vertex")
            .with_custom_type(vertex_type)
            .with_custom_type(material_type)
            .with_binding_group(3)
            .with_additional_shader_code("SOME ADDITIONAL CODE 1")
            .with_additional_shader_code("SOME ADDITIONAL CODE 2");

        let result = make_executable(&template, create_argument_formatter!("{argument}.position.xyz"));
        
        assert_eq!(result, REFERENCE_SHADER_CODE);
    }

    const REFERENCE_SHADER_CODE: &str =
r#"@group(3) @binding( 0) var<storage, read> input: array<Vertex>;
@group(3) @binding( 1) var<storage, read_write> output: array<vec4<f32>>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&input)) {
        return;
    }
    let argument = input[index];
    output[index] = shade_vertex(argument.position.xyz);
}

struct Vertex {
     position: vec3<f32>, 
}

struct Material {
     diffuse: vec3<f32>,  shininess: f32, 
}


SOME ADDITIONAL CODE 1

SOME ADDITIONAL CODE 2
"#;
    
}