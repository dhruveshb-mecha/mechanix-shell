use gtk::{
    gdk, gio, glib,
    prelude::{BoxExt, FlowBoxChildExt, GridExt, GtkWindowExt},
};
use relm4::{
    factory::FactoryVecDeque, gtk, prelude::FactoryComponent, ComponentParts, ComponentSender,
    RelmApp, RelmWidgetExt, SimpleComponent, WidgetTemplate,
};
use relm4::{
    gtk::{
        glib::clone,
        prelude::{EditableExt, EditableExtManual, EntryExt, ObjectExt},
    },
    RelmRemoveAllExt,
};

mod settings;
mod theme;
mod widgets;
use tracing::{error, info};
use widgets::basic_widget::{
    BasicWidget, BasicWidgetSettings, BasicWidgetType, MessageInput as BasicWidgetMessageInput,
    MessageOutput as BasicWidgetMessageOutput,
};
pub mod errors;
mod event_handler;
mod modules;

use event_handler::zbus::ZbusServiceHandle;

use modules::battery::handler::BatteryServiceHandle;

use crate::settings::SettingsDrawerSettings;
use crate::theme::SettingsDrawerTheme;

#[derive(Debug, Copy, Clone)]
pub enum WifiState {
    On,
    Off,
    Connected,
}

#[derive(Debug, Copy, Clone)]
pub enum BluetoothState {
    On,
    Off,
    Connected,
}
/// # SettingsDrawer State
///
/// This struct is the state definition of the entire application
pub struct SettingsDrawer {
    settings: SettingsDrawerSettings,
    custom_theme: SettingsDrawerTheme,
    wifi_state: WifiState,
    bluetooth_state: BluetoothState,
    setting_actions: FactoryVecDeque<BasicWidget>,
    battery_capacity: u32, // battery_action: BasicWidget,
                           // cpu_action: BasicWidget,
                           // memory_action: BasicWidget,
                           // running_apps_action: BasicWidget,
                           //wifi_action: BasicWidget,
                           // bluetooth_action: BasicWidget,
                           // auto_rotate_action: BasicWidget,
                           // settings_action: BasicWidget,
                           // sound_action: BasicWidget,
                           // brigtness_action: BasicWidget,
}

/// ## Message
///
/// These are the events (or messages) that update state.
/// Each of them are handled in the ``impl Application()::update()``
#[derive(Debug, Clone)]
pub enum Message {
    CpuStatusChanged(BasicWidgetMessageOutput),
    BatteryStatusChanged(BasicWidgetMessageOutput),
    MemoryStatusChanged(BasicWidgetMessageOutput),
    RunningAppsChanged(BasicWidgetMessageOutput),
    WifiStatusChanged(BasicWidgetMessageOutput),
    BluetoothStatusChanged(BasicWidgetMessageOutput),
    AutoRotateStatusChanged(BasicWidgetMessageOutput),
    SettingsStatusChanged(BasicWidgetMessageOutput),
    SoundStatusChanged(BasicWidgetMessageOutput),
    BrightnessStatusChanged(BasicWidgetMessageOutput),
    WidgetClicked(usize, String),
    BatteryCapacityChanged(u32),
}

pub struct AppWidgets {
    pub current_battery_capacity: gtk::Label,
}

#[cfg(not(feature = "layer-shell"))]
fn init_window(settings: SettingsDrawerSettings) -> gtk::Window {
    let window_settings = settings.window;
    let window = gtk::Window::builder()
        .title(settings.title)
        .default_width(window_settings.size.0)
        .default_height(window_settings.size.1)
        .css_classes(["window"])
        .build();
    window
}

