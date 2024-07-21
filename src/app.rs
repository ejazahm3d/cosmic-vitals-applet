// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use cosmic::app::{Command, Core};
use cosmic::iced::wayland::popup::{destroy_popup, get_popup};
use cosmic::iced::window::Id;
use cosmic::iced::{time, Alignment, Limits, Subscription};
use cosmic::iced_style::application;
use cosmic::widget::{self, settings};
use cosmic::{Application, Element, Theme};

use crate::fl;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WatcherType {
    Ram,
    Disk(String),
    MaxTemp,
}

#[derive(Debug, Clone)]
pub struct Watcher {
    pub watcher_type: WatcherType,
    pub show: bool,
    pub label: String,
}

/// This is the struct that represents your application.
/// It is used to define the data that will be used by your application.
#[derive(Default)]
pub struct YourApp {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// The popup id.
    popup: Option<Id>,
    watchers: Vec<Watcher>,
}

fn to_gb(bytes: u64) -> f64 {
    bytes as f64 / 1000.0 / 1000.0 / 1000.0
}

fn get_ram_usage() -> String {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let ram_usage_text = format!("RAM {:.2} GB", to_gb(system.used_memory()));

    ram_usage_text
}

fn get_storage_usage(name: String) -> String {
    let mut disks = sysinfo::Disks::new();
    disks.refresh_list();
    let mut storage_usage_text = String::from("");
    for disk in &mut disks {
        if disk.name().eq(name.as_str()) {
            storage_usage_text = format!("Disk {:.2} GB", to_gb(disk.available_space()));
        }
    }
    storage_usage_text
}

fn get_disks() -> Vec<(String, String)> {
    let mut disks = sysinfo::Disks::new();
    disks.refresh_list();

    let mut disk_availables: HashMap<String, String> = HashMap::new();

    for disk in &mut disks {
        disk_availables.insert(
            disk.name().to_str().unwrap().to_string(),
            to_gb(disk.available_space()).to_string(),
        );
    }

    let mut disk_availables: Vec<(String, String)> = disk_availables
        .into_iter()
        .map(|(x, y)| (x.clone(), y.clone()))
        .collect();

    disk_availables.sort_by(|a, b| a.1.cmp(&b.1));

    disk_availables
}

fn get_max_temp() -> String {
    let mut components = sysinfo::Components::new();
    components.refresh_list();

    let max_temp = components
        .iter()
        .map(|x| x.temperature() as u32)
        .max()
        .unwrap_or(0);

    format!("Max temp: {}°C", max_temp)
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    ToggleWatcher(Watcher),
    Tick,
}

impl Application for YourApp {
    type Executor = cosmic::executor::Default;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "com.example.CosmicVitalsApplet";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let app = YourApp {
            core,
            ..Default::default()
        };

        (app, Command::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<Self::Message> {
        let mut children = vec![];

        for watcher in &self.watchers {
            if watcher.show {
                children.push(Element::from(widget::button(widget::text(
                    watcher.label.clone(),
                ))));
            }
        }

        let content_list = widget::row::with_children(children)
            .spacing(5)
            .push(
                widget::button(widget::icon::from_name("display-symbolic"))
                    .on_press(Message::TogglePopup),
            )
            .align_items(Alignment::Center);

        content_list.into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        time::every(std::time::Duration::from_secs(5)).map(|_| Message::Tick)
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        let is_ram_checked = match self
            .watchers
            .iter()
            .find(|x| x.watcher_type == WatcherType::Ram)
        {
            Some(w) => w.show,
            None => false,
        };

        let is_max_temp_checked = match self
            .watchers
            .iter()
            .find(|x| x.watcher_type == WatcherType::MaxTemp)
        {
            Some(w) => w.show,
            None => false,
        };

        let mut disks_children = vec![];

        let disks = get_disks();

        for (name, space_available) in disks {
            let is_storage_checked = match self
                .watchers
                .iter()
                .find(|x| x.watcher_type == WatcherType::Disk(name.clone()))
            {
                Some(w) => w.show,
                None => false,
            };

            let formatted_name = format!(
                "{} - ({:.2} GB)",
                name,
                space_available.parse::<f64>().unwrap()
            );

            let item = Element::from(widget::settings::item(
                formatted_name,
                widget::toggler(None, is_storage_checked, move |value| {
                    Message::ToggleWatcher(Watcher {
                        watcher_type: WatcherType::Disk(name.clone()),
                        show: value,
                        label: format!("{} - {}", name, space_available),
                    })
                }),
            ));
            disks_children.push(item);
        }

        let disks_list = widget::column::with_children::<Self::Message>(disks_children).spacing(5);

        let content_list = widget::list_column()
            .padding(5)
            .add(settings::item(
                fl!("max-temp"),
                widget::toggler(None, is_max_temp_checked, |value| {
                    Message::ToggleWatcher(Watcher {
                        watcher_type: WatcherType::MaxTemp,
                        show: value,
                        label: "".into(),
                    })
                }),
            ))
            .spacing(5)
            .add(settings::item(
                fl!("ram-usage"),
                widget::toggler(None, is_ram_checked, |value| {
                    Message::ToggleWatcher(Watcher {
                        watcher_type: WatcherType::Ram,
                        show: value,
                        label: "".into(),
                    })
                }),
            ))
            .spacing(5)
            .add(settings::item(fl!("disk-usage"), widget::text("")))
            .add(disks_list);

        self.core.applet.popup_container(content_list).into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings =
                        self.core
                            .applet
                            .get_popup_settings(Id::MAIN, new_id, None, None, None);
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(372.0)
                        .min_width(300.0)
                        .min_height(200.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
                }
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::ToggleWatcher(mut watcher) => {
                if watcher.show {
                    watcher.label = match watcher.watcher_type {
                        WatcherType::Ram => get_ram_usage(),
                        WatcherType::Disk(ref name) => get_storage_usage(name.clone()),
                        WatcherType::MaxTemp => get_max_temp(),
                    };

                    self.watchers.push(watcher);
                } else {
                    self.watchers
                        .retain(|x| x.watcher_type != watcher.watcher_type);
                }
            }
            Message::Tick => {
                for watcher in &mut self.watchers {
                    watcher.label = match watcher.watcher_type {
                        WatcherType::Ram => {
                            let ram_text = get_ram_usage();
                            ram_text.to_owned()
                        }
                        WatcherType::Disk(ref name) => get_storage_usage(name.clone()),
                        WatcherType::MaxTemp => get_max_temp(),
                    }
                }
            }
        }
        Command::none()
    }

    fn style(&self) -> Option<<Theme as application::StyleSheet>::Style> {
        Some(cosmic::applet::style())
    }
}
