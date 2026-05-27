use std::future::Future;
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::Window;

use crate::renderer::WgpuState;

#[cfg(not(target_arch = "wasm32"))]
fn spawn(f: impl Future<Output = ()> + 'static) {
    pollster::block_on(f);
}
#[cfg(target_arch = "wasm32")]
fn spawn(f: impl Future<Output = ()> + 'static) {
    wasm_bindgen_futures::spawn_local(f);
}

enum AppAction {
    Ready(WgpuState),
}

enum AppState {
    Uninitialized,
    Loading,
    Running(WgpuState),
}

struct App {
    proxy: EventLoopProxy<AppAction>,
    window: Option<Arc<Window>>,
    state: AppState,
}

impl App {
    fn new(event_loop: &EventLoop<AppAction>) -> Self {
        Self {
            proxy: event_loop.create_proxy(),
            window: None,
            state: AppState::Uninitialized,
        }
    }
}

impl ApplicationHandler<AppAction> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !matches!(self.state, AppState::Uninitialized) {
            return;
        }
        self.state = AppState::Loading;

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
            let state = WgpuState::new(window, display_handle).await;
            let _ = proxy.send_event(AppAction::Ready(state));
        });
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, action: AppAction) {
        match action {
            AppAction::Ready(state) => {
                self.state = AppState::Running(state);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let AppState::Running(state) = &mut self.state else {
            return;
        };

        match event {
            WindowEvent::Resized(size) => {
                state.resize(size.width, size.height);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                state.render();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
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
