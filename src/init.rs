use glutin::{
    config::{Config, ConfigTemplateBuilder},
    context::ContextAttributesBuilder,
    display::GetGlDisplay,
    prelude::{GlConfig, GlDisplay, NotCurrentGlContext},
    surface::GlSurface,
};
use glutin_winit::{self, DisplayBuilder, GlWindow};
use glyph_brush::{ab_glyph::*, Section, *};
use raw_window_handle::HasRawWindowHandle;
use std::{
    ffi::{c_void, CString},
    time::Duration,
};
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowBuilder,
};

use crate::{
    gl_assert_ok,
    gl_renderer::render_gl::{GLTextPipe, GlGlyphTexture},
    utils::{Res, Vertex},
};

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
    let dimensions = window.inner_size();

    let (gl_surface, gl_ctx) = {
        let attrs = window.build_surface_attributes(<_>::default());
        let surface = unsafe { gl_display.create_window_surface(&gl_config, &attrs)? };
        let context = unsafe { gl_display.create_context(&gl_config, &context_attributes)? }
            .make_current(&surface)?;
        (surface, context)
    };

    gl::load_with(|symbol| gl_display.get_proc_address(&CString::new(symbol).unwrap()) as _);

    let max_image_dimension = {
        let mut value = 0;
        unsafe { gl::GetIntegerv(gl::MAX_TEXTURE_SIZE, &mut value) };
        value as u32
    };

    let sans = FontRef::try_from_slice(include_bytes!("../fonts/DejaVuSansMono.ttf"))?;
    let mut glyph_brush = GlyphBrushBuilder::using_font(sans).build();

    let mut texture = GlGlyphTexture::new(glyph_brush.texture_dimensions());
    let mut text_pipe = GLTextPipe::new(dimensions)?;

    let mut text: String = include_str!("../text/lipsum.txt").into();
    let font_size: f32 = 18.0;

    let mut interval = spin_sleep_util::interval(Duration::from_secs(1) / 250);
    let mut reporter = spin_sleep_util::RateReporter::new(Duration::from_secs(1));

    events.run(move |event, elwt| match event {
        Event::AboutToWait => window.request_redraw(),
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => elwt.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match logical_key {
                Key::Named(NamedKey::Escape) => elwt.exit(),
                Key::Named(NamedKey::Backspace) => {
                    text.pop();
                }
                key => {
                    if let Some(str) = key.to_text() {
                        text.push_str(str);
                    }
                }
            },
            WindowEvent::RedrawRequested => {
                let width = dimensions.width as f32;
                let height = dimensions.height as f32;
                let scale = (font_size * window.scale_factor() as f32).round();
                let base_text = Text::new(&text).with_scale(scale);

                //queue sections of text
                glyph_brush.queue(
                    Section::default()
                        .add_text(base_text.with_color([0.9, 0.3, 0.3, 0.1]))
                        .with_bounds((width / 3.15, height)),
                );

                glyph_brush.queue(
                    Section::default()
                        .add_text(base_text.with_color([0.9, 0.3, 0.3, 1.0]))
                        .with_screen_position((width / 2.0, height / 2.0))
                        .with_bounds((width / 3.15, height))
                        .with_layout(
                            Layout::default()
                                .h_align(HorizontalAlign::Center)
                                .v_align(VerticalAlign::Center),
                        ),
                );

                glyph_brush.queue(
                    Section::default()
                        .add_text(base_text.with_color([0.9, 0.9, 0.9, 1.0]))
                        .with_screen_position((width, height))
                        .with_bounds((width / 3.15, height))
                        .with_layout(
                            Layout::default()
                                .h_align(HorizontalAlign::Right)
                                .v_align(VerticalAlign::Bottom),
                        ),
                );

                //process the queue
                let mut brush_action;
                loop {
                    brush_action = glyph_brush.process_queued(
                        |rect, tex_data| unsafe {
                            gl::BindTexture(gl::TEXTURE_2D, texture.name);
                            gl::TexSubImage2D(
                                gl::TEXTURE_2D,
                                0,
                                rect.min[0] as i32,
                                rect.min[1] as i32,
                                rect.width() as i32,
                                rect.height() as i32,
                                gl::RED,
                                gl::UNSIGNED_BYTE,
                                tex_data.as_ptr() as *const c_void,
                            );

                            gl_assert_ok!();
                        },
                        to_vertex,
                    );

                    match brush_action {
                        Ok(_) => break,
                        Err(BrushError::TextureTooSmall { suggested, .. }) => {
                            let (new_width, new_height) = if (suggested.0 > max_image_dimension
                                || suggested.1 > max_image_dimension)
                                && (glyph_brush.texture_dimensions().0 < max_image_dimension
                                    || glyph_brush.texture_dimensions().1 < max_image_dimension)
                            {
                                (max_image_dimension, max_image_dimension)
                            } else {
                                suggested
                            };
                            eprint!("\r                            \r");
                            eprintln!("Resizing glyph texture -> {new_width}x{new_height}");

                            // Recreate texture to larger size
                            texture = GlGlyphTexture::new((new_width, new_height));

                            glyph_brush.resize_texture(new_width, new_height);
                        }
                    }
                }

                // upload new vertices to GPU if text has changed
                match brush_action.unwrap() {
                    BrushAction::Draw(vertices) => text_pipe.upload_vertices(&vertices),
                    BrushAction::ReDraw => {}
                }

                //draw text to the screen
                unsafe {
                    gl::Clear(gl::COLOR_BUFFER_BIT);
                }

                text_pipe.draw();

                //swap front and back buffers to render text on screen
                gl_surface.swap_buffers(&gl_ctx).unwrap();

                if let Some(rate) = reporter.increment_and_report() {
                    window.set_title(&format!("{TITLE} {rate:.0} FPS"));
                }
                interval.tick();
            }
            _ => (),
        },
        _ => (),
    })?;
    Ok(())
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

