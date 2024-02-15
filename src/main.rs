use gl::types::GLfloat;
use glutin::{
    config::ConfigTemplateBuilder,
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
    ffi::CString,
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

// pub mod text-document;
pub type Res<T> = Result<T, Box<dyn std::error::Error>>;

fn main() -> Res<()> {
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

    let (window, gl_config) = display_builder.build(&events, <_>::default(), |mut configs| {
        configs
            .find(|c| c.srgb_capable() && c.num_samples() == 0)
            .unwrap()
    })?;

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
