/*
 * Copyright (c) 2025 Bastiaan van der Plaat
 *
 * SPDX-License-Identifier: MIT
 */

use std::ffi::{CStr, CString, c_char, c_void};
use std::path::Path;
use std::process::exit;
use std::ptr::{null, null_mut};
use std::{env, fs, iter};

use self::headers::*;
use crate::{Event, LogicalPoint, LogicalSize, WebviewBuilder};

mod headers;

/// Webview
pub(crate) struct Webview {
    builder: Option<WebviewBuilder>,
    app: *mut GtkApplication,
    window: *mut GtkWindow,
    webview: *mut WebKitWebView,
    event_handler: Option<fn(&mut Webview, Event)>,
    #[cfg(feature = "remember_window_state")]
    remember_window_state: bool,
}

impl Webview {
    pub(crate) fn new(builder: WebviewBuilder) -> Self {
        #[cfg(feature = "remember_window_state")]
        let remember_window_state = builder.remember_window_state;
        Self {
            builder: Some(builder),
            app: unsafe { gtk_application_new(null_mut(), G_APPLICATION_DEFAULT_FLAGS) },
            window: null_mut(),
            webview: null_mut(),
            event_handler: None,
            #[cfg(feature = "remember_window_state")]
            remember_window_state,
        }
    }

    #[cfg(feature = "remember_window_state")]
    fn settings_path(&self) -> String {
        let config_dir = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
            format!(
                "{}/.config",
                env::var("HOME").expect("Can't read $HOME env variable")
            )
        });
        format!(
            "{}/{}/settings.ini",
            config_dir,
            env::current_exe()
                .expect("Can't get current process name")
                .file_name()
                .expect("Can't get current process name")
                .to_string_lossy()
        )
    }

    #[cfg(feature = "remember_window_state")]
    fn load_window_state(&mut self) {
        unsafe {
            let settings = g_key_file_new();
            let file = CString::new(self.settings_path()).expect("Can't convert to CString");
            let mut error = null_mut();
            g_key_file_load_from_file(settings, file.as_ptr(), 0, &mut error);
            if error.is_null() {
                let group = c"window".as_ptr();
                let x = g_key_file_get_integer(settings, group, c"x".as_ptr(), null_mut());
                let y = g_key_file_get_integer(settings, group, c"y".as_ptr(), null_mut());
                gtk_window_move(self.window, x, y);

                let width = g_key_file_get_integer(settings, group, c"width".as_ptr(), null_mut());
                let height =
                    g_key_file_get_integer(settings, group, c"height".as_ptr(), null_mut());
                gtk_window_set_default_size(self.window, width, height);

                let maximized =
                    g_key_file_get_boolean(settings, group, c"maximized".as_ptr(), null_mut());
                if maximized {
                    gtk_window_maximize(self.window);
                }
            } else {
                g_error_free(error);
            }
            g_key_file_free(settings);
        }
    }

    #[cfg(feature = "remember_window_state")]
    fn save_window_state(&mut self) {
        let settings_path = self.settings_path();
        fs::create_dir_all(
            Path::new(&settings_path)
                .parent()
                .expect("Can't create settings directory"),
        )
        .expect("Can't create settings directory");

        unsafe {
            let settings = g_key_file_new();
            let group = c"window".as_ptr();

            let mut x = 0;
            let mut y = 0;
            gtk_window_get_position(self.window, &mut x, &mut y);
            g_key_file_set_integer(settings, group, c"x".as_ptr(), x);
            g_key_file_set_integer(settings, group, c"y".as_ptr(), y);

            let mut width = 0;
            let mut height = 0;
            gtk_window_get_size(self.window, &mut width, &mut height);
            g_key_file_set_integer(settings, group, c"width".as_ptr(), width);
            g_key_file_set_integer(settings, group, c"height".as_ptr(), height);

            let maximized = gtk_window_is_maximized(self.window);
            g_key_file_set_boolean(settings, group, c"maximized".as_ptr(), maximized);

            let file = CString::new(settings_path).expect("Can't convert to CString");
            g_key_file_save_to_file(settings, file.as_ptr(), null_mut());
            g_key_file_free(settings);
        }
    }

    fn send_event(&mut self, event: Event) {
        self.event_handler.expect("Should be some")(self, event);
    }
}