#[cfg(feature = "layer-shell")]
fn init_window(settings: SettingsDrawerSettings) -> gtk::Window {
    let window_settings = settings.window;
    let window = gtk::Window::builder()
        .title(settings.title)
        .default_width(window_settings.size.0)
        .default_height(window_settings.size.1)
        .css_classes(["window"])
        .build();

    gtk4_layer_shell::init_for_window(&window);

    // Display above normal windows
    gtk4_layer_shell::set_layer(&window, gtk4_layer_shell::Layer::Top);

    // The margins are the gaps around the window's edges
    // Margins and anchors can be set like this...
    gtk4_layer_shell::set_margin(&window, gtk4_layer_shell::Edge::Left, 0);
    gtk4_layer_shell::set_margin(&window, gtk4_layer_shell::Edge::Right, 0);
    gtk4_layer_shell::set_margin(&window, gtk4_layer_shell::Edge::Top, 0);
    gtk4_layer_shell::set_margin(&window, gtk4_layer_shell::Edge::Bottom, 0);

    // ... or like this
    // Anchors are if the window is pinned to each edge of the output
    let anchors = [
        (gtk4_layer_shell::Edge::Left, true),
        (gtk4_layer_shell::Edge::Right, true),
        (gtk4_layer_shell::Edge::Top, true),
        (gtk4_layer_shell::Edge::Bottom, true),
    ];

    for (anchor, state) in anchors {
        gtk4_layer_shell::set_anchor(&window, anchor, state);
    }

    window
}

impl SimpleComponent for SettingsDrawer {
    /// The type of the messages that this component can receive.
    type Input = Message;
    /// The type of the messages that this component can send.
    type Output = ();
    /// The type of data with which this component will be initialized.
    type Init = ();
    /// The root GTK widget that this component will create.
    type Root = gtk::Window;
    /// A data structure that contains the widgets that you will need to update.
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        let settings = match settings::read_settings_yml() {
            Ok(settings) => settings,
            Err(_) => SettingsDrawerSettings::default(),
        };

        info!(
            task = "init_settings",
            "settings initialized for app drawer {:?}", settings
        );

        let custom_theme = match theme::read_theme_yml() {
            Ok(theme) => theme,
            Err(_) => SettingsDrawerTheme::default(),
        };

        info!(
            task = "init_theme",
            "theme initialized for app drawer {:?}", custom_theme
        );

