use gl::types::{GLfloat, GLuint};
use glutin::{
    config::{Config, ConfigTemplateBuilder},
    context::ContextAttributesBuilder,
    display::GetGlDisplay,
    prelude::{GlConfig, GlDisplay, NotCurrentGlContext},
    surface::GlSurface,
};
use glutin_winit::{self, DisplayBuilder, GlWindow};
use glyph_brush::{ab_glyph::*, *};
use raw_window_handle::HasRawWindowHandle;
use std::{
    env,
    error::Error,
    ffi::{c_void, CString},
    io::{self, Write},
    mem, ptr, str,
    time::Duration,
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Theme, WindowBuilder},
};

use crate::{gl_assert_ok, utils::Res};
// pub mod text-document;

pub fn init() -> Res<()> {
    let events = EventLoop::new()?;
    events.set_control_flow(ControlFlow::Poll);
    const TITLE: &str = "glyph_brush opengl example - scroll to size, type to modify";

    let window_builder = WindowBuilder::new()
        .with_inner_size(winit::dpi::PhysicalSize::new(1024, 576))
        .with_transparent(true)
        .with_title(TITLE);

    // The template will match only the configurations supporting rendering
    // to windows.
    //
    // XXX We force transparency only on macOS, given that EGL on X11 doesn't
    // have it, but we still want to show window. The macOS situation is like
    // that, because we can query only one config at a time on it, but all
    // normal platforms will return multiple configs, so we can find the config
    // with transparency ourselves inside the `reduce`.

    let template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_transparency(cfg!(cgl_backend));

    let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));

    // let (window, gl_config) = display_builder.build(&events, <_>::default(), |mut configs| {
    //     configs
    //         .find(|c| c.srgb_capable() && c.num_samples() == 0)
    //         .unwrap()
    // })?;

    let (window, gl_config) = display_builder.build(&events, template, gl_config_picker)?;

    let raw_window_handle = window.as_ref().map(|window| window.raw_window_handle());

    let gl_display = gl_config.display();

    let context_attributes = ContextAttributesBuilder::new()
        .with_profile(glutin::context::GlProfile::Core)
        .with_context_api(glutin::context::ContextApi::OpenGl(Some(
            glutin::context::Version::new(3, 2),
        )))
        .build(raw_window_handle);

    let window = window.unwrap();
    let mut dimensions = window.inner_size();

    let (gl_surface, gl_ctx) = {
        let attrs = window.build_surface_attributes(<_>::default());
        let surface = unsafe { gl_display.create_window_surface(&gl_config, &attrs)? };
        let context = unsafe { gl_display.create_context(&gl_config, &context_attributes)? }
            .make_current(&surface)?;
        (surface, context)
    };

    gl::load_with(|symbol| gl_display.get_proc_address(&CString::new(symbol).unwrap()) as _);

    let sans = FontRef::try_from_slice(include_bytes!("../fonts/DejaVuSansMono.ttf"))?;
    let mut glyph_brush: GlyphBrush = GlyphBrushBuilder::using_font(sans).build();

    let mut texture = GlGlyphTexture::new(glyph_brush.texture_dimensions());

    events.run(move |event, elwt| match event {
        Event::AboutToWait => window.request_redraw(),
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => elwt.exit(),
            _ => (),
        },
        _ => (),
    })?;
    Ok(())
}

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

pub fn gl_config_picker(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
    configs
        .reduce(|accum, config| {
            let transparency_check = config.supports_transparency().unwrap_or(false)
                & !accum.supports_transparency().unwrap_or(false);
            if transparency_check || config.num_samples() > accum.num_samples() {
                config
            } else {
                accum
            }
        })
        .unwrap()
}
