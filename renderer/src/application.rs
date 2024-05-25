use std::{ffi::{CStr, CString}, sync::Arc, time::SystemTime};

use egui_winit::winit::{event::{Event, WindowEvent}, event_loop::{ControlFlow, EventLoop}, keyboard::ModifiersState};

use crate::vk::{renderer::{Renderer, RendererOptions}, context::Context};
use crate::window::Window;

#[derive(Debug)]
pub struct ApplicationOptions {
    pub(crate) title : String,
    pub(crate) renderer : RendererOptions,
}

impl Default for ApplicationOptions {
    fn default() -> Self {
        Self {
            renderer: Default::default(),
            title: "WorldEdit".to_owned(),
        }
    }
}

impl ApplicationOptions {
    #[inline] pub fn title(mut self, title : impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    #[inline] pub fn renderer(mut self, renderer : RendererOptions) -> Self {
        self.renderer = renderer;
        self
    }
}

#[derive(Debug)]
pub enum ApplicationRenderError {
    InvalidSwapchain,
}

pub type PrepareFn = fn() -> ApplicationOptions;
pub type SetupFn<T> = fn(&mut Application) -> T;
pub type UpdateFn<T> = fn(&mut Application, &mut T);
pub type RenderFn<T> = fn(&mut Application, &mut T) -> Result<(), ApplicationRenderError>;
pub type WindowEventFn<T> = fn(&mut Application, &mut T, event: &WindowEvent);

pub struct ApplicationBuilder<State : 'static> {
    pub prepare : Option<PrepareFn>,
    pub setup : SetupFn<State>,
    pub update : Option<UpdateFn<State>>,
    pub event : Option<WindowEventFn<State>>,
    pub render : Option<RenderFn<State>>,
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

    pub(crate) fn run_render(&self, application : &mut Application, data : &mut T) -> bool {
        if let Some(render_fn) = self.render {
            match render_fn(application, data) {
                Err(ApplicationRenderError::InvalidSwapchain) => true,
                _ => false,
            }
        } else {
            false
        }
    }
}

#[allow(dead_code, unused)]
fn main_loop<T : 'static>(builder: ApplicationBuilder<T>) {
    let event_loop = EventLoop::new().unwrap();
    let settings = {
        if let Some(prepare) = builder.prepare {
            prepare()
        } else {
            ApplicationOptions::default()
        }
    };
    let mut app = Application::new(settings, &event_loop);
    let mut app_data = (builder.setup)(&mut app);
    let mut dirty_swapchain = false;

    let now = SystemTime::now();
    let mut modifiers = ModifiersState::default();

    event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Poll);

        if !app.window.is_minimized() {
            
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
                    if let Some(event_fn) = builder.event {
                        event_fn(&mut app, &mut app_data, &event);
                    }
                }
                Event::AboutToWait => {
                    let now = now.elapsed().unwrap();

                    match builder.update {
                        Some(update_fn) => {
                            update_fn(&mut app, &mut app_data);
                        }
                        None => {}
                    }

                    dirty_swapchain = builder.run_render(&mut app, &mut app_data);
                }
                Event::Suspended => println!("Suspended."),
                Event::Resumed => println!("Resumed."),
                Event::LoopExiting => app.renderer.logical_device.wait_idle(),
                _ => { }
            }
        }
    });
}


pub struct Application {
    pub context : Arc<Context>,
    pub renderer : Renderer,
    window : Window,
}

impl Application {
    pub fn build<T>(setup: SetupFn<T>) -> ApplicationBuilder<T> {
        ApplicationBuilder {
            prepare: None,
            setup,
            update: None,
            event: None,
            render: None,
        }
    }

    pub fn new(settings : ApplicationOptions, event_loop : &EventLoop<()>) -> Self {
        let window = Window::new(&settings, event_loop);

        let context = Arc::new(unsafe {
            let mut all_extensions = settings.renderer.instance_extensions.clone();
            all_extensions.extend(window.surface_extensions().iter().map(|&extension| CStr::from_ptr(extension).to_owned()));
            all_extensions.dedup();

            Context::new(CString::new("send-help").unwrap_unchecked(), all_extensions)
        });

        Self {
            context : context.clone(),
            renderer : Renderer::new(&settings.renderer, &context, &window),
            window,
        }
    }

    pub fn recreate_swapchain(&mut self) {

    }
}