use based::{context::Context, renderer::RenderInfo};
use vello::{
    kurbo::Affine, peniko::Color, util::RenderContext, AaSupport, RenderParams, Renderer,
    RendererOptions, Scene, SceneBuilder, SceneFragment,
};
use winit::{
    dpi::LogicalSize,
    event::{Event, MouseScrollDelta, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

#[derive(Default)]
pub struct Frontend {
    scroll_y: f64,
}

impl Frontend {
    pub async fn run(&mut self, mut context: Context) {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("Based Frontend")
            .with_resizable(false)
            .with_inner_size(LogicalSize::new(1080, 720))
            .with_transparent(true)
            .build(&event_loop)
            .unwrap();
        let mut ctx = RenderContext::new().unwrap();
        let size = window.inner_size();
        context.resize(size.width as usize, size.height as usize);
        context.load().await;
        context.go();
        let mut surface = ctx
            .create_surface(&window, size.width, size.height)
            .await
            .unwrap();
        let dev_handle = ctx.devices.get(surface.dev_id).unwrap();
        let render_options = RendererOptions {
            surface_format: Some(surface.format),
            use_cpu: false, //configurable??
            antialiasing_support: AaSupport::all(),
        };
        let mut renderer = Renderer::new(&dev_handle.device, render_options).unwrap();
        context.layoutify(window.scale_factor());

        let mut scene = Scene::new();
        let mut context_frag = SceneFragment::new();
        let mut render_info = RenderInfo::default();
        event_loop.run(move |event, _, ctrl| {
            ctrl.set_wait();
            println!("{:?}", event);
            match event {
                Event::WindowEvent { window_id, event } => match event {
                    WindowEvent::MouseWheel {
                        device_id,
                        delta,
                        phase,
                        modifiers,
                    } => match delta {
                        MouseScrollDelta::LineDelta(x, y) => {
                            render_info.scroll_y -= y as f64 * window.scale_factor();
                        }
                        MouseScrollDelta::PixelDelta(pos) => {
                            render_info.scroll_y -= pos.y as f64 * window.scale_factor();
                        }
                    },
                    _ => {}
                },
                Event::MainEventsCleared => {
                    window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    let dev_handle = ctx.devices.get(surface.dev_id).unwrap();
                    let render_params = RenderParams {
                        base_color: Color::TRANSPARENT,
                        width: size.width,
                        height: size.height,
                        antialiasing_method: vello::AaConfig::Area,
                    };
                    let mut context_builder = SceneBuilder::for_fragment(&mut context_frag);
                    let mut builder = SceneBuilder::for_scene(&mut scene);
                    context.render(&mut builder, render_info);
                    builder.append(&context_frag, Some(Affine::IDENTITY));
                    let surface_texture = surface.surface.get_current_texture().unwrap();
                    vello::block_on_wgpu(
                        &dev_handle.device,
                        renderer.render_to_surface_async(
                            &dev_handle.device,
                            &dev_handle.queue,
                            &scene,
                            &surface_texture,
                            &render_params,
                        ),
                    )
                    .unwrap();
                    surface_texture.present();
                }
                _ => {}
            }
        });
    }

    fn render_info(&self) -> RenderInfo {
        RenderInfo {
            scroll_y: self.scroll_y,
        }
    }

    fn handle_window_event(&mut self, event: WindowEvent) {}
}