        let window = init_window(settings);
        window
    }

    /// Initialize the UI and model.
    fn init(
        _: Self::Init,
        window: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let settings = match settings::read_settings_yml() {
            Ok(settings) => settings,
            Err(_) => SettingsDrawerSettings::default(),
        };

        let css = settings.css.clone();
        relm4::set_global_css_from_file(css.default);

        let custom_theme = match theme::read_theme_yml() {
            Ok(theme) => theme,
            Err(_) => SettingsDrawerTheme::default(),
        };

        let modules = settings.modules.clone();

        let mut setting_actions: FactoryVecDeque<BasicWidget> = FactoryVecDeque::builder()
            .launch(
                gtk::FlowBox::builder()
                    .valign(gtk::Align::Start)
                    .max_children_per_line(30)
                    .min_children_per_line(4)
                    .selection_mode(gtk::SelectionMode::None)
                    .row_spacing(7)
                    .column_spacing(6)
                    .homogeneous(true)
                    .build(),
            )
            .forward(sender.input_sender(), |msg| match msg {
                BasicWidgetMessageOutput::WidgetClicked(index, widget) => {
                    Message::WidgetClicked(index, widget)
                }
            });

        let layout = settings.layout.clone();

        let battery_capacity: u32 = 0;

        layout.grid.into_iter().for_each(|key| {
            let mut widget_settings = BasicWidgetSettings::default();

            if key == modules.wifi.title {
                widget_settings = BasicWidgetSettings {
                    title: modules.wifi.title.to_owned(),
                    icon: modules.wifi.icon.strong.to_owned(),
                    ..Default::default()
                }
            } else if key == modules.bluetooth.title {
                widget_settings = BasicWidgetSettings {
                    title: modules.bluetooth.title.to_owned(),
                    icon: modules.bluetooth.icon.on.to_owned(),
                    ..Default::default()
                }
            } else if key == modules.battery.title {
                widget_settings = BasicWidgetSettings {
                    title: modules.battery.title.to_owned(),
                    icon: modules.battery.icon.level_60.to_owned(),
                    value: battery_capacity.to_owned().to_string().parse::<i8>().ok(),
                    value_subscript: Option::from("%".to_string()),
                    ..Default::default()
                }
            } else if key == modules.auto_rotate.title {
                widget_settings = BasicWidgetSettings {
                    title: modules.auto_rotate.title.to_owned(),
                    icon: modules.auto_rotate.icon.portrait.to_owned(),
                    ..Default::default()
                }
            } else if key == modules.settings.title {
                widget_settings = BasicWidgetSettings {
                    title: modules.settings.title.to_owned(),
                    icon: modules.settings.icon.default.to_owned(),
                    ..Default::default()
                }
            } else if key == modules.running_apps.title {
                widget_settings = BasicWidgetSettings {
                    title: modules.running_apps.title.to_owned(),
                    icon: modules.running_apps.icon.medium.to_owned(),
                    value: Option::from(7),
                    ..Default::default()
                }
            } else if key == modules.cpu.title {
                widget_settings = BasicWidgetSettings {
                    title: modules.cpu.title.to_owned(),
                    icon: modules.cpu.icon.medium.to_owned(),
                    value: Option::from(65),
                    value_subscript: Option::from("%".to_string()),
                    ..Default::default()
                }
            } else if key == modules.memory.title {
                widget_settings = BasicWidgetSettings {
                    title: modules.memory.title.to_owned(),
                    icon: modules.memory.icon.medium.to_owned(),
                    value: Option::from(75),
                    value_subscript: Option::from("%".to_string()),
                    ..Default::default()
                }
            } else if key == modules.sound.title {
                widget_settings = BasicWidgetSettings {
                    title: modules.sound.title.to_owned(),
                    icon: modules.sound.icon.medium.to_owned(),
                    widget_type: BasicWidgetType::Slider,
                    ..Default::default()
                }
            } else if key == modules.brightness.title {
                widget_settings = BasicWidgetSettings {
                    title: modules.brightness.title.to_owned(),
                    icon: modules.brightness.icon.medium.to_owned(),
                    //widget_type: BasicWidgetType::Slider,
                    ..Default::default()
                }
            }
            setting_actions.guard().push_back(widget_settings);
        });

        let container_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(["container"])
            .build();

        let settings_actions_widget = setting_actions.widget();

        container_box.append(settings_actions_widget);

        window.set_child(Some(&container_box));

        let model = SettingsDrawer {
            settings: settings.clone(),
            custom_theme,
            wifi_state: WifiState::Off,
            bluetooth_state: BluetoothState::Off,
            setting_actions,
            battery_capacity,
        };

        let widgets = AppWidgets {
            current_battery_capacity: gtk::Label::builder()
                .label(&model.battery_capacity.to_string())
                .build(),
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        info!("Update message is {:?}", message);
        match message {
            Message::WidgetClicked(index, widget) => self.setting_actions.send(
                index,
                BasicWidgetMessageInput::TitleChanged("Connected".to_string()),
            ),
            Message::BatteryCapacityChanged(capacity) => {
                info!("Battery capacity is {:?}", capacity);
                self.battery_capacity = capacity;
            }
            _ => {}
        }
    }

    /// Update the view to represent the updated model.
    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        // widgets.apps_grid.remove_all();
        // self.settings
        //     .clone()
        //     .modules
        //     .apps
        //     .into_iter()
        //     .filter(|app| app.name.to_lowercase().starts_with(&self.search_text))
        //     .for_each(|app| {
        //         let widget = generate_apps_ui(app);
        //         widgets.apps_grid.insert(&widget, -1);
        //     });
        widgets
            .current_battery_capacity
            .set_text(&self.battery_capacity.to_string());
    }
}

/// Initialize the application with settings, and starts
fn main() {
    // Enables logger
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter("mecha_settings_drawer=trace")
        .with_thread_names(true)
        .init();

    let app = RelmApp::new("app.drawer").with_args(vec![]);
    app.run::<SettingsDrawer>(());
}

async fn init_services(settings: SettingsDrawerSettings, sender: relm4::Sender<Message>) {
    let mut zbus_service_handle = ZbusServiceHandle::new();
    let sender_clone_1 = sender.clone();
    let _ = relm4::spawn_local(async move {
        info!(task = "init_services", "Starting zbus service");
        zbus_service_handle.run(sender_clone_1).await;
    });

    let mut battery_service_handle = BatteryServiceHandle::new();
    let sender_clone_4 = sender.clone();
    let _ = relm4::spawn_local(async move {
        info!(task = "init_services", "Starting battery service");
        battery_service_handle.run(sender_clone_4).await;
    });
}
