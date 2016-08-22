use super::Vertex;

use gl_raii::{GLVertexBuffer, GLVertex, BufferUsage,
              VertexAttribData, GLSLType, GLPrim,
              GLVertexArray};

pub struct BufferData {
    verts: GLVertexBuffer<Vertex>,
    verts_vao: GLVertexArray<Vertex>
}

impl BufferData {
    pub fn new() -> BufferData {
        let verts = GLVertexBuffer::new(0, BufferUsage::Static);
        let verts_vao = GLVertexArray::new(&verts);
        BufferData {
            verts: verts,
            verts_vao: verts_vao
        }
    }
}

unsafe impl GLVertex for Vertex {
    unsafe fn vertex_attrib_data() -> &'static [VertexAttribData] {
        const VAD: &'static [VertexAttribData] = &[
            // Relative point
            VertexAttribData {
                index: 0,
                glsl_type: GLSLType::Vec2(GLPrim::Float),
                offset: 0
            },

            // Absolute point
            VertexAttribData {
                index: 1,
                glsl_type: GLSLType::Vec2(GLPrim::Float),
                offset: 8
            },

            // Normal
            VertexAttribData {
                index: 2,
                glsl_type: GLSLType::Vec2(GLPrim::Float),
                offset: 16
            },

            // Color
            VertexAttribData {
                index: 3,
                glsl_type: GLSLType::Vec4(GLPrim::NByte),
                offset: 24
            }
        ];

        VAD
    }
}
