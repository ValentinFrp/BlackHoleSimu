use std::future::Future;
use std::sync::Arc;

use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::Window;

use crate::camera::{CameraController, OrbitCamera};
use crate::renderer::Renderer;
use crate::scene::Scene;

#[cfg(not(target_arch = "wasm32"))]
fn spawn(f: impl Future<Output = ()> + 'static) {
    pollster::block_on(f);
}
#[cfg(target_arch = "wasm32")]
fn spawn(f: impl Future<Output = ()> + 'static) {
    wasm_bindgen_futures::spawn_local(f);
}

enum AppAction {
    RendererReady(Renderer),
}

enum Stage {
    Uninitialized,
    Loading,
    Running(Renderer),
}

struct App {
    proxy: EventLoopProxy<AppAction>,
    window: Option<Arc<Window>>,
    stage: Stage,
    camera: OrbitCamera,
    controller: CameraController,
    scene: Scene,
    start: Instant,
}

impl App {
    fn new(event_loop: &EventLoop<AppAction>) -> Self {
        Self {
            proxy: event_loop.create_proxy(),
            window: None,
            stage: Stage::Uninitialized,
            camera: OrbitCamera::new(20.0, 50.0),
            controller: CameraController::default(),
            scene: Scene::default(),
            start: Instant::now(),
        }
    }

    fn redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler<AppAction> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !matches!(self.stage, Stage::Uninitialized) {
            return;
        }
        self.stage = Stage::Loading;

        #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
        let mut attributes = Window::default_attributes().with_title("BlackHoleSimu");

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            let canvas = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("canvas"))
                .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
                .expect("élément <canvas id=\"canvas\"> introuvable dans la page");
            attributes = attributes.with_canvas(Some(canvas));
        }

        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("création de la fenêtre impossible"),
        );
        self.window = Some(window.clone());

        let display_handle = event_loop.owned_display_handle();
        let proxy = self.proxy.clone();
        spawn(async move {
            let renderer = Renderer::new(window, display_handle).await;
            let _ = proxy.send_event(AppAction::RendererReady(renderer));
        });
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, action: AppAction) {
        match action {
            AppAction::RendererReady(renderer) => {
                self.stage = Stage::Running(renderer);
                self.redraw();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Stage::Running(renderer) = &mut self.stage {
                    renderer.resize(size.width, size.height);
                }
                self.redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.controller.on_mouse_button(button, state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.controller.on_cursor_moved(position, &mut self.camera);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.controller.on_scroll(delta, &mut self.camera);
            }
            WindowEvent::RedrawRequested => {
                let time = self.start.elapsed().as_secs_f32();
                if let Stage::Running(renderer) = &mut self.stage {
                    renderer.render(&self.camera, &self.scene, time);
                }
                self.redraw();
            }
            _ => {}
        }
    }
}

pub fn run() {
    let event_loop = EventLoop::with_user_event()
        .build()
        .expect("création de l'EventLoop impossible");

    #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
    let mut app = App::new(&event_loop);

    #[cfg(not(target_arch = "wasm32"))]
    event_loop
        .run_app(&mut app)
        .expect("erreur dans la boucle d'évènements");

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
    }
}