impl crate::Webview for Webview {
    fn run(&mut self, event_handler: fn(&mut Webview, Event)) -> ! {
        self.event_handler = Some(event_handler);
        unsafe {
            g_signal_connect_data(
                self.app as *mut Self as *mut c_void,
                c"activate".as_ptr(),
                app_on_activate as *const c_void,
                self as *mut Self as *const c_void,
                null(),
                G_CONNECT_DEFAULT,
            );
        }

        // Start event loop
        let args = env::args()
            .map(|arg| CString::new(arg.as_str()).expect("Can't convert to CString"))
            .collect::<Vec<CString>>();
        let argv = args
            .iter()
            .map(|arg| arg.as_ptr())
            .chain(iter::once(null()))
            .collect::<Vec<*const c_char>>();
        exit(unsafe {
            g_application_run(
                self.app as *mut GApplication,
                argv.len() as i32,
                argv.as_ptr(),
            )
        });
    }

    fn set_title(&mut self, title: impl AsRef<str>) {
        let title = CString::new(title.as_ref()).expect("Can't convert to CString");
        unsafe { gtk_window_set_title(self.window, title.as_ptr()) }
    }

    fn position(&self) -> LogicalPoint {
        let mut x = 0;
        let mut y = 0;
        unsafe { gtk_window_get_position(self.window, &mut x, &mut y) };
        LogicalPoint::new(x as f32, y as f32)
    }

    fn size(&self) -> LogicalSize {
        let mut width = 0;
        let mut height = 0;
        unsafe { gtk_window_get_size(self.window, &mut width, &mut height) };
        LogicalSize::new(width as f32, height as f32)
    }

    fn set_position(&mut self, point: LogicalPoint) {
        unsafe { gtk_window_move(self.window, point.x as i32, point.y as i32) }
    }

    fn set_size(&mut self, size: LogicalSize) {
        unsafe { gtk_window_set_default_size(self.window, size.width as i32, size.height as i32) }
    }

    fn set_min_size(&mut self, min_size: LogicalSize) {
        unsafe {
            gtk_widget_set_size_request(
                self.window as *mut GtkWidget,
                min_size.width as i32,
                min_size.height as i32,
            )
        }
    }

    fn set_resizable(&mut self, resizable: bool) {
        unsafe { gtk_window_set_resizable(self.window, resizable) }
    }

    fn load_url(&mut self, url: impl AsRef<str>) {
        let url = CString::new(url.as_ref()).expect("Can't convert to CString");
        unsafe { webkit_web_view_load_uri(self.webview, url.as_ptr()) }
    }

    fn load_html(&mut self, html: impl AsRef<str>) {
        let html = CString::new(html.as_ref()).expect("Can't convert to CString");
        unsafe { webkit_web_view_load_html(self.webview, html.as_ptr(), null()) }
    }

    fn evaluate_script(&mut self, script: impl AsRef<str>) {
        let script = script.as_ref();
        unsafe {
            webkit_web_view_evaluate_javascript(
                self.webview,
                script.as_ptr() as *const c_char,
                script.len(),
                null(),
                null(),
                null(),
                null(),
                null(),
            )
        }
    }
}

