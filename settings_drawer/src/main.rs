use gtk::{
    gdk, gio, glib,
    prelude::{BoxExt, FlowBoxChildExt, GridExt, GtkWindowExt},
};
use relm4::{
    factory::FactoryVecDeque, gtk, prelude::FactoryComponent, ComponentParts, ComponentSender,
    RelmApp, RelmWidgetExt, WidgetTemplate,
};
use relm4::{
    gtk::{
        glib::clone,
        prelude::{EditableExt, EditableExtManual, EntryExt, ObjectExt},
    },
    RelmRemoveAllExt,
};

use relm4::{async_trait::async_trait, SimpleComponent};
use relm4::{
    component::{AsyncComponent, AsyncComponentParts},
    AsyncComponentSender,
};
mod settings;
mod theme;
mod widgets;
use serde::de;
use tracing::{error, info};
use widgets::basic_widget::{
    BasicWidget, BasicWidgetSettings, BasicWidgetType, MessageInput as BasicWidgetMessageInput,
    MessageOutput as BasicWidgetMessageOutput,
};
pub mod errors;

mod modules;

use modules::{
    battery::handler::BatteryServiceHandle, device_info::handler::DeviceInfoServiceHandle,
};

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

#[derive(Default, Debug, Clone, Copy)]
pub enum BatteryState {
    #[default]
    Level0,
    Level10,
    Level20,
    Level30,
    Level40,
    Level50,
    Level60,
    Level70,
    Level80,
    Level90,
    Level100,
    NotFound,
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
    battery_capacity: u32,
    cpu_usage: f32,
    memory_usage: f64,
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
    BatteryStatusUpdate(BatteryState),
    CpuUsgaeStatusChanged(f32),
    MemoryUsageStatusChanged(f64),
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

#[async_trait(?Send)]
impl AsyncComponent for SettingsDrawer {
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

    type CommandOutput = Message;

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
    async fn init(
        _: Self::Init,
        window: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
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
        let cpu_usage: f32 = 0.0;
        let memory_usage: f64 = 0.0;

        let current_battery_capacity = gtk::Label::builder()
            .label(&battery_capacity.to_string())
            .build();

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
            cpu_usage,
            memory_usage,
        };

        let widgets = AppWidgets {
            current_battery_capacity,
        };

        let sender: relm4::Sender<Message> = sender.input_sender().clone();
        init_services(settings, sender).await;

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        info!("Update message is {:?}", message);
        match message {
            Message::WidgetClicked(index, widget) => self.setting_actions.send(
                index,
                BasicWidgetMessageInput::TitleChanged("Connected".to_string()),
            ),
            Message::BatteryCapacityChanged(capacity) => {
                info!("Battery capacity is {:?}", capacity);
                self.battery_capacity = capacity;
                BasicWidgetMessageInput::ValueChanged(capacity.to_string().parse::<i8>().ok());
            }
            Message::CpuUsgaeStatusChanged(status) => {
                info!("Cpu status is {:?}", status);
                // self.setting_actions.send(6, status);
            }
            Message::MemoryUsageStatusChanged(status) => {
                info!("Memory status is {:?}", status);
                // self.setting_actions.send(7, status);
            }

            _ => {}
        }
    }

    /// Update the view to represent the updated model.
    fn update_view(&self, widgets: &mut Self::Widgets, _sender: AsyncComponentSender<Self>) {
        // update the view for battery capacity
        widgets
            .current_battery_capacity
            .set_label(&self.battery_capacity.to_string());

        match self.battery_capacity {
            0..=10 => {
                self.setting_actions.send(
                    2,
                    BasicWidgetMessageInput::IconChanged(
                        self.settings.modules.battery.icon.level_0.to_owned(),
                    ),
                );
            }
            11..=20 => {
                self.setting_actions.send(
                    2,
                    BasicWidgetMessageInput::IconChanged(
                        self.settings.modules.battery.icon.level_10.to_owned(),
                    ),
                );
            }
            21..=30 => {
                self.setting_actions.send(
                    2,
                    BasicWidgetMessageInput::IconChanged(
                        self.settings.modules.battery.icon.level_20.to_owned(),
                    ),
                );
            }
            31..=40 => {
                self.setting_actions.send(
                    2,
                    BasicWidgetMessageInput::IconChanged(
                        self.settings.modules.battery.icon.level_30.to_owned(),
                    ),
                );
            }
            41..=50 => {
                self.setting_actions.send(
                    2,
                    BasicWidgetMessageInput::IconChanged(
                        self.settings.modules.battery.icon.level_40.to_owned(),
                    ),
                );
            }
            51..=60 => {
                self.setting_actions.send(
                    2,
                    BasicWidgetMessageInput::IconChanged(
                        self.settings.modules.battery.icon.level_50.to_owned(),
                    ),
                );
            }
            61..=70 => {
                self.setting_actions.send(
                    2,
                    BasicWidgetMessageInput::IconChanged(
                        self.settings.modules.battery.icon.level_60.to_owned(),
                    ),
                );
            }
            71..=80 => {
                self.setting_actions.send(
                    2,
                    BasicWidgetMessageInput::IconChanged(
                        self.settings.modules.battery.icon.level_70.to_owned(),
                    ),
                );
            }
            81..=90 => {
                self.setting_actions.send(
                    2,
                    BasicWidgetMessageInput::IconChanged(
                        self.settings.modules.battery.icon.level_80.to_owned(),
                    ),
                );
            }
            91..=100 => {
                self.setting_actions.send(
                    2,
                    BasicWidgetMessageInput::IconChanged(
                        self.settings.modules.battery.icon.level_90.to_owned(),
                    ),
                );
            }
            101..=1000 => {
                self.setting_actions.send(
                    2,
                    BasicWidgetMessageInput::IconChanged(
                        self.settings.modules.battery.icon.level_100.to_owned(),
                    ),
                );
            }
            _ => {}
        }
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
    app.run_async::<SettingsDrawer>(());
}

async fn init_services(settings: SettingsDrawerSettings, sender: relm4::Sender<Message>) {
    let mut battery_service_handle = BatteryServiceHandle::new();
    let sender_clone_4 = sender.clone();
    let _ = relm4::spawn_local(async move {
        info!(task = "init_services", "Starting battery service");
        battery_service_handle.run(sender_clone_4).await;
    });

    let mut device_info_service_handle = DeviceInfoServiceHandle::new();

    let sender_clone_5 = sender.clone();
    let _ = relm4::spawn_local(async move {
        info!(task = "init_services", "Starting device info service");
        device_info_service_handle.run(sender_clone_5).await;
    });
}
