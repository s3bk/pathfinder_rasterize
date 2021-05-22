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
use pathfinder_gpu::{Device, TextureData, RenderTarget, TextureFormat};
use pathfinder_geometry::{
    vector::{Vector2F, Vector2I},
    rect::{RectF, RectI},
};
use pathfinder_color::ColorF;
use pathfinder_resources::embedded::EmbeddedResourceLoader;

use khronos_egl as egl;
use image::RgbaImage;
use egl::api::EGL1_2;

pub fn rasterize_scene(mut scene: Scene) -> RgbaImage {
    let render_level = RendererLevel::D3D9;
    let background = ColorF::new(0.0, 0.0, 0.0, 0.0);
    let resource_loader = EmbeddedResourceLoader::new();
    let size = scene.view_box().size().ceil().to_i32();
    let viewport = RectI::new(Vector2I::zero(), size);

    let lib;
    let egl;

    unsafe {
        lib = libloading::Library::new("libEGL.so.1").expect("unable to find libEGL.so.1");
        egl = egl::DynamicInstance::<egl::EGL1_4>::load_required_from(lib).expect("unable to load libEGL.so.1");
    }
    
    let display = egl.get_display(egl::DEFAULT_DISPLAY).expect("display");
    let (a, b) = egl.initialize(display).expect("init");

    let attrib_list = [
        egl::SURFACE_TYPE, egl::PBUFFER_BIT,
        egl::BLUE_SIZE, 8,
        egl::GREEN_SIZE, 8,
        egl::RED_SIZE, 8,
        egl::DEPTH_SIZE, 8,
        egl::RENDERABLE_TYPE, egl::OPENGL_BIT,
        egl::NONE
    ];
    
    let config = egl.choose_first_config(display, &attrib_list).unwrap().unwrap();

    let pbuffer_attrib_list = [
        egl::WIDTH, size.x(),
        egl::HEIGHT, size.y(),
        egl::NONE
    ];
    let surface = egl.create_pbuffer_surface(display, config, &pbuffer_attrib_list).unwrap();

    egl.bind_api(egl::OPENGL_API).expect("unable to select OpenGL API");

    let context = egl.create_context(display, config, None, &[egl::NONE]).unwrap();
    egl.make_current(display, Some(surface), Some(surface), Some(context)).unwrap();

    // Setup Open GL.
    gl::load_with(|name| egl.get_proc_address(name).unwrap() as *const std::ffi::c_void);

    let renderer_gl_version = match render_level {
        RendererLevel::D3D9 => GLVersion::GLES3,
        RendererLevel::D3D11 => GLVersion::GL4,
    };

    let render_to_texture = false;

    let device = GLDevice::new(renderer_gl_version, 0);

    let dest = if render_to_texture {
        let tex = device.create_texture(TextureFormat::RGBA8, viewport.size());
        let fb = device.create_framebuffer(tex);
        DestFramebuffer::Other(fb)
    } else {
        DestFramebuffer::full_window(size)
    };

    // Create a Pathfinder renderer.
    let render_mode = RendererMode { level: render_level };
    let render_options = RendererOptions {
        dest,
        background_color: Some(background),
        show_debug_ui: false,
    };
    let mut renderer = Renderer::new(device,
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

    let render_target = match renderer.options().dest {
        DestFramebuffer::Other(ref fb) => RenderTarget::Framebuffer(fb),
        _=> RenderTarget::Default
    };
    let texture_data_receiver = renderer.device().read_pixels(&render_target, viewport);
    let pixels = match renderer.device().recv_texture_data(&texture_data_receiver) {
        TextureData::U8(pixels) => pixels,
        _ => panic!("Unexpected pixel format for default framebuffer!"),
    };

    egl.terminate(display).unwrap();

    RgbaImage::from_raw(viewport.width() as u32, viewport.height() as u32, pixels).unwrap()
}

#[test]
fn test_render() {
    let mut scene = Scene::new();
    scene.set_view_box(RectF::new(Vector2F::zero(), Vector2F::new(100., 100.)));
    rasterize_scene(scene);
}