extern "C" fn app_on_activate(app: *mut GApplication, _self: &mut Webview) {
    let builder = _self.builder.take().expect("Should be some");

    // Force dark mode if enabled
    if builder.should_force_dark_mode {
        unsafe {
            let settings = gtk_settings_get_default();
            g_object_set(
                settings as *mut c_void,
                c"gtk-application-prefer-dark-theme".as_ptr(),
                1 as *const c_void,
                null::<c_void>(),
            );
        }
    }

    // Create window
    unsafe {
        _self.window = gtk_application_window_new(app as *mut GtkApplication);
        let title = CString::new(builder.title).expect("Can't convert to CString");
        gtk_window_set_title(_self.window, title.as_ptr());
        gtk_window_set_default_size(
            _self.window,
            builder.size.width as i32,
            builder.size.height as i32,
        );
        if let Some(min_size) = builder.min_size {
            gtk_widget_set_size_request(
                _self.window as *mut GtkWidget,
                min_size.width as i32,
                min_size.height as i32,
            );
        }
        gtk_window_set_resizable(_self.window, builder.resizable);
        if builder.should_center {
            gtk_window_set_position(_self.window, GTK_WIN_POS_CENTER);
        }
        #[cfg(feature = "remember_window_state")]
        if builder.remember_window_state {
            _self.load_window_state();
        }

        let display = gdk_display_get_default();
        let display_name = CStr::from_ptr(gdk_display_get_name(display)).to_string_lossy();
        if !display_name.contains("wayland") {
            g_signal_connect_data(
                _self.window as *mut c_void,
                c"configure-event".as_ptr(),
                window_on_move as *const c_void,
                _self as *mut Webview as *const c_void,
                null(),
                G_CONNECT_DEFAULT,
            );
        }
        g_signal_connect_data(
            _self.window as *mut c_void,
            c"size-allocate".as_ptr(),
            window_on_resize as *const c_void,
            _self as *mut Webview as *const c_void,
            null(),
            G_CONNECT_DEFAULT,
        );
        g_signal_connect_data(
            _self.window as *mut c_void,
            c"delete-event".as_ptr(),
            window_on_close as *const c_void,
            _self as *mut Webview as *const c_void,
            null(),
            G_CONNECT_DEFAULT,
        );
    }

    // Create webview
    unsafe {
        if cfg!(feature = "ipc") {
            let user_content_controller = webkit_user_content_manager_new();
            let user_script = webkit_user_script_new(
                c"window.ipc = new EventTarget();\
                        window.ipc.postMessage = message => window.webkit.messageHandlers.ipc.postMessage(typeof message !== 'string' ? JSON.stringify(message) : message);\
                        console.log = message => window.webkit.messageHandlers.console.postMessage(typeof message !== 'string' ? JSON.stringify(message) : message);".as_ptr(),
                WEBKIT_USER_CONTENT_INJECT_TOP_FRAME,
                WEBKIT_USER_SCRIPT_INJECT_AT_DOCUMENT_START,
                null(),
                null(),
            );
            webkit_user_content_manager_add_script(user_content_controller, user_script);
            g_signal_connect_data(
                user_content_controller as *mut c_void,
                c"script-message-received::ipc".as_ptr(),
                webview_on_message_ipc as *const c_void,
                _self as *mut Webview as *const c_void,
                null(),
                G_CONNECT_DEFAULT,
            );
            g_signal_connect_data(
                user_content_controller as *mut c_void,
                c"script-message-received::console".as_ptr(),
                webview_on_message_console as *const c_void,
                _self as *mut Webview as *const c_void,
                null(),
                G_CONNECT_DEFAULT,
            );
            webkit_user_content_manager_register_script_message_handler(
                user_content_controller,
                c"ipc".as_ptr(),
            );
            webkit_user_content_manager_register_script_message_handler(
                user_content_controller,
                c"console".as_ptr(),
            );
            _self.webview = webkit_web_view_new_with_user_content_manager(user_content_controller);
        }
        if cfg!(not(feature = "ipc")) {
            _self.webview = webkit_web_view_new();
        }
        gtk_container_add(
            _self.window as *mut GtkWidget,
            _self.webview as *mut GtkWidget,
        );
        if let Some(should_load_url) = builder.should_load_url {
            let url = CString::new(should_load_url).expect("Can't convert to CString");
            webkit_web_view_load_uri(_self.webview, url.as_ptr());
        }
        if let Some(should_load_html) = builder.should_load_html {
            let html = CString::new(should_load_html).expect("Can't convert to CString");
            webkit_web_view_load_html(_self.webview, html.as_ptr(), null());
        }
        g_signal_connect_data(
            _self.webview as *mut c_void,
            c"load-changed".as_ptr(),
            webview_on_load_changed as *const c_void,
            _self as *mut Webview as *const c_void,
            null(),
            G_CONNECT_DEFAULT,
        );
        g_signal_connect_data(
            _self.webview as *mut c_void,
            c"decide-policy".as_ptr(),
            webview_on_navigation_policy_decision as *const c_void,
            _self as *mut Webview as *const c_void,
            null(),
            G_CONNECT_DEFAULT,
        );
    }

    // Show window
    unsafe { gtk_widget_show_all(_self.window) };

    // Send window created event
    _self.send_event(Event::WindowCreated);
}

