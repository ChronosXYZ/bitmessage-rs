use relm4::{
    adw::{
        self,
        traits::{ActionRowExt, PreferencesRowExt},
    },
    gtk::{
        self,
        traits::{ButtonExt, ListBoxRowExt, WidgetExt},
    },
    prelude::{DynamicIndex, FactoryComponent},
    FactorySender,
};
use relm4_icons::icon_name;

use crate::ui::components::identities_list::IdentitiesListInput;

pub struct IdentityListRow {
    pub label: String,
    pub address: String,
}

pub struct IdentityListRowInit {
    pub label: String,
    pub address: String,
}

#[derive(Debug)]
pub enum IdentityListRowOutput {
    DeleteIdentity(DynamicIndex),
    RenameIdentity(DynamicIndex),
}

#[relm4::factory(pub)]
impl FactoryComponent for IdentityListRow {
    type Init = IdentityListRowInit;
    type Input = ();
    type Output = IdentityListRowOutput;
    type CommandOutput = ();
    type ParentInput = IdentitiesListInput;
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        adw::ActionRow {
            set_selectable: false,
            set_activatable: false,
            set_title: &self.label.to_string(),
            set_subtitle: &self.address.to_string(),

            add_suffix = &gtk::Button {
                set_icon_name: icon_name::EDIT,
                add_css_class: "circular",
                add_css_class: "flat",
                connect_clicked[sender, index] => move |_| {
                    sender.output(IdentityListRowOutput::RenameIdentity(index.clone()))
                },
            },
            add_suffix = &gtk::Button {
                set_icon_name: icon_name::X_CIRCULAR,
                add_css_class: "circular",
                add_css_class: "flat",
                connect_clicked[sender, index] => move |_| {
                    sender.output(IdentityListRowOutput::DeleteIdentity(index.clone()));
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
        Self {
            label: init.label,
            address: init.address,
        }
    }

    fn forward_to_parent(output: Self::Output) -> Option<Self::ParentInput> {
        Some(match output {
            IdentityListRowOutput::DeleteIdentity(i) => IdentitiesListInput::DeleteIdentity(i),
            IdentityListRowOutput::RenameIdentity(i) => {
                IdentitiesListInput::HandleRenameIdentity(i)
            }
        })
    }
}
