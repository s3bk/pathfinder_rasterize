use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::{
    concurrent::{
        rayon::RayonExecutor,
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
    rect::{RectI, RectF},
    transform2d::Transform2F,
};
use pathfinder_color::ColorF;
use pathfinder_resources::embedded::EmbeddedResourceLoader;

use khronos_egl as egl;
use image::RgbaImage;
use egl::{Instance};

pub struct Rasterizer {
    egl: Instance<egl::Static>,
    display: egl::Display,
    surface: egl::Surface,
    context: egl::Context,
    renderer: Option<(Renderer<GLDevice>, Vector2I)>,
    render_level: RendererLevel,
}

impl Rasterizer {
    pub fn new() -> Self {
        Rasterizer::new_with_level(RendererLevel::D3D9)
    }
    pub fn new_with_level(render_level: RendererLevel) -> Self {
        let egl = egl::Instance::new(egl::Static);

        let display = egl.get_display(egl::DEFAULT_DISPLAY).expect("display");
        let (major, minor) = egl.initialize(display).expect("init");
    
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
            egl::WIDTH, 1,
            egl::HEIGHT, 1,
            egl::NONE
        ];
        let surface = egl.create_pbuffer_surface(display, config, &pbuffer_attrib_list).unwrap();
    
        egl.bind_api(egl::OPENGL_API).expect("unable to select OpenGL API");
    
        let context = egl.create_context(display, config, None, &[egl::NONE]).unwrap();
        egl.make_current(display, Some(surface), Some(surface), Some(context)).unwrap();
    
        // Setup Open GL.
        gl::load_with(|name| egl.get_proc_address(name).unwrap() as *const std::ffi::c_void);
    

        Rasterizer {
            egl, display, surface, context,
            renderer: None, render_level,
        }
    }

    fn make_current(&self) {
        self.egl.make_current(
            self.display,
            Some(self.surface),
            Some(self.surface),
            Some(self.context)
        ).unwrap();
    }

    fn renderer_for_size(&mut self, size: Vector2I) -> &mut Renderer<GLDevice> {
        let level = self.render_level;
        let size = Vector2I::new((size.x() + 15) & !15, (size.y() + 15) & !15);
        let (ref mut renderer, ref mut current_size) = *self.renderer.get_or_insert_with(|| {
            let resource_loader = EmbeddedResourceLoader::new();

            let renderer_gl_version = match level {
                RendererLevel::D3D9 => GLVersion::GLES3,
                RendererLevel::D3D11 => GLVersion::GL4,
            };
    
            let device = GLDevice::new(renderer_gl_version, 0);

            let tex = device.create_texture(TextureFormat::RGBA8, size);
            let fb = device.create_framebuffer(tex);
            let dest = DestFramebuffer::Other(fb);
            let render_options = RendererOptions {
                dest,
                background_color: None,
                show_debug_ui: false,
            };
            let renderer = Renderer::new(device,
                &resource_loader,
                RendererMode { level },
                render_options,
            );
            (renderer, size)
        });

        if size != *current_size {
            let tex = renderer.device().create_texture(TextureFormat::RGBA8, size);
            let fb = renderer.device().create_framebuffer(tex);
            let dest = DestFramebuffer::Other(fb);
            renderer.options_mut().dest = dest;
            *current_size = size;
        }

        renderer
    }

    pub fn rasterize(&mut self, mut scene: Scene, background: Option<ColorF>) -> RgbaImage {
        self.make_current();
        
        let view_box = dbg!(scene.view_box());
        let size = view_box.size().ceil().to_i32();
        let transform = Transform2F::from_translation(-view_box.origin());

        let renderer = self.renderer_for_size(size);
        renderer.options_mut().background_color = background;
        scene.set_view_box(RectF::new(Vector2F::zero(), view_box.size()));

        let options = BuildOptions {
            transform: RenderTransform::Transform2D(transform),
            dilation: Vector2F::default(),
            subpixel_aa_enabled: false
        };

        scene.build_and_render(renderer, options, RayonExecutor);

        let render_target = match renderer.options().dest {
            DestFramebuffer::Other(ref fb) => RenderTarget::Framebuffer(fb),
            _=> panic!()
        };
        let texture_data_receiver = renderer.device().read_pixels(&render_target, RectI::new(Vector2I::zero(), size));
        let pixels = match renderer.device().recv_texture_data(&texture_data_receiver) {
            TextureData::U8(pixels) => pixels,
            _ => panic!("Unexpected pixel format for default framebuffer!"),
        };

        RgbaImage::from_raw(size.x() as u32, size.y() as u32, pixels).unwrap()
    }
}

impl Drop for Rasterizer {
    fn drop(&mut self) {
        self.egl.terminate(self.display).unwrap();
    }
}

#[test]
fn test_render() {
    use pathfinder_geometry::rect::RectF;

    let mut scene = Scene::new();
    scene.set_view_box(RectF::new(Vector2F::zero(), Vector2F::new(100., 100.)));
    Rasterizer::new().rasterize(scene, None);
}