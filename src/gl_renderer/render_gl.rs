use gl::types::GLuint;

use crate::utils::{compile_shader, link_programs, Res};

pub struct GLTextPipe {
    shaders: [GLuint; 2],
    program: GLuint,
    vao: GLuint,
    vbo: GLuint,
    transform_uniform: GLuint,
    vertex_count: usize,
    vertex_buffer_len: usize,
}

impl GLTextPipe {
    pub fn new(window_size: winit::dpi::PhysicalSize<u32>) -> Res<Self> {
        let (w, h) = (window_size.width as f32, window_size.height as f32);

        let fs = compile_shader(include_str!("shaders/text.fs"), gl::FRAGMENT_SHADER)?;
        let vs = compile_shader(include_str!("shaders/text.vs"), gl::VERTEX_SHADER)?;
        let program = link_programs(vs, fs)?;

        let mut vao = 0;
        let mut vbo = 0;
    }
}