extern "C" fn window_on_move(
    _window: *mut GtkWindow,
    _allocation: *mut c_void,
    _self: &mut Webview,
) -> bool {
    let mut x = 0;
    let mut y = 0;
    unsafe { gtk_window_get_position(_self.window, &mut x, &mut y) };
    _self.send_event(Event::WindowMoved(LogicalPoint::new(x as f32, y as f32)));
    false
}

extern "C" fn window_on_resize(
    _window: *mut GtkWindow,
    _allocation: *mut c_void,
    _self: &mut Webview,
) {
    let mut width = 0;
    let mut height = 0;
    unsafe { gtk_window_get_size(_self.window, &mut width, &mut height) };
    _self.send_event(Event::WindowResized(LogicalSize::new(
        width as f32,
        height as f32,
    )));
}

extern "C" fn window_on_close(
    _window: *mut GtkWindow,
    _event: *mut c_void,
    _self: &mut Webview,
) -> bool {
    // Save window state
    #[cfg(feature = "remember_window_state")]
    if _self.remember_window_state {
        _self.save_window_state();
    }

    // Send window closed event
    _self.send_event(Event::WindowClosed);
    false
}

extern "C" fn webview_on_load_changed(
    _webview: *mut WebKitWebView,
    event: i32,
    _self: &mut Webview,
) {
    if event == WEBKIT_LOAD_STARTED {
        _self.send_event(Event::PageLoadStarted)
    }
    if event == WEBKIT_LOAD_FINISHED {
        _self.send_event(Event::PageLoadFinished)
    }
}

extern "C" fn webview_on_navigation_policy_decision(
    _webview: *mut WebKitWebView,
    decision: *mut WebKitNavigationPolicyDecision,
    decision_type: i32,
    _self: &mut Webview,
) -> bool {
    if decision_type == WEBKIT_POLICY_DECISION_TYPE_NEW_WINDOW_ACTION {
        let request = unsafe { webkit_navigation_policy_decision_get_request(decision) };
        let uri = unsafe { webkit_uri_request_get_uri(request) };
        unsafe { gtk_show_uri_on_window(null_mut(), uri, 0, null_mut()) };
        return true;
    }
    false
}

#[cfg(feature = "ipc")]
extern "C" fn webview_on_message_ipc(
    _manager: *mut WebKitUserContentManager,
    _message: *mut WebKitJavascriptResult,
    _self: &mut Webview,
) {
    let message = unsafe { webkit_javascript_result_get_js_value(_message) };
    let message = unsafe { jsc_value_to_string(message) };
    let message = unsafe { CStr::from_ptr(message) }.to_string_lossy();
    _self.send_event(Event::PageMessageReceived(message.to_string()));
}

#[cfg(feature = "ipc")]
extern "C" fn webview_on_message_console(
    _manager: *mut WebKitUserContentManager,
    _message: *mut WebKitJavascriptResult,
    _self: &mut Webview,
) {
    let message = unsafe { webkit_javascript_result_get_js_value(_message) };
    let message = unsafe { jsc_value_to_string(message) };
    let message = unsafe { CStr::from_ptr(message) }.to_string_lossy();
    println!("{}", message);
}
