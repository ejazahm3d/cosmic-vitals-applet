// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use cosmic::app::{Command, Core};
use cosmic::applet::menu_button;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::wayland::popup::{destroy_popup, get_popup};
use cosmic::iced::window::Id;
use cosmic::iced::{time, Alignment, Length, Limits, Subscription};
use cosmic::iced_core::text::Wrap;
use cosmic::iced_style::application;
use cosmic::iced_widget::row;
use cosmic::widget::settings::item_row;
use cosmic::widget::{
    button, column, container, horizontal_space, icon, list_column, row as row_mod,
};
use cosmic::widget::{text, toggler};
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
    ram_stat_toggle: bool,
    disk_stat_toggle: bool,
    temp_stat_toggle: bool,
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
            format!("{:.2}", to_gb(disk.available_space())),
        );
    }

    let mut disk_availables: Vec<(String, String)> = disk_availables
        .into_iter()
        .map(|(x, y)| (x.clone(), y.clone()))
        .collect();

    disk_availables.sort_by(|a, b| a.0.cmp(&b.0));

    disk_availables
}

fn get_temps() -> Vec<(String, String)> {
    let mut components = sysinfo::Components::new();
    components.refresh_list();

    let mut temps = components
        .iter()
        .map(|x| (x.label().to_string(), format!("{}", x.temperature() as u32)))
        .collect::<Vec<(String, String)>>();

    let max_temp = components.iter().map(|x| x.temperature() as u32).max();

    let min_temp = components.iter().map(|x| x.temperature() as u32).min();

    temps.sort_by(|a, b| a.0.cmp(&b.0));

    temps.push((fl!("max-temp"), format!("{}", max_temp.unwrap_or(0))));
    temps.push((fl!("min-temp"), format!("{}", min_temp.unwrap_or(0))));

    temps
}

fn get_temp_usage(name: &str) -> String {
    for (temp_name, temp) in get_temps() {
        if name == temp_name {
            return format!("Temp {} °C", temp);
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
    RamStatsToggle(bool),
    DiskStatsToggle(bool),
    TempStatsToggle(bool),
}

impl YourApp {
    fn stat_list(
        &self,
        stats: Vec<(String, String)>,
        fn_name: impl Fn(String) -> StatType,
    ) -> Vec<Element<'_, Message>> {
        let mut children = vec![];

        for (name, value) in stats {
            let stat_type = fn_name(name.clone());
            let is_checked = match self.stats.iter().find(|x| x.stat_type == stat_type.clone()) {
                Some(w) => w.show,
                None => false,
            };

            let formatted_value = match stat_type {
                StatType::Ram(_) => format!("({} GB)", value),
                StatType::Disk(_) => format!("({} GB)", value),
                StatType::MaxTemp(_) => format!("({} °C)", value),
            };

            let item = item_row(vec![
                text(name.clone()).wrap(Wrap::Word).width(125).into(),
                horizontal_space(Length::Fill).into(),
                text(formatted_value)
                    .wrap(Wrap::Word)
                    .horizontal_alignment(Horizontal::Left)
                    .into(),
                toggler(None, is_checked, move |value| {
                    Message::ToggleStat(Stat {
                        stat_type: stat_type.clone(),
                        show: value,
                        label: format!("{} - {}", name, value),
                    })
                })
                .into(),
            ])
            .into();

            children.push(item);
        }

        children
    }

    fn dropdown_menu_button(
        &self,
        stat_toggle: bool,
        text_str: String,
        on_press: Message,
    ) -> button::Button<'_, Message> {
        let dropdown_icon = if stat_toggle {
            "go-down-symbolic"
        } else {
            "go-next-symbolic"
        };

        menu_button(row![
            text(text_str)
                .size(14)
                .width(Length::Fill)
                .height(Length::Fixed(24.0))
                .vertical_alignment(Vertical::Center),
            container(icon::from_name(dropdown_icon).size(14).symbolic(true))
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .width(Length::Fixed(24.0))
                .height(Length::Fixed(24.0))
        ])
        .on_press(on_press)
    }
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
                children.push(Element::from(button(text(stat.label.clone()))));
            }
        }

        let content_list = row_mod::with_children(children)
            .spacing(5)
            .push(button(icon::from_name("display-symbolic")).on_press(Message::TogglePopup))
            .align_items(Alignment::Center);

        content_list.into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        time::every(std::time::Duration::from_secs(5)).map(|_| Message::Tick)
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        let ram_list =
            column::with_children(self.stat_list(get_ram_stats(), StatType::Ram)).spacing(5);

        let disks_list =
            column::with_children(self.stat_list(get_disks(), StatType::Disk)).spacing(5);

        let temp_list =
            column::with_children(self.stat_list(get_temps(), StatType::MaxTemp)).spacing(5);

        let mut content_list = list_column().add(self.dropdown_menu_button(
            self.temp_stat_toggle,
            fl!("temp-usage"),
            Message::TempStatsToggle(!self.temp_stat_toggle),
        ));

        if self.temp_stat_toggle {
            content_list = content_list.add(temp_list);
        }

        content_list = content_list.add(self.dropdown_menu_button(
            self.disk_stat_toggle,
            fl!("disk-usage"),
            Message::DiskStatsToggle(!self.disk_stat_toggle),
        ));

        if self.disk_stat_toggle {
            content_list = content_list.add(disks_list);
        }

        content_list = content_list.add(
            self.dropdown_menu_button(
                self.ram_stat_toggle,
                fl!("ram-usage"),
                Message::TempStatsToggle(!self.temp_stat_toggle),
            )
            .on_press(Message::RamStatsToggle(!self.ram_stat_toggle)),
        );

        if self.ram_stat_toggle {
            content_list = content_list.add(ram_list);
        }

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
                        StatType::Ram(ref name) => get_ram_usage(name),
                        StatType::Disk(ref name) => get_storage_usage(name),
                        StatType::MaxTemp(ref name) => get_temp_usage(name),
                    }
                }
            }
            Message::RamStatsToggle(toggle) => self.ram_stat_toggle = toggle,
            Message::DiskStatsToggle(toggle) => self.disk_stat_toggle = toggle,
            Message::TempStatsToggle(toggle) => self.temp_stat_toggle = toggle,
        }
        Command::none()
    }

    fn style(&self) -> Option<<Theme as application::StyleSheet>::Style> {
        Some(cosmic::applet::style())
    }
}
