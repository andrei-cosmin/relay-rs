use dioxus::desktop::tao::event::{Event, WindowEvent};
use dioxus::desktop::{Config, LogicalSize, WindowBuilder, use_window, use_wry_event_handler};
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;

const HEIGHT: f64 = 44.0;

const BAR_STYLE: &str = "position: fixed; top: 0; left: 0; right: 0; display: flex; \
    justify-content: flex-end; z-index: 10000;";

const BUTTON_STYLE: &str = "width: 46px; display: grid; place-items: center; padding: 0; \
    border: none; border-radius: 0; box-shadow: none; background: transparent; color: inherit; \
    cursor: default; font: 10px 'Segoe Fluent Icons', 'Segoe MDL2 Assets';";

const HOVER_CSS: &str = "
.title-bar-button:hover { background: rgba(127, 127, 127, 0.18); }
.title-bar-button.close:hover { background: #e81123; color: #ffffff; }
";

pub struct TitleBarWindow;

impl TitleBarWindow {
    pub fn config(title: &str, size: (f64, f64), min_size: (f64, f64)) -> Config {
        let window = Self::chrome(
            WindowBuilder::new()
                .with_title(title)
                .with_inner_size(LogicalSize::new(size.0, size.1))
                .with_min_inner_size(LogicalSize::new(min_size.0, min_size.1)),
        );
        let config = Config::new().with_window(window);
        if cfg!(windows) {
            config.with_menu(None)
        } else {
            config
        }
    }

    #[cfg(target_os = "macos")]
    fn chrome(window: WindowBuilder) -> WindowBuilder {
        use dioxus::desktop::tao::platform::macos::WindowBuilderExtMacOS;

        window
            .with_titlebar_transparent(true)
            .with_title_hidden(true)
            .with_fullsize_content_view(true)
    }

    #[cfg(windows)]
    fn chrome(window: WindowBuilder) -> WindowBuilder {
        use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;

        window.with_decorations(false).with_undecorated_shadow(true)
    }

    #[cfg(not(any(target_os = "macos", windows)))]
    fn chrome(window: WindowBuilder) -> WindowBuilder {
        window
    }
}

#[component]
pub fn TitleBar() -> Element {
    let window = use_window();
    let drag = window.clone();
    let zoom = window.clone();

    #[cfg(target_os = "macos")]
    use_hook(|| {
        use dioxus::desktop::LogicalPosition;
        use dioxus::desktop::wry::WebViewExtMacOS;

        _ = window
            .webview
            .set_traffic_light_inset(LogicalPosition::new(15.0, HEIGHT / 2.0 + 2.0));
    });

    if !cfg!(any(target_os = "macos", windows)) {
        return rsx! {};
    }

    rsx! {
        div {
            style: "{BAR_STYLE} height: {HEIGHT}px;",
            onmousedown: move |event: MouseEvent| {
                if event.data().trigger_button() == Some(MouseButton::Primary) {
                    drag.drag();
                }
            },
            ondoubleclick: move |_| zoom.toggle_maximized(),
            if cfg!(windows) {
                WindowButtons {}
            }
        }
    }
}

#[component]
fn WindowButtons() -> Element {
    let window = use_window();
    let mut maximized = use_signal(|| window.is_maximized());

    use_wry_event_handler({
        let window = window.clone();
        move |event, _| {
            if let Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } = event
            {
                maximized.set(window.is_maximized());
            }
        }
    });

    let minimize = window.clone();
    let zoom = window.clone();
    let close = window.clone();

    rsx! {
        document::Style { {HOVER_CSS} }
        button {
            class: "title-bar-button",
            style: "{BUTTON_STYLE} height: {HEIGHT}px;",
            tabindex: -1,
            onmousedown: move |event| event.stop_propagation(),
            onclick: move |_| minimize.set_minimized(true),
            "\u{E921}"
        }
        button {
            class: "title-bar-button",
            style: "{BUTTON_STYLE} height: {HEIGHT}px;",
            tabindex: -1,
            onmousedown: move |event| event.stop_propagation(),
            onclick: move |_| zoom.toggle_maximized(),
            if maximized() { "\u{E923}" } else { "\u{E922}" }
        }
        button {
            class: "title-bar-button close",
            style: "{BUTTON_STYLE} height: {HEIGHT}px;",
            tabindex: -1,
            onmousedown: move |event| event.stop_propagation(),
            onclick: move |_| close.close(),
            "\u{E8BB}"
        }
    }
}