#[inline]
pub fn to_vertex(
    glyph_brush::GlyphVertex {
        mut tex_coords,
        pixel_coords,
        bounds,
        extra,
    }: glyph_brush::GlyphVertex,
) -> Vertex {
    let gl_bounds = bounds;

    let mut gl_rect = Rect {
        min: point(pixel_coords.min.x, pixel_coords.min.y),
        max: point(pixel_coords.max.x, pixel_coords.max.y),
    };

    // handle overlapping bounds, modify uv_rect to preserve texture aspect
    if gl_rect.max.x > gl_bounds.max.x {
        let old_width = gl_rect.width();
        gl_rect.max.x = gl_bounds.max.x;
        tex_coords.max.x = tex_coords.min.x + tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.min.x < gl_bounds.min.x {
        let old_width = gl_rect.width();
        gl_rect.min.x = gl_bounds.min.x;
        tex_coords.min.x = tex_coords.max.x - tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.max.y > gl_bounds.max.y {
        let old_height = gl_rect.height();
        gl_rect.max.y = gl_bounds.max.y;
        tex_coords.max.y = tex_coords.min.y + tex_coords.height() * gl_rect.height() / old_height;
    }
    if gl_rect.min.y < gl_bounds.min.y {
        let old_height = gl_rect.height();
        gl_rect.min.y = gl_bounds.min.y;
        tex_coords.min.y = tex_coords.max.y - tex_coords.height() * gl_rect.height() / old_height;
    }

    [
        gl_rect.min.x,
        gl_rect.max.y,
        extra.z,
        gl_rect.max.x,
        gl_rect.min.y,
        tex_coords.min.x,
        tex_coords.max.y,
        tex_coords.max.x,
        tex_coords.min.y,
        extra.color[0],
        extra.color[1],
        extra.color[2],
        extra.color[3],
    ]
}
