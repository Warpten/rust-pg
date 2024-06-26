use std::{ffi::{CStr, CString}, sync::Arc, time::SystemTime};

use egui_winit::winit::{event::{Event, WindowEvent}, event_loop::{ControlFlow, EventLoop}, keyboard::ModifiersState};

use crate::orchestration::rendering::{Orchestrator, RendererOrchestrator};
use crate::vk::{context::Context, renderer::RendererOptions};
use crate::window::Window;

type OrchestratorFn = fn(Arc<Context>) -> Orchestrator;

pub struct ApplicationOptions {
    pub title : String,
    pub renderer_options : RendererOptions,
    pub device_extensions : Vec<CString>,
    pub instance_extensions : Vec<CString>,
    pub orchestrator : OrchestratorFn,
}

impl Default for ApplicationOptions {
    fn default() -> Self {
        Self {
            title: "WorldEdit".to_owned(),

            orchestrator : Orchestrator::new,

            renderer_options: Default::default(),
            device_extensions : vec![],
            instance_extensions : vec![],
        }
    }
}

impl ApplicationOptions {
    #[inline] pub fn title(mut self, title : impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    #[inline] pub fn instance_extension(mut self, value : CString) -> Self {
        self.instance_extensions.push(value);
        self
    }

    #[inline] pub fn device_extension(mut self, value : CString) -> Self {
        self.device_extensions.push(value);
        self
    }

    value_builder! { renderer, renderer_options, RendererOptions }
    value_builder! { orchestrator, orchestrator, OrchestratorFn }
}

#[derive(Debug)]
pub enum RendererError {
    InvalidSwapchain,
}

pub type PrepareFn = fn() -> ApplicationOptions;
pub type SetupFn<T> = fn(&mut Application) -> T;
pub type UpdateFn<T> = fn(&mut Application, &mut T);
pub type RenderFn<T> = fn(&mut Application, &mut T) -> Result<(), RendererError>;
pub type WindowEventFn<T> = fn(&mut Application, &mut T, event : &WindowEvent);
pub type InterfaceFn<T> = fn(&mut T, ctx : &mut egui::Context);

pub struct ApplicationBuilder<State : 'static> {
    pub prepare : Option<PrepareFn>,
    pub setup : SetupFn<State>,
    pub update : Option<UpdateFn<State>>,
    pub event : Option<WindowEventFn<State>>,
    pub render : Option<RenderFn<State>>,
}

pub struct ApplicationCallbacks<State : 'static> {
    pub prepare : PrepareFn,
    pub setup : SetupFn<State>,
    pub update : UpdateFn<State>,
    pub event : WindowEventFn<State>,
    pub render : RenderFn<State>,
}

impl<T> ApplicationBuilder<T> {
    pub fn prepare(mut self, prepare: PrepareFn) -> Self {
        self.prepare = Some(prepare);
        self
    }

    pub fn update(mut self, update: UpdateFn<T>) -> Self {
        self.update = Some(update);
        self
    }

    pub fn render(mut self, render: RenderFn<T>) -> Self {
        self.render = Some(render);
        self
    }

    pub fn window_event(mut self, window_event: WindowEventFn<T>) -> Self {
        self.event = Some(window_event);
        self
    }

    pub fn run(self) {
        main_loop(self);
    }
}

#[allow(dead_code, unused)]
fn main_loop<T : 'static>(builder: ApplicationBuilder<T>) {
    let event_loop = EventLoop::new().unwrap();

    let builder = ApplicationCallbacks {
        prepare: builder.prepare.unwrap_or(ApplicationOptions::default),
        setup: builder.setup,
        update: builder.update.unwrap_or(|_, _| { }),
        event: builder.event.unwrap_or(|_, _, _| { }),
        render: builder.render.unwrap_or(|_, _| Ok(())),
    };

    let mut settings = (builder.prepare)();

    let mut app = Application::new(settings, &event_loop);
    let mut app_data = (builder.setup)(&mut app);
    let mut dirty_swapchain = false;

    let now = SystemTime::now();
    let mut modifiers = ModifiersState::default();

    event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Poll);

        if !app.orchestrator.context.window.is_minimized() {
            if dirty_swapchain {
                app.recreate_swapchain();
                dirty_swapchain = false;
            }

            match event {
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => target.exit(),
                        WindowEvent::ModifiersChanged(m) => modifiers = m.state(),
                        _ => (),
                    }
                    (builder.event)(&mut app, &mut app_data, &event);
                }
                Event::AboutToWait => {
                    puffin::GlobalProfiler::lock().new_frame();
            
                    let now = now.elapsed().unwrap();

                    (builder.update)(&mut app, &mut app_data);

                    dirty_swapchain = match (builder.render)(&mut app, &mut app_data) {
                        Ok(_) => false,
                        Err(RendererError::InvalidSwapchain) => true,
                    };
                }
                Event::Suspended => println!("Suspended."),
                Event::Resumed => println!("Resumed."),
                Event::LoopExiting => app.orchestrator.context.device.wait_idle(),
                _ => { }
            }
        }
    });
}


pub struct Application {
    pub orchestrator : RendererOrchestrator,
}

impl Application {
    pub fn build<T>(setup: SetupFn<T>) -> ApplicationBuilder<T> {
        ApplicationBuilder {
            setup,
            prepare : None,
            update : None,
            event : None,
            render : None,
        }
    }

    pub fn new(options : ApplicationOptions, event_loop : &EventLoop<()>) -> Self {
        let mut window = Window::new(&options, event_loop);

        let context = Arc::new(unsafe {
            let mut all_extensions = options.instance_extensions.clone();
            all_extensions.extend(window.surface_extensions().iter().map(|&extension| CStr::from_ptr(extension).to_owned()));
            all_extensions.push(ash::ext::debug_utils::NAME.into());
            all_extensions.dedup();

            Context::new(CString::new("send-help").unwrap_unchecked(), all_extensions)
        });
        window.create_surface(&context);

        let orchestrator = (options.orchestrator)(context).build(options.renderer_options, window, options.device_extensions);

        Self {
            orchestrator,
        }
    }

    pub fn recreate_swapchain(&mut self) {
        self.orchestrator.recreate_swapchain();
    }
}