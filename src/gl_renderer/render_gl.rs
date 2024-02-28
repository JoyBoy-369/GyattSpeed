use std::{
    ffi::{c_void, CString},
    mem, ptr,
};

use gl::types::{GLint, GLsizeiptr, GLuint, GLvoid};

use crate::{
    gl_assert_ok,
    utils::{compile_shader, link_programs, ortho, Res, Vertex},
};

pub struct GlGlyphTexture {
    pub name: GLuint,
}

impl GlGlyphTexture {
    pub fn new((width, height): (u32, u32)) -> Self {
        let mut name = 0;
        unsafe {
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::GenTextures(1, &mut name);
            gl::BindTexture(gl::TEXTURE_2D, name);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RED as i32,
                width as i32,
                height as i32,
                0,
                gl::RED,
                gl::UNSIGNED_BYTE,
                ptr::null(),
            );

            gl_assert_ok!();

            Self { name }
        }
    }

    pub fn clear(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.name);
            gl::ClearTexImage(
                self.name,
                0,
                gl::RED,
                gl::UNSIGNED_BYTE,
                [12_u8].as_ptr() as *const c_void,
            );

            gl_assert_ok!();
        }
    }
}

impl Drop for GlGlyphTexture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.name);
        }
    }
}

pub struct GLTextPipe {
    shaders: [GLuint; 2],
    program: GLuint,
    vao: GLuint,
    vbo: GLuint,
    transform_uniform: GLint,
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

        let transform_uniform = unsafe {
            // create and bind vao
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            // create and bind vbo
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            //use shader program
            gl::UseProgram(program);
            gl::BindFragDataLocation(program, 0, CString::new("out_color")?.as_ptr());

            //get location of transform uniform variable in the program
            let uniform = gl::GetUniformLocation(program, CString::new("transform")?.as_ptr());
            if uniform < 0 {
                return Err(format!("GetUniformLocation(\"transform\") -> {uniform}").into());
            }

            //transform the vertices
            let transform = ortho(0.0, w, 0.0, h, 1.0, -1.0);
            gl::UniformMatrix4fv(uniform, 1, 0, transform.as_ptr());

            let mut offset = 0;
            // iterate over vetex attributes
            // get attrib location for each attrib
            // set attrib pointer
            // enable attrib and set attrib divisor
            for (v_field, float_count) in &[
                ("left_top", 3),
                ("right_bottom", 2),
                ("tex_left_top", 2),
                ("tex_right_bottom", 2),
                ("color", 4),
            ] {
                let attr = gl::GetAttribLocation(program, CString::new(*v_field)?.as_ptr());
                if attr < 0 {
                    return Err(format!("{v_field} GetAttribLocation -> {attr}").into());
                }

                gl::VertexAttribPointer(
                    attr as u32,
                    *float_count,
                    gl::FLOAT,
                    gl::FALSE,
                    mem::size_of::<Vertex>() as i32,
                    offset as *const c_void,
                );
                gl::EnableVertexAttribArray(attr as u32);
                gl::VertexAttribDivisor(attr as u32, 1);

                offset += float_count * 4;
            }

            //enable alpha blending
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE);

            //use srgb
            gl::Enable(gl::FRAMEBUFFER_SRGB);
            gl::ClearColor(0.02, 0.02, 0.02, 1.0);
            gl_assert_ok!();

            uniform
        };

        Ok(Self {
            shaders: [vs, fs],
            program,
            vao,
            vbo,
            transform_uniform,
            vertex_count: 0,
            vertex_buffer_len: 0,
        })
    }

    // update vertex data
    pub fn upload_vertices(&mut self, vertices: &[Vertex]) {
        self.vertex_count = vertices.len();

        unsafe {
            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);

            // resize buffer or update buffer
            if self.vertex_buffer_len < self.vertex_count {
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (self.vertex_count * mem::size_of::<Vertex>()) as GLsizeiptr,
                    vertices.as_ptr() as *const GLvoid,
                    gl::DYNAMIC_DRAW,
                );
                self.vertex_buffer_len = self.vertex_count;
            } else {
                gl::BufferSubData(
                    gl::ARRAY_BUFFER,
                    0,
                    (self.vertex_count * mem::size_of::<Vertex>()) as GLsizeiptr,
                    vertices.as_ptr() as *const GLvoid,
                );
            }

            gl_assert_ok!();
        }
    }

    // update transformation based on window size
    pub fn update_geometry(&self, window_size: winit::dpi::PhysicalSize<u32>) {
        let (w, h) = (window_size.width as f32, window_size.height as f32);
        let transform = ortho(0.0, w, 0.0, h, 1.0, -1.0);

        unsafe {
            gl::UseProgram(self.program);
            gl::UniformMatrix4fv(self.transform_uniform, 1, 0, transform.as_ptr());
            gl_assert_ok!();
        }
    }

    // draw text
    pub fn draw(&self) {
        unsafe {
            gl::UseProgram(self.program);
            gl::BindVertexArray(self.vao);
            gl::DrawArraysInstanced(gl::TRIANGLE_STRIP, 0, 4, self.vertex_count as i32);
            gl_assert_ok!();
        }
    }
}

impl Drop for GLTextPipe {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program);
            self.shaders.iter().for_each(|s| gl::DeleteShader(*s));
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
