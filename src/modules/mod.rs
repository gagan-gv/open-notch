pub mod calendar;
pub mod mpris;
pub mod notes;
pub mod pomodoro;

use calendar::CalendarWidget;
use mpris::MprisWidget;
use notes::NotesWidget;
use pomodoro::PomodoroWidget;

use crate::config::Config;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Orientation, Stack, StackSwitcher};

pub struct ModulesContainer {
    pub container: GtkBox,
    #[allow(dead_code)]
    pub mpris: MprisWidget,
}

impl ModulesContainer {
    pub fn new(config: &Config) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 6);
        container.add_css_class("modules-container");

        let stack = Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
        stack.set_transition_duration(200);

        // 1. Calendar Widget (Default idle view)
        let calendar_widget = CalendarWidget::new();
        stack.add_titled(&calendar_widget.container, Some("calendar"), "📅 Calendar");

        // 2. Media Widget
        let mpris_widget = MprisWidget::new();
        stack.add_titled(&mpris_widget.container, Some("media"), "🎵 Media");

        // 3. Pomodoro Widget
        let pomodoro_widget = PomodoroWidget::new(
            config.pomodoro.work_duration_min,
            config.pomodoro.break_duration_min,
        );
        stack.add_titled(&pomodoro_widget.container, Some("pomodoro"), "⏳ Focus");

        // 4. Notes Widget
        let notes_widget = NotesWidget::new();
        stack.add_titled(&notes_widget.container, Some("notes"), "📝 Notes");

        // Switcher Bar (Tabs inside the expanded notch)
        let switcher = StackSwitcher::new();
        switcher.set_stack(Some(&stack));
        switcher.set_halign(gtk4::Align::Center);
        switcher.add_css_class("notch-switcher");

        // Default screen is the calendar when nothing is playing
        stack.set_visible_child_name("calendar");

        container.append(&switcher);
        container.append(&stack);

        Self {
            container,
            mpris: mpris_widget,
        }
    }
}
