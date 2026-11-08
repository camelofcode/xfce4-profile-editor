// SPDX-License-Identifier: GPL-2.0-or-later

use gettextrs::gettext;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::core::{LoadWarning, LoadedProfile, Profile, ProfileStore};

mod imp {
    use super::*;
    use glib::subclass::InitializingObject;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/camelofcode/Xfce4ProfileEditor/ui/windows/profile-window.ui")]
    pub struct ProfileWindow {
        #[template_child]
        pub avatar_picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub close_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub email_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub first_name_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub home_phone_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub last_name_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub loading_indicator: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub office_phone_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub status_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProfileWindow {
        const NAME: &'static str = "Xfce4ProfileEditorWindow";
        type Type = super::ProfileWindow;
        type ParentType = gtk::ApplicationWindow;

        fn class_init(class: &mut Self::Class) {
            class.bind_template();
        }

        fn instance_init(instance: &InitializingObject<Self>) {
            instance.init_template();
        }
    }

    impl ObjectImpl for ProfileWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let window = self.obj();
            let weak_window = window.downgrade();
            self.close_button.connect_clicked(move |_| {
                if let Some(window) = weak_window.upgrade() {
                    window.close();
                }
            });
            window.load_profile();
        }
    }
    impl WidgetImpl for ProfileWindow {}
    impl WindowImpl for ProfileWindow {}
    impl ApplicationWindowImpl for ProfileWindow {}
}

glib::wrapper! {
    pub struct ProfileWindow(ObjectSubclass<imp::ProfileWindow>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible,
                    gtk::Buildable, gtk::ConstraintTarget, gtk::Native,
                    gtk::Root, gtk::ShortcutManager;
}

impl ProfileWindow {
    pub(crate) fn new(application: &gtk::Application) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    fn load_profile(&self) {
        let weak_window = self.downgrade();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(ProfileStore::load_current).await;
            let Some(window) = weak_window.upgrade() else {
                return;
            };

            match result {
                Ok(Ok(loaded)) => window.show_profile(loaded),
                Ok(Err(_)) => {
                    window.show_load_error(&gettext("The current user account could not be read."))
                }
                Err(_) => window.show_load_error(&gettext("Profile loading stopped unexpectedly.")),
            }
        });
    }

    fn show_profile(&self, loaded: LoadedProfile) {
        let imp = self.imp();
        let Profile {
            first_name,
            last_name,
            home_phone,
            email,
            office_phone,
            avatars,
        } = loaded.profile;

        imp.first_name_entry.set_text(&first_name);
        imp.last_name_entry.set_text(&last_name);
        imp.home_phone_entry.set_text(&home_phone);
        imp.email_entry.set_text(&email);
        imp.office_phone_entry.set_text(&office_phone);

        for avatar in avatars {
            let tooltip = format!(
                "{}\n{}",
                gettext("Profile image"),
                avatar.source_path().display()
            );
            let bytes = glib::Bytes::from_owned(avatar.bytes);
            if let Ok(texture) = gtk::gdk::Texture::from_bytes(&bytes) {
                imp.avatar_picture.set_paintable(Some(&texture));
                imp.avatar_picture.set_tooltip_text(Some(&tooltip));
                imp.avatar_picture.set_visible(true);
                break;
            }
        }

        imp.loading_indicator.set_spinning(false);
        imp.loading_indicator.set_visible(false);
        show_warnings(&imp.status_label, &loaded.warnings);
    }

    fn show_load_error(&self, message: &str) {
        let imp = self.imp();
        imp.loading_indicator.set_spinning(false);
        imp.loading_indicator.set_visible(false);
        imp.status_label.remove_css_class("dim-label");
        imp.status_label.add_css_class("error");
        imp.status_label.set_label(message);
        imp.status_label.set_tooltip_text(None);
    }
}

fn show_warnings(label: &gtk::Label, warnings: &[LoadWarning]) {
    if warnings.is_empty() {
        label.set_visible(false);
        return;
    }

    label.remove_css_class("dim-label");
    label.add_css_class("warning");
    label.set_label(&gettext("Some profile information could not be loaded."));
    label.set_tooltip_text(Some(
        &warnings
            .iter()
            .map(localized_warning)
            .collect::<Vec<_>>()
            .join("\n\n"),
    ));
}

fn localized_warning(warning: &LoadWarning) -> String {
    match warning {
        LoadWarning::AccountsServiceUnavailable(_) => {
            gettext("AccountsService profile information could not be read.")
        }
        LoadWarning::AvatarUnavailable { path, .. } => format!(
            "{}\n{}",
            gettext("A profile image could not be used."),
            path.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_window_constructs_from_its_resource_template() {
        if gtk::init().is_err() {
            eprintln!("skipping GTK window test because no display is available");
            return;
        }

        gio::resources_register_include!("xfce4_profile_editor.gresource")
            .expect("failed to register test resources");
        crate::i18n::init();
        crate::ui::load_styles();

        let application = gtk::Application::builder()
            .application_id("io.github.camelofcode.Xfce4ProfileEditor.Tests")
            .build();
        application
            .register(None::<&gio::Cancellable>)
            .expect("failed to register test application");
        let window = ProfileWindow::new(&application);

        assert!(window.title().is_some_and(|title| !title.is_empty()));
        window.present();
        assert!(window.is_visible());
        window.close();
    }
}
