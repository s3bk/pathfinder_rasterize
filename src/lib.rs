use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::{
    concurrent::{
        rayon::RayonExecutor,
        scene_proxy::SceneProxy
    },
    gpu::{
        options::{DestFramebuffer, RendererOptions, RendererMode, RendererLevel},
        renderer::{Renderer},
    },
    scene::Scene,
    options::{BuildOptions, RenderTransform}
};
use pathfinder_gpu::{Device, TextureData, RenderTarget};
use pathfinder_geometry::{
    vector::{Vector2F, Vector2I},
    rect::{RectF, RectI},
};
use pathfinder_color::ColorF;
use pathfinder_resources::embedded::EmbeddedResourceLoader;

use glutin::{GlRequest, Api, WindowedContext, PossiblyCurrent};
use winit::{
    event_loop::EventLoop,
    window::{WindowBuilder, Window},
    dpi::{PhysicalSize},
};
use image::RgbaImage;

pub fn rasterize_scene(mut scene: Scene) -> RgbaImage {
    let render_level = RendererLevel::D3D9;
    let background = ColorF::new(0.0, 0.0, 0.0, 0.0);
    let resource_loader = EmbeddedResourceLoader::new();
    let size = scene.view_box().size().ceil().to_i32();
    let viewport = RectI::new(Vector2I::zero(), size);
    let event_loop = EventLoop::new();

    let (glutin_gl_version, renderer_gl_version) = match render_level {
        RendererLevel::D3D9 => ((3, 0), GLVersion::GLES3),
        RendererLevel::D3D11 => ((4, 3), GLVersion::GL4),
    };
    let physical_size = PhysicalSize::new(size.x() as u32, size.y() as u32);
    let windowed_context = glutin::ContextBuilder::new()
        .with_gl(GlRequest::Specific(Api::OpenGl, glutin_gl_version))
        .build_headless(&event_loop, physical_size)
        .unwrap();

    let windowed_context = unsafe {
        windowed_context.make_current().unwrap()
    };

    gl::load_with(|ptr| windowed_context.get_proc_address(ptr));

    // Create a Pathfinder renderer.
    let render_mode = RendererMode { level: render_level };
    let render_options = RendererOptions {
        dest:  DestFramebuffer::full_window(size),
        background_color: Some(background),
        show_debug_ui: false,
    };


    let mut renderer = Renderer::new(GLDevice::new(renderer_gl_version, 0),
        &resource_loader,
        render_mode,
        render_options,
    );
    let options = BuildOptions {
        transform: RenderTransform::default(),
        dilation: Vector2F::default(),
        subpixel_aa_enabled: false
    };
    scene.build_and_render(&mut renderer, options, RayonExecutor);

    let texture_data_receiver = renderer.device().read_pixels(&RenderTarget::Default, viewport);
    let pixels = match renderer.device().recv_texture_data(&texture_data_receiver) {
        TextureData::U8(pixels) => pixels,
        _ => panic!("Unexpected pixel format for default framebuffer!"),
    };

    RgbaImage::from_raw(viewport.width() as u32, viewport.height() as u32, pixels).unwrap()
}