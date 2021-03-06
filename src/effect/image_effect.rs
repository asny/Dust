use crate::math::*;
use crate::core::*;

///
/// A customizable 2D effect.
/// Can for example be used for adding an effect on top of the rendered 3D scene, like [fog](crate::FogEffect).
///
pub struct ImageEffect {
    program: Program,
    positions: VertexBuffer,
    uvs: VertexBuffer
}

impl ImageEffect {
    pub fn new(context: &Context, fragment_shader: &str) -> Result<Self, Error>
    {
        let program = program::Program::from_source(&context,
                                                    "in vec3 position;
                                                    in vec2 uv_coordinate;
                                                    out vec2 uv;
                                                    void main()
                                                    {
                                                        uv = uv_coordinate;
                                                        gl_Position = vec4(position, 1.0);
                                                    }",
                                                    fragment_shader)?;

        let positions = vec![
            -3.0, -1.0, 0.0,
            3.0, -1.0, 0.0,
            0.0, 2.0, 0.0
        ];
        let uvs = vec![
            -1.0, 0.0,
            2.0, 0.0,
            0.5, 1.5
        ];
        let positions = VertexBuffer::new_with_static_f32(&context, &positions).unwrap();
        let uvs = VertexBuffer::new_with_static_f32(&context, &uvs).unwrap();

        Ok(Self {program, positions, uvs})
    }

    pub fn program(&self) -> &Program {
        &self.program
    }

    pub fn apply(&self, render_states: RenderStates, viewport: Viewport) -> Result<(), Error> {
        self.program.use_attribute_vec3(&self.positions, "position")?;
        self.program.use_attribute_vec2(&self.uvs, "uv_coordinate")?;
        self.program.draw_arrays(render_states, viewport,3);
        Ok(())
    }
}