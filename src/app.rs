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
pub enum StatType {
    Ram(String),
    Disk(String),
    MaxTemp(String),
}

#[derive(Debug, Clone)]
pub struct Stat {
    pub stat_type: StatType,
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
    stats: Vec<Stat>,
}

fn to_gb(bytes: u64) -> f64 {
    bytes as f64 / 1000.0 / 1000.0 / 1000.0
}

fn get_ram_usage(name: &str) -> String {
    let mut ram_usage_text = String::from("");

    for (ram_name, ram) in get_ram_stats() {
        if name == ram_name {
            ram_usage_text = format!("RAM {} GB", ram);
        }
    }

    ram_usage_text
}

fn get_storage_usage(name: &str) -> String {
    let mut disks = sysinfo::Disks::new();
    disks.refresh_list();
    let mut storage_usage_text = String::from("");
    for disk in &mut disks {
        if disk.name().eq(name) {
            storage_usage_text = format!("Disk {:.2} GB", to_gb(disk.available_space()));
        }
    }
    storage_usage_text
}

fn get_ram_stats() -> Vec<(String, String)> {
    let mut ram_stats = vec![];
    let mut system = sysinfo::System::new();
    system.refresh_memory();

    ram_stats.push((
        fl!("total-ram"),
        format!("{:.2} GB", to_gb(system.total_memory())),
    ));

    ram_stats.push((
        fl!("used-ram"),
        format!("{:.2} GB", to_gb(system.used_memory())),
    ));

    ram_stats.push((
        fl!("free-ram"),
        format!("{:.2} GB", to_gb(system.free_memory())),
    ));

    ram_stats.push((
        fl!("available-ram"),
        format!("{:.2} GB", to_gb(system.available_memory())),
    ));

    ram_stats.push((
        fl!("free-swap"),
        format!("{:.2} GB", to_gb(system.free_swap())),
    ));

    ram_stats.push((
        fl!("total-swap"),
        format!("{:.2} GB", to_gb(system.total_swap())),
    ));

    ram_stats.push((
        fl!("used-swap"),
        format!("{:.2} GB", to_gb(system.used_swap())),
    ));

    ram_stats
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

fn get_temps() -> Vec<(String, String)> {
    let mut components = sysinfo::Components::new();
    components.refresh_list();

    let temps = components
        .iter()
        .map(|x| (x.label().to_string(), x.temperature().to_string()))
        .collect();

    temps
}

fn get_temp_usage(name: &str) -> String {
    for (temp_name, temp) in get_temps() {
        if name == temp_name {
            return temp;
        }
    }

    String::from("")
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    ToggleStat(Stat),
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

        for stat in &self.stats {
            if stat.show {
                children.push(Element::from(widget::button(widget::text(
                    stat.label.clone(),
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
        let mut ram_children = vec![];

        for (name, ram) in get_ram_stats() {
            let is_ram_checked = match self
                .stats
                .iter()
                .find(|x| x.stat_type == StatType::Ram(name.clone()))
            {
                Some(w) => w.show,
                None => false,
            };

            let formatted_name = format!("{} - ({} GB)", name, ram);

            let item = Element::from(widget::settings::item(
                formatted_name,
                widget::toggler(None, is_ram_checked, move |value| {
                    Message::ToggleStat(Stat {
                        stat_type: StatType::Ram(name.clone()),
                        show: value,
                        label: format!("{} - {}", name, ram),
                    })
                }),
            ));
            ram_children.push(item);
        }

        let ram_list = widget::column::with_children(ram_children)
            .padding(5)
            .spacing(5);

        let mut disks_children = vec![];

        for (name, space_available) in get_disks() {
            let is_storage_checked = match self
                .stats
                .iter()
                .find(|x| x.stat_type == StatType::Disk(name.clone()))
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
                    Message::ToggleStat(Stat {
                        stat_type: StatType::Disk(name.clone()),
                        show: value,
                        label: format!("{} - {}", name, space_available),
                    })
                }),
            ));
            disks_children.push(item);
        }

        let disks_list = widget::column::with_children::<Self::Message>(disks_children).spacing(5);

        let mut temp_children = vec![];

        for (name, temp) in get_temps() {
            let is_temp_checked = match self
                .stats
                .iter()
                .find(|x| x.stat_type == StatType::MaxTemp(name.clone()))
            {
                Some(w) => w.show,
                None => false,
            };

            let formatted_name = format!("{} - ({}°C)", name.clone(), temp);

            let item = Element::from(widget::settings::item(
                formatted_name,
                widget::toggler(None, is_temp_checked, move |value| {
                    Message::ToggleStat(Stat {
                        stat_type: StatType::MaxTemp(name.clone()),
                        show: value,
                        label: "".into(),
                    })
                }),
            ));

            temp_children.push(item);
        }

        let temp_list = widget::column::with_children(temp_children).spacing(5);

        let content_list = widget::list_column()
            .padding(5)
            .add(settings::item(fl!("max-temp"), widget::text("")))
            .spacing(5)
            .add(temp_list)
            .add(settings::item(fl!("ram-usage"), widget::text("")))
            .add(ram_list)
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
            Message::ToggleStat(mut stat) => {
                if stat.show {
                    stat.label = match stat.stat_type {
                        StatType::Ram(ref name) => get_ram_usage(name),
                        StatType::Disk(ref name) => get_storage_usage(name),
                        StatType::MaxTemp(ref name) => get_temp_usage(name),
                    };

                    self.stats.push(stat);
                } else {
                    self.stats.retain(|x| x.stat_type != stat.stat_type);
                }
            }
            Message::Tick => {
                for stat in &mut self.stats {
                    stat.label = match stat.stat_type {
                        StatType::Ram(ref name) => {
                            let ram_text = get_ram_usage(name);
                            ram_text.to_owned()
                        }
                        StatType::Disk(ref name) => get_storage_usage(name),
                        StatType::MaxTemp(ref name) => get_temp_usage(name),
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
