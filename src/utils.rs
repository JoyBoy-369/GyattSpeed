use std::{ffi::CString, ptr, str};

use gl::types::{GLchar, GLenum, GLfloat, GLint, GLuint};

pub type Res<T> = Result<T, Box<dyn std::error::Error>>;

pub type Vertex = [GLfloat; 13];

pub fn gl_err_to_str(err: u32) -> &'static str {
    match err {
        gl::INVALID_ENUM => "INVALID_ENUM",
        gl::INVALID_VALUE => "INVALID_VALUE",
        gl::INVALID_OPERATION => "INVALID_OPERATION",
        gl::INVALID_FRAMEBUFFER_OPERATION => "INVALID_FRAMEBUFFER_OPERATION",
        gl::OUT_OF_MEMORY => "OUT_OF_MEMORY",
        gl::STACK_UNDERFLOW => "STACK_UNDERFLOW",
        gl::STACK_OVERFLOW => "STACK_OVERFLOW",
        _ => "UNKNOWN",
    }
}

#[macro_export]
macro_rules! gl_assert_ok {
    () => {{
        let err = gl::GetError();
        assert_eq!(err, gl::NO_ERROR, "{}", $crate::utils::gl_err_to_str(err));
    }};
}

pub fn compile_shader(src: &str, kind: GLenum) -> Res<GLuint> {
    let shader;

    unsafe {
        shader = gl::CreateShader(kind);

        //compile shader
        let c_str = CString::new(src.as_bytes())?;
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        //get compile status
        let mut status = GLint::from(gl::FALSE);
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        //fail on error
        if status != GLint::from(gl::TRUE) {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = vec![0; len as usize - 1];
            gl::GetShaderInfoLog(
                shader,
                len,
                ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            return Err(str::from_utf8(&buf)?.into());
        }
    }
    Ok(shader)
}

pub fn link_programs(vs: GLuint, fs: GLuint) -> Res<GLuint> {
    let program;
    unsafe {
        program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);

        //link status
        let mut status = GLint::from(gl::FALSE);
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        //fail on error
        if status != GLint::from(gl::TRUE) {
            let mut len = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = vec![0; len as usize - 1];
            gl::GetProgramInfoLog(
                program,
                len,
                ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            return Err(str::from_utf8(&buf)?.into());
        }
    }
    Ok(program)
}

#[rustfmt::skip]
pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> [f32; 16] {
    let tx = -(right + left) / (right - left);
    let ty = -(top + bottom) / (top - bottom);
    let tz = -(far + near) / (far - near);

    [
        2.0/(right - left),0.0,               0.0,            0.0,
        0.0,               2.0/(top - bottom),0.0,            0.0,
        0.0,               0.0,               -2.0/(far-near),0.0,
        tx,                ty,                tz,             1.0
    ]
}